//! Read-only real-recording planner diagnostics. Cached estimates are not annotations.
use anyhow::{Context,Result,ensure};
use musicos_tauri::{analysis::Features,dj_plan::{self,Track}};
fn track(f:&Features)->Track<'_>{Track{key_index:f.key_index.filter(|_|f.key_confidence>=0.5),duration:f.structure.as_ref().map(|s|s.duration).unwrap_or(0.),bpm:f.bpm,structure:f.structure.as_ref()}}
fn main()->Result<()> {
 let path=std::env::args().nth(1).context("Pass library database")?;
 let db=rusqlite::Connection::open_with_flags(path,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
 let mut query=db.prepare("SELECT t.title,d.result FROM tracks t JOIN desktop_dsp_v1 d ON d.track_id=t.id WHERE t.deleted_at IS NULL ORDER BY t.id")?;
 let records=query.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
 let mut recordings=records.into_iter().map(|(name,json)|Ok((name,serde_json::from_str::<Features>(&json)?))).collect::<Result<Vec<_>>>()?;
 let refreshed=std::env::args().any(|a|a=="--refresh-selected");
 if refreshed {
  let titles=["The Moment","i","Moon Crystal","Dyscalculia","Aerodynamic","Atlantique Sud","Crescendolls","High Life","Midnight City","New Map","Raconte-Moi Une Histoire","Rapp Snitch Knishes [f. Mr. Fantastik]","Voyager","squabble up"];
  recordings.retain(|(title,_)|titles.contains(&title.as_str()));
  for (_,f) in &mut recordings {let path:String=db.query_row("SELECT path FROM tracks WHERE id=?",[f.track_id],|r|r.get(0))?;let structure=musicos_tauri::structure::scan(std::path::Path::new(&path),f.bpm,f.bpm_confidence,||true)?;if structure.beat_confidence>=0.5{f.bpm=structure.tempo;f.bpm_confidence=structure.beat_confidence;}f.structure=Some(structure);}
 }
 let config=dj_plan::Config{enabled:true,..Default::default()};let mut supported=0;let mut fallbacks=std::collections::BTreeMap::<String,usize>::new();let mut worst_drift=0_f32;let mut examples=Vec::new();let mut eligible=0;
 for (i,(name,a)) in recordings.iter().enumerate(){for (j,(other,b)) in recordings.iter().enumerate(){if i==j{continue;}
  let Some(plan)=dj_plan::plan(track(a),track(b),&config,false)else{continue};eligible+=1;
  if plan.confidence>=0.5 {supported+=1;let sa=a.structure.as_ref().unwrap();let sb=b.structure.as_ref().unwrap();
   let mut drift=0_f32;for beat in sa.beats.iter().filter(|t|**t>=plan.outgoing_cue&&**t<=plan.outgoing_cue+plan.duration){let target=plan.incoming_cue+(*beat-plan.outgoing_cue)*plan.incoming_tempo;let error=sb.beats.iter().map(|t|(*t-target).abs()/plan.incoming_tempo).fold(f32::MAX,f32::min);drift=drift.max(error);}
   ensure!(drift<=0.0501,"Accepted plan drifts by {drift}s");worst_drift=worst_drift.max(drift);
   if examples.len()<12{examples.push(serde_json::json!({"outgoing":name,"incoming":other,"plan":plan,"predicted_drift_ms":drift*1000.}));}
  }else{*fallbacks.entry(plan.reason.clone()).or_default()+=1;}
 }}
 println!("{}",serde_json::to_string_pretty(&serde_json::json!({"fresh_structural_analysis":refreshed,"recordings":recordings.len(),"eligible_pairs":eligible,"musical_plans":supported,"fallback_reasons":fallbacks,"maximum_predicted_drift_ms":worst_drift*1000.,"examples":examples,"independent_boundary_accuracy":null,"audible_acceptance":false}))?);Ok(())
}
