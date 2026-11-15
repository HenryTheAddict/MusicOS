use anyhow::Result;
use serde::{Deserialize,Serialize};
use std::{path::Path,sync::Mutex,time::{SystemTime,UNIX_EPOCH}};

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Job{
    pub id:String,
    pub kind:String,
    pub title:String,
    pub state:String,
    pub completed:usize,
    pub total:usize,
    pub message:String,
    pub finished_at:Option<u64>,
}

pub fn update(db:&Path,job:Job)->Result<()>{
    static WRITER:Mutex<()>=Mutex::new(());
    let _guard=WRITER.lock().unwrap_or_else(|error|error.into_inner());
    let conn=connection(db)?;
    let time=SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let mut job=job;
    if job.state!="running"{job.finished_at=Some(time);}
    let transaction=conn.unchecked_transaction()?;
    transaction.execute("INSERT INTO musicos_activity_v1(id,value) VALUES(?1,?2)
        ON CONFLICT(id) DO UPDATE SET value=excluded.value",
        rusqlite::params![job.id,serde_json::to_string(&job)?])?;
    transaction.execute("DELETE FROM musicos_activity_v1 WHERE rowid IN
        (SELECT rowid FROM musicos_activity_v1 WHERE json_extract(value,'$.state') <> 'running' ORDER BY rowid DESC LIMIT -1 OFFSET 32)",[])?;
    transaction.commit()?;
    Ok(())
}

fn connection(db:&Path)->Result<rusqlite::Connection>{
    let conn=rusqlite::Connection::open(db)?;
    conn.busy_timeout(std::time::Duration::from_secs(2))?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS musicos_activity_v1(id TEXT PRIMARY KEY,value TEXT NOT NULL)")?;
    Ok(conn)
}

pub fn list(db:&Path)->Result<Vec<Job>>{
    let conn=connection(db)?;
    let mut query=conn.prepare("SELECT value FROM musicos_activity_v1 ORDER BY rowid DESC LIMIT 40")?;
    let data=query.query_map([],|row|row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(data.iter().filter_map(|value|serde_json::from_str(value).ok()).collect())
}

pub fn recover(db:&Path){
    if let Ok(jobs)=list(db){
        for mut job in jobs.into_iter().filter(|job|job.state=="running"){
            job.state="interrupted".into();
            job.message="musicOS closed before this activity finished.".into();
            let _=update(db,job);
        }
    }
}

pub fn id(kind:&str)->String{format!("{kind}-{:016x}",rand::random::<u64>())}
