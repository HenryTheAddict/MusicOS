use serde::{Deserialize,Serialize};
#[derive(Clone,Serialize,Deserialize)]
pub struct Palette {pub background:String,pub blob:String,pub accent:String,pub text:String}
#[derive(Clone,Serialize,Deserialize)]
pub struct Snapshot {pub appearance:crate::preferences::Appearance,pub dark:bool,pub palette:Palette}
fn color(s:&str)->bool{s.len()==7&&s.starts_with('#')&&s[1..].bytes().all(|c|c.is_ascii_hexdigit())}
impl Snapshot {
 pub fn valid(&self)->bool{[&self.palette.background,&self.palette.blob,&self.palette.accent,&self.palette.text].iter().all(|s|color(s))}
 pub fn matches(&self,appearance:&crate::preferences::Appearance,dark:bool)->bool{self.valid()&&&self.appearance==appearance&&self.dark==dark}
 pub fn native_color(&self)->tauri::window::Color{let s=&self.palette.blob;tauri::window::Color(u8::from_str_radix(&s[1..3],16).unwrap(),u8::from_str_radix(&s[3..5],16).unwrap(),u8::from_str_radix(&s[5..7],16).unwrap(),255)}
}
pub fn load()->Option<Snapshot>{serde_json::from_slice(&std::fs::read(crate::preferences::path().with_file_name("launch-palette-v1.json")).ok()?).ok()}
pub fn save(snapshot:&Snapshot)->anyhow::Result<()>{anyhow::ensure!(snapshot.valid(),"Invalid launch colors");musicos_desktop::storage::atomic_write_json(&crate::preferences::path().with_file_name("launch-palette-v1.json"),snapshot)}
#[cfg(test)]mod tests{use super::*;#[test]fn stale_or_invalid_palettes_are_rejected(){let mut s=Snapshot{appearance:Default::default(),dark:true,palette:Palette{background:"#112233".into(),blob:"#223344".into(),accent:"#aabbcc".into(),text:"#ffffff".into()}};assert!(s.matches(&s.appearance,true));assert!(!s.matches(&s.appearance,false));assert_eq!(s.native_color().0,34);s.palette.blob="red".into();assert!(!s.valid());}}
