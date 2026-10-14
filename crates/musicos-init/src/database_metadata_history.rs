// These methods share Db's connection so edits and their undo records commit together.
impl Db {
    fn ensure_metadata_history(&self)->Result<()> {
        self.conn.execute_batch("CREATE TABLE IF NOT EXISTS desktop_metadata_changes_v1(id INTEGER PRIMARY KEY,created_at INTEGER NOT NULL,undone INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS desktop_metadata_change_entries_v1(change_id INTEGER NOT NULL,track_id INTEGER NOT NULL,before_json TEXT NOT NULL,after_json TEXT NOT NULL,PRIMARY KEY(change_id,track_id));")?;
        self.ensure_column("desktop_metadata_changes_v1","canonical_json","TEXT")?;
        Ok(())
    }
    pub fn metadata_values(&self,id:i64)->Result<serde_json::Value>{
        let track=self.get_track_by_id(id)?.ok_or_else(||anyhow::anyhow!("A selected song no longer exists."))?;
        let mut value=serde_json::to_value(track)?;
        let extra:(String,i32,i32,i64)=self.conn.query_row("SELECT coalesce(o.genre,t.genre,''),coalesce(o.year,t.year,0),coalesce(o.disc_number,t.disc_number,0),t.revision FROM tracks t LEFT JOIN track_metadata_overrides o ON o.track_id=t.id WHERE t.id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
        value["genre"]=extra.0.into();value["year"]=extra.1.into();value["disc_number"]=extra.2.into();value["revision"]=extra.3.into();Ok(value)
    }
    fn metadata_history_snapshot(&self,id:i64)->Result<String> {
        let track=self.metadata_values(id)?;
        let overrides:Option<String>=self.conn.query_row("SELECT json_object('title',title,'artist',artist,'album',album,'track_number',track_number,'disc_number',disc_number,'genre',genre,'year',year,'artwork_path',artwork_path,'updated_at',updated_at,'origin',origin,'provider',provider,'confidence',confidence) FROM track_metadata_overrides WHERE track_id=?",[id],|r|r.get(0)).optional()?;
        Ok(serde_json::to_string(&serde_json::json!({"track":track,"override":overrides.map(|s|serde_json::from_str::<serde_json::Value>(&s)).transpose()?}))?)
    }
    fn canonical_history_snapshot(&self)->Result<String>{
        if self.conn.query_row("SELECT count(*) FROM sqlite_master WHERE name='desktop_album_artwork_v1'",[],|r|r.get::<_,i64>(0))?==0{return Ok("{}".into());}
        let mut stmt=self.conn.prepare("SELECT album_id,path FROM desktop_album_artwork_v1")?;
        let values=stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<std::collections::BTreeMap<_,_>>>()?;
        Ok(serde_json::to_string(&values)?)
    }
    pub fn metadata_history(&self)->Result<serde_json::Value> {
        self.ensure_metadata_history()?;
        let mut stmt=self.conn.prepare("SELECT id,created_at,undone FROM desktop_metadata_changes_v1 ORDER BY id DESC LIMIT 50")?;
        let batches=stmt.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,bool>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut result=Vec::new();
        for (id,created_at,undone) in batches {
            let mut entries=self.conn.prepare("SELECT track_id,before_json,after_json FROM desktop_metadata_change_entries_v1 WHERE change_id=? ORDER BY track_id")?;
            let rows=entries.query_map([id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let mut conflicts=false;let mut changes=Vec::new();
            for (track_id,before,after) in rows {
                let current=self.metadata_history_snapshot(track_id).ok();
                let a:serde_json::Value=serde_json::from_str(&after)?;
                let c=current.as_deref().map(serde_json::from_str::<serde_json::Value>).transpose()?;
                let conflict=c.as_ref().is_none_or(|c|c["override"]!=a["override"] || c["track"]["modified_at"]!=a["track"]["modified_at"] || c["track"]["file_size"]!=a["track"]["file_size"]);
                conflicts|=conflict;
                changes.push(serde_json::json!({"track_id":track_id,"before":serde_json::from_str::<serde_json::Value>(&before)?["track"],"after":a["track"],"current":c.map(|c|c["track"].clone()),"conflict":conflict}));
            }
            result.push(serde_json::json!({"id":id,"created_at":created_at,"undone":undone,"conflict":conflicts,"changes":changes}));
        }
        Ok(serde_json::json!(result))
    }
    pub fn undo_metadata_change(&self,id:i64)->Result<()> {
        self.ensure_metadata_history()?;self.begin_transaction()?;
        let result=(||{
            let history=self.metadata_history()?;
            let batch=history.as_array().unwrap().iter().find(|v|v["id"]==id).ok_or_else(||anyhow::anyhow!("This change is no longer available."))?;
            anyhow::ensure!(batch["undone"]==false,"This change was already undone.");
            anyhow::ensure!(batch["conflict"]==false,"Later edits conflict with this change. Compare the current values before undoing.");
            let mut stmt=self.conn.prepare("SELECT track_id,before_json FROM desktop_metadata_change_entries_v1 WHERE change_id=?")?;
            let rows=stmt.query_map([id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let canonical:Option<String>=self.conn.query_row("SELECT canonical_json FROM desktop_metadata_changes_v1 WHERE id=?",[id],|r|r.get(0))?;
            let previous:std::collections::BTreeMap<String,String>=serde_json::from_str(canonical.as_deref().unwrap_or("{}"))?;
            let mut replaced_art=std::collections::BTreeSet::new();
            for change in batch["changes"].as_array().unwrap(){if change["after"]["cover_art_path"]!=change["before"]["cover_art_path"] {if let Some(path)=change["after"]["cover_art_path"].as_str(){replaced_art.insert(path.to_owned());}}}
            for (track_id,before) in rows {
                let before:serde_json::Value=serde_json::from_str(&before)?;let old=&before["override"];
                self.conn.execute("DELETE FROM track_metadata_overrides WHERE track_id=?",[track_id])?;
                if !old.is_null(){self.conn.execute("INSERT INTO track_metadata_overrides(track_id,title,artist,album,track_number,disc_number,genre,year,artwork_path,updated_at,origin,provider,confidence) SELECT ?,json_extract(?2,'$.title'),json_extract(?2,'$.artist'),json_extract(?2,'$.album'),json_extract(?2,'$.track_number'),json_extract(?2,'$.disc_number'),json_extract(?2,'$.genre'),json_extract(?2,'$.year'),json_extract(?2,'$.artwork_path'),json_extract(?2,'$.updated_at'),json_extract(?2,'$.origin'),json_extract(?2,'$.provider'),json_extract(?2,'$.confidence')",params![track_id,serde_json::to_string(old)?])?;}
                self.conn.execute("UPDATE tracks SET revision=revision+1 WHERE id=?",[track_id])?;
                self.conn.execute("UPDATE metadata_enrichment_status SET state='kept' WHERE track_id=?",[track_id])?;
            }
            for path in replaced_art {
                if self.conn.query_row("SELECT count(*) FROM sqlite_master WHERE name='desktop_album_artwork_v1'",[],|r|r.get::<_,i64>(0))?==0{break;}
                let mut stmt=self.conn.prepare("SELECT album_id FROM desktop_album_artwork_v1 WHERE path=?")?;
                let albums=stmt.query_map([path],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
                for album in albums {if let Some(old)=previous.get(&album){self.conn.execute("UPDATE desktop_album_artwork_v1 SET path=? WHERE album_id=?",params![old,album])?;}else{self.conn.execute("DELETE FROM desktop_album_artwork_v1 WHERE album_id=?",[album])?;}}
            }
            self.conn.execute("UPDATE desktop_metadata_changes_v1 SET undone=1 WHERE id=?",[id])?;Ok(())
        })();
        match result {Ok(())=>self.commit_transaction(),Err(e)=>{let _=self.rollback_transaction();Err(e)}}
    }
}
