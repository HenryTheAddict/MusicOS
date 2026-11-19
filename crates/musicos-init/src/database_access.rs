//! One short-lived writer per database; readers never acquire the writer permit.
use rusqlite::{Params, Result, TransactionBehavior};
use std::{cell::RefCell,collections::HashMap,ops::{Deref,DerefMut},path::{Path,PathBuf},sync::{Arc,Condvar,Mutex,OnceLock,Weak},time::{Duration,Instant}};
#[derive(Debug,Default)]struct State{owner:Option<std::thread::ThreadId>,depth:usize,interactive:usize}
#[derive(Debug,Default)]struct Gate{state:Mutex<State>,wake:Condvar}
#[derive(Debug)]pub struct Permit{gate:Arc<Gate>}
impl Drop for Permit{fn drop(&mut self){let mut state=self.gate.state.lock().unwrap_or_else(|e|e.into_inner());state.depth-=1;if state.depth==0{state.owner=None;self.gate.wake.notify_all();}}}
impl Gate{fn acquire(self:&Arc<Self>,background:bool)->Result<Permit>{let thread=std::thread::current().id();let start=Instant::now();let mut state=self.state.lock().unwrap_or_else(|e|e.into_inner());if state.owner==Some(thread){state.depth+=1;return Ok(Permit{gate:self.clone()});}if !background{state.interactive+=1;}
 while state.owner.is_some()||(background&&state.interactive>0){let remaining=Duration::from_secs(5).saturating_sub(start.elapsed());if remaining.is_zero(){if !background{state.interactive-=1;}return Err(rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),Some("Library is busy. Background changes are queued; try this action again.".into())));}state=self.wake.wait_timeout(state,remaining).unwrap_or_else(|e|e.into_inner()).0;}
 if !background{state.interactive-=1;}state.owner=Some(thread);state.depth=1;Ok(Permit{gate:self.clone()})}}
fn gate(path:&Path)->Arc<Gate>{static GATES:OnceLock<Mutex<HashMap<PathBuf,Weak<Gate>>>>=OnceLock::new();let key=path.canonicalize().unwrap_or_else(|_|path.parent().and_then(|p|p.canonicalize().ok()).unwrap_or_default().join(path.file_name().unwrap_or_default()));let mut gates=GATES.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner());if let Some(g)=gates.get(&key).and_then(Weak::upgrade){return g;}gates.retain(|_,g|g.strong_count()>0);let g=Arc::new(Gate::default());gates.insert(key,Arc::downgrade(&g));g}
#[derive(Debug)]pub struct Connection{raw:rusqlite::Connection,gate:Arc<Gate>,background:bool,explicit:RefCell<Option<Permit>>}
impl Deref for Connection{type Target=rusqlite::Connection;fn deref(&self)->&Self::Target{&self.raw}}
impl DerefMut for Connection{fn deref_mut(&mut self)->&mut Self::Target{&mut self.raw}}
impl Connection{
 pub fn open(path:impl AsRef<Path>)->Result<Self>{Self::open_priority(path,false)}
 pub fn open_background(path:impl AsRef<Path>)->Result<Self>{Self::open_priority(path,true)}
 fn open_priority(path:impl AsRef<Path>,background:bool)->Result<Self>{let gate=gate(path.as_ref());let raw=rusqlite::Connection::open(path)?;raw.busy_timeout(Duration::from_secs(5))?;raw.pragma_update(None,"foreign_keys","ON")?;raw.pragma_update(None,"synchronous","NORMAL")?;raw.pragma_update(None,"temp_store","MEMORY")?;Ok(Self{raw,gate,background,explicit:Default::default()})}
 pub fn open_in_memory()->Result<Self>{let raw=rusqlite::Connection::open_in_memory()?;Ok(Self{raw,gate:Arc::new(Gate::default()),background:false,explicit:Default::default()})}
 pub fn reserve_writer(&self)->Result<Permit>{self.gate.acquire(self.background)}
 pub fn execute(&self,sql:&str,params:impl Params)->Result<usize>{let command=sql.trim().to_ascii_uppercase();if command.starts_with("BEGIN"){let permit=self.gate.acquire(self.background)?;let result=self.raw.execute("BEGIN IMMEDIATE",params);if result.is_ok(){*self.explicit.borrow_mut()=Some(permit);}return result;}
 if self.raw.is_autocommit(){let _permit=self.gate.acquire(self.background)?;self.raw.execute(sql,params)}else{let result=self.raw.execute(sql,params);if self.raw.is_autocommit(){self.explicit.borrow_mut().take();}result}}
 pub fn execute_batch(&self,sql:&str)->Result<()>{if self.raw.is_autocommit(){let _permit=self.gate.acquire(self.background)?;self.raw.execute_batch(sql)}else{self.raw.execute_batch(sql)}}
 pub fn unchecked_transaction(&self)->Result<Transaction<'_>>{let permit=self.gate.acquire(self.background)?;let transaction=rusqlite::Transaction::new_unchecked(&self.raw,TransactionBehavior::Immediate)?;Ok(Transaction{transaction, _permit:permit})}
 pub fn transaction(&mut self)->Result<Transaction<'_>>{self.unchecked_transaction()}
}
pub struct Transaction<'a>{transaction:rusqlite::Transaction<'a>,_permit:Permit}
impl<'a>Deref for Transaction<'a>{type Target=rusqlite::Transaction<'a>;fn deref(&self)->&Self::Target{&self.transaction}}
impl Transaction<'_>{pub fn commit(self)->Result<()>{self.transaction.commit()}pub fn rollback(self)->Result<()>{self.transaction.rollback()}}
#[cfg(test)]mod tests{use super::*;
 #[test]fn concurrent_writes_wait_without_lock_upgrade_failures(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("db");let first=Connection::open(&path).unwrap();first.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE values_test(n INTEGER)").unwrap();let tx=first.unchecked_transaction().unwrap();tx.execute("INSERT INTO values_test VALUES(1)",[]).unwrap();let (send,receive)=std::sync::mpsc::channel();let thread=std::thread::spawn(move||{let second=Connection::open_background(path).unwrap();let tx=second.unchecked_transaction().unwrap();tx.execute("INSERT INTO values_test VALUES(2)",[]).unwrap();tx.commit().unwrap();send.send(()).unwrap();});assert!(receive.recv_timeout(Duration::from_millis(30)).is_err());tx.commit().unwrap();receive.recv_timeout(Duration::from_secs(2)).unwrap();thread.join().unwrap();assert_eq!(first.query_row("SELECT count(*) FROM values_test",[],|r|r.get::<_,i64>(0)).unwrap(),2);}
 #[test]fn rollback_releases_explicit_permit(){let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE values_test(n INTEGER)").unwrap();db.execute("BEGIN TRANSACTION",[]).unwrap();db.execute("INSERT INTO values_test VALUES(1)",[]).unwrap();db.execute("ROLLBACK",[]).unwrap();let tx=db.unchecked_transaction().unwrap();tx.execute("INSERT INTO values_test VALUES(2)",[]).unwrap();tx.commit().unwrap();assert_eq!(db.query_row("SELECT n FROM values_test",[],|r|r.get::<_,i64>(0)).unwrap(),2);}
}

/// Cache schema initialization by file identity, including replacement databases.
pub fn initialize_schema(path:&Path,label:&'static str,initialize:impl FnOnce()->anyhow::Result<()>)->anyhow::Result<()>{
 static SCHEMAS:OnceLock<Mutex<HashMap<String,Arc<Mutex<std::collections::HashSet<&'static str>>>>>>=OnceLock::new();
 if path == Path::new(":memory:") { return initialize(); }
 let metadata=std::fs::metadata(path)?;
 #[cfg(unix)]let identity={use std::os::unix::fs::MetadataExt;format!("{}:{}:{}",path.canonicalize()?.display(),metadata.dev(),metadata.ino())};
 #[cfg(not(unix))]let identity=format!("{}:{:?}",path.canonicalize()?.display(),metadata.created().ok());
 let schema={let mut schemas=SCHEMAS.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner());if schemas.len()>256{schemas.retain(|_,v|Arc::strong_count(v)>1);}schemas.entry(identity).or_default().clone()};
 let mut initialized=schema.lock().unwrap_or_else(|e|e.into_inner());if !initialized.contains(label){initialize()?;initialized.insert(label);}Ok(())
}
