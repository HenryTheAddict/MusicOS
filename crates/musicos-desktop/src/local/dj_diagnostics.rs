//! Bounded service-thread diagnostics. Never called from the output callback.
use serde::Serialize;
use std::{collections::VecDeque,sync::{Mutex,OnceLock},time::{SystemTime,UNIX_EPOCH}};
#[derive(Clone,Debug,Serialize)]
pub struct Record {pub time_ms:u128,pub event:String,pub outgoing:Option<u64>,pub incoming:Option<u64>,pub detail:String}
static RECORDS:OnceLock<Mutex<VecDeque<Record>>>=OnceLock::new();
pub fn record(event:&str,outgoing:Option<u64>,incoming:Option<u64>,detail:impl Into<String>){
 let mut records=RECORDS.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner());if records.len()==256{records.pop_front();}
 records.push_back(Record{time_ms:SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis(),event:event.into(),outgoing,incoming,detail:detail.into()});
}
pub fn snapshot()->Vec<Record>{RECORDS.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner()).iter().cloned().collect()}
#[cfg(test)]mod tests{use super::*;#[test]fn records_are_bounded(){for _ in 0..300{record("test",None,None,"");}assert_eq!(snapshot().len(),256);}}
