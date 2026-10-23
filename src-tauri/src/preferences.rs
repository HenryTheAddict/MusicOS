use musicos_desktop::storage::{atomic_write_json, desktop_data_dir};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
#[serde(default)]
pub struct LibraryFilters {pub sort:String,pub genre:String,pub quality:String,pub favorites:bool,pub availability:String,pub separate_singles:bool}
impl Default for LibraryFilters {fn default()->Self{Self{sort:"recent".into(),genre:String::new(),quality:"all".into(),favorites:false,availability:"all".into(),separate_singles:false}}}
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
#[serde(default)]
pub struct Appearance {pub mode:String,pub source:String,pub accent:String,pub saturation:u16,pub tint:u16}
impl Default for Appearance{fn default()->Self{Self{mode:"system".into(),source:"blue".into(),accent:"#0064df".into(),saturation:100,tint:15}}}
impl Appearance{fn normalize(mut self)->Self{if !["system","light","dark"].contains(&self.mode.as_str()){self.mode="system".into();}if !["artwork","blue","custom"].contains(&self.source.as_str()){self.source="blue".into();}if self.accent.len()!=7||!self.accent.starts_with('#')||!self.accent[1..].bytes().all(|c|c.is_ascii_hexdigit()){self.accent="#0064df".into();}self.saturation=self.saturation.min(150);self.tint=self.tint.min(40);self}}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Preferences {
    pub version: u32,
    pub mini_player_view: String,
    pub mini_player_density: String,
    pub now_playing_artwork_action: String,
    pub display_name: String,
    pub interface_percent: u16,
    pub coverflow_colors: bool,
    pub appearance:Appearance,
    pub artists_album_only:bool,
    pub audio_presentation: String,
    pub artwork_parallax:bool,
    pub artwork_depth:bool,
    pub show_song_durations: bool,
    pub show_now_playing_time: bool,
    pub sparklist_beta: bool,
    pub library_filters: LibraryFilters,
    pub listening:musicos_desktop::local::listening::Listening,
    pub dj_config: crate::dj_plan::Config,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: 1,
            mini_player_view: "artwork".into(),
            mini_player_density: "standard".into(),
            now_playing_artwork_action: "album".into(),
            display_name: String::new(),
            interface_percent: 100,
            coverflow_colors: false,
            appearance:Default::default(),
            artists_album_only:false,
            audio_presentation: "sidebar".into(),
            artwork_parallax:true,
            artwork_depth:false,
            show_song_durations: true,
            show_now_playing_time: true,
            sparklist_beta: false,
            library_filters: Default::default(),
            dj_config: Default::default(),
            listening:Default::default(),
        }
    }
}
impl Preferences {
    pub fn normalize(mut self) -> Self {
        self.version = 1;
        if self.mini_player_density != "compact" {self.mini_player_density="standard".into();}
        if self.mini_player_view != "queue" {self.mini_player_view="artwork".into();}
        if !["album","mini_player","artwork"].contains(&self.now_playing_artwork_action.as_str()){self.now_playing_artwork_action="album".into();}
        self.listening=self.listening.normalize();
        self.appearance=self.appearance.normalize();
        self.coverflow_colors=self.appearance.source=="artwork";
        if !["recent","title"].contains(&self.library_filters.sort.as_str()){self.library_filters.sort="recent".into();}
        if !["all","lossless","lossy","mixed"].contains(&self.library_filters.quality.as_str()){self.library_filters.quality="all".into();}
        if !["all","available","unavailable"].contains(&self.library_filters.availability.as_str()){self.library_filters.availability="all".into();}
        if !["sidebar", "floating"].contains(&self.audio_presentation.as_str()) {
            self.audio_presentation = "sidebar".into();
        }
        self.display_name = self.display_name.trim().to_owned();
        if self.display_name.chars().count() > 48
            || self.display_name.contains(['\n', '\r', '\t'])
        {
            self.display_name.clear();
        }
        if ![80, 100, 125, 150].contains(&self.interface_percent) {
            self.interface_percent = 100;
        }
        self
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.display_name.is_empty() || !self.display_name.trim().is_empty(),
            "Enter a name before saving."
        );
        anyhow::ensure!(
            self.display_name.trim().chars().count() <= 48
                && !self.display_name.contains(['\n', '\r', '\t']),
            "Use a name of up to 48 characters on one line."
        );
        anyhow::ensure!(
            [80, 100, 125, 150].contains(&self.interface_percent),
            "Choose 80%, 100%, 125%, or 150%."
        );
        Ok(())
    }
}
pub fn load_from(path: &Path, legacy: Option<&Path>) -> Preferences {
    if let Ok(bytes) = std::fs::read(path) {
        // Parse fields independently: one invalid field must not discard valid siblings.
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            let mut p = Preferences::default();
            if let Some(s)=v["mini_player_density"].as_str(){p.mini_player_density=s.into();}
            if let Some(s)=v["mini_player_view"].as_str(){p.mini_player_view=s.into();}
            if let Some(s)=v["now_playing_artwork_action"].as_str(){p.now_playing_artwork_action=s.into();}
            if let Some(s) = v["display_name"].as_str() {
                p.display_name = s.into()
            }
            if let Some(n) = v["interface_percent"]
                .as_u64()
                .and_then(|n| u16::try_from(n).ok())
            {
                p.interface_percent = n
            }
            if let Some(b) = v["coverflow_colors"].as_bool() {
                p.coverflow_colors = b
            }
            if let Some(value)=v["artwork_parallax"].as_bool(){p.artwork_parallax=value;}
            if let Some(value)=v["artwork_depth"].as_bool(){p.artwork_depth=value;}
            if let Some(value) = v["show_song_durations"].as_bool() { p.show_song_durations=value; }
            if let Some(value) = v["show_now_playing_time"].as_bool() { p.show_now_playing_time=value; }
            if let Some(value)=v.get("library_filters"){if let Ok(filters)=serde_json::from_value(value.clone()){p.library_filters=filters;}}
            if let Some(value) = v["sparklist_beta"].as_bool() { p.sparklist_beta=value; }
            if let Some(value) = v["audio_presentation"].as_str() {
                p.audio_presentation = value.into();
            }
            if let Some(value)=v.get("listening"){if let Ok(value)=serde_json::from_value(value.clone()){p.listening=value;}}
            if let Some(value) = v.get("dj_config") {
                if let Ok(config) = serde_json::from_value(value.clone()) {
                    p.dj_config = config;
                }
            }
            p.appearance=v.get("appearance").and_then(|value|serde_json::from_value(value.clone()).ok()).unwrap_or_else(||Appearance{mode:"light".into(),source:if p.coverflow_colors{"artwork"}else{"blue"}.into(),..Default::default()});
            return p.normalize();
        }
        return Preferences::default();
    }
    let mut p = Preferences::default();
    if let Some(text) = legacy.and_then(|p| std::fs::read_to_string(p).ok()) {
        for line in text.lines() {
            if let Some(v) = line
                .strip_prefix("name ")
                .and_then(|v| serde_json::from_str::<String>(v).ok())
            {
                p.display_name = v
            }
            if let Some(v) = line.strip_prefix("scale ").and_then(|v| v.parse().ok()) {
                p.interface_percent = v
            }
            if let Some(v) = line.strip_prefix("colors ") {
                p.coverflow_colors = v.trim() == "1"
            }
        }
    }
    if legacy.is_some_and(|p|p.exists()){p.appearance.mode="light".into();p.appearance.source=if p.coverflow_colors{"artwork"}else{"blue"}.into();}
    p.normalize()
}
pub fn path() -> std::path::PathBuf {
    desktop_data_dir().join("interface-v1.json")
}
pub fn load() -> Preferences {
    #[cfg(target_os = "macos")]
    let legacy = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|h| h.join("Library/Application Support/MusicOS/MusicOSDesktop/preferences.txt"));
    #[cfg(target_os = "windows")]
    let legacy = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .map(|p| p.join("MusicOS/MusicOSDesktop/preferences.txt"));
    #[cfg(all(unix, not(target_os = "macos")))]
    let legacy = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|h| h.join(".local/share"))
        })
        .map(|p| p.join("MusicOS/MusicOSDesktop/preferences.txt"));
    load_from(&path(), legacy.as_deref())
}
pub fn save(p: &Preferences) -> anyhow::Result<Preferences> {
    p.validate()?;
    let p = p.clone().normalize();
    atomic_write_json(&path(), &p)?;
    Ok(p)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn artwork_action_defaults_and_round_trips(){
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("prefs.json");
        std::fs::write(&path,r#"{"display_name":"Listener","mini_player_view":"queue"}"#).unwrap();
        let mut value=load_from(&path,None);assert_eq!(value.now_playing_artwork_action,"album");
        for action in ["album","mini_player","artwork"]{value.now_playing_artwork_action=action.into();atomic_write_json(&path,&value).unwrap();assert_eq!(load_from(&path,None).now_playing_artwork_action,action);}
        value.now_playing_artwork_action="unknown".into();atomic_write_json(&path,&value).unwrap();let loaded=load_from(&path,None);assert_eq!(loaded.now_playing_artwork_action,"album");assert_eq!(loaded.display_name,"Listener");assert_eq!(loaded.mini_player_view,"queue");
    }
    #[test] fn library_filters_persist_and_invalid_enums_fall_back(){
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("prefs.json");
        let mut p=Preferences::default();p.library_filters.sort="title".into();p.library_filters.separate_singles=true;p.library_filters.genre="Electronic".into();p.library_filters.favorites=true;
        atomic_write_json(&path,&p).unwrap();assert_eq!(load_from(&path,None),p);
        p.library_filters.quality="invalid".into();assert_eq!(p.normalize().library_filters.quality,"all");
    }
    #[test]
    fn round_trip_and_independent_fallback() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("prefs.json");
        let prefs = Preferences {
            display_name: "  Zoë  ".into(),
            interface_percent: 125,
            coverflow_colors: true,
            ..Default::default()
        };
        atomic_write_json(&p, &prefs).unwrap();
        assert_eq!(load_from(&p, None).display_name, "Zoë");
        assert_eq!(load_from(&p, None).interface_percent, 125);
        std::fs::write(
            &p,
            r#"{"display_name":"Ada","interface_percent":42,"coverflow_colors":true}"#,
        )
        .unwrap();
        let loaded = load_from(&p, None);
        assert_eq!(loaded.display_name, "Ada");
        assert_eq!(loaded.interface_percent, 100);
        assert!(loaded.coverflow_colors);
    }
    #[test]
    fn time_and_beta_preferences_default_safely_and_persist() {
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("prefs.json");
        let defaults=load_from(&path,None);
        assert!(defaults.artwork_parallax&&defaults.show_song_durations&&defaults.show_now_playing_time&&!defaults.sparklist_beta);
        let edited=Preferences{artwork_parallax:false,show_song_durations:false,show_now_playing_time:false,sparklist_beta:true,..defaults};
        atomic_write_json(&path,&edited).unwrap();let loaded=load_from(&path,None);
        assert!(!loaded.artwork_parallax&&!loaded.show_song_durations&&!loaded.show_now_playing_time&&loaded.sparklist_beta);
        std::fs::write(&path,r#"{"show_song_durations":"bad","sparklist_beta":"true"}"#).unwrap();
        let loaded=load_from(&path,None);assert!(loaded.show_song_durations&&!loaded.sparklist_beta);
    }
    #[test]fn theme_migration_preserves_light_artwork_and_new_defaults_follow_system(){let tmp=tempfile::tempdir().unwrap();let path=tmp.path().join("prefs.json");assert_eq!(load_from(&path,None).appearance.mode,"system");std::fs::write(&path,r#"{"coverflow_colors":true}"#).unwrap();let migrated=load_from(&path,None);assert_eq!(migrated.appearance.mode,"light");assert_eq!(migrated.appearance.source,"artwork");let custom=Preferences{appearance:Appearance{mode:"dark".into(),source:"custom".into(),accent:"#ff6688".into(),saturation:125,tint:8},..migrated};atomic_write_json(&path,&custom).unwrap();assert_eq!(load_from(&path,None).appearance,custom.appearance);}
    #[test]
    fn migration_and_blank_validation() {
        let t = tempfile::tempdir().unwrap();
        let old = t.path().join("old");
        std::fs::write(&old, "version 1\nname \"Henry Two\"\nscale 150\ncolors 1\n").unwrap();
        let loaded = load_from(&t.path().join("new"), Some(&old));
        assert_eq!(loaded.display_name, "Henry Two");
        assert_eq!(loaded.interface_percent, 150);
        assert!(loaded.coverflow_colors);
        assert!(Preferences {
            display_name: " ".into(),
            ..Default::default()
        }
        .validate()
        .is_err());
    }
    #[test]fn mini_view_defaults_migrates_and_survives_restart(){
      let dir=tempfile::tempdir().unwrap();let path=dir.path().join("prefs.json");
      assert_eq!(load_from(&path,None).mini_player_view,"artwork");
      std::fs::write(&path,r#"{"mini_player_view":"slim","display_name":"Listener"}"#).unwrap();
      let mut value=load_from(&path,None);assert_eq!(value.mini_player_view,"artwork");assert_eq!(value.display_name,"Listener");
      value.mini_player_view="queue".into();atomic_write_json(&path,&value).unwrap();assert_eq!(load_from(&path,None).mini_player_view,"queue");
    }

}

#[cfg(test)]
mod density_tests {
 use super::*;
 #[test]fn compact_density_round_trips_without_inventing_a_name(){let root=tempfile::tempdir().unwrap();let path=root.path().join("prefs.json");let initial=load_from(&path,None);assert!(initial.display_name.is_empty());assert_eq!(initial.mini_player_density,"standard");std::fs::write(&path,r#"{"mini_player_density":"compact","display_name":"Zoë","mini_player_view":"queue"}"#).unwrap();let value=load_from(&path,None);assert_eq!(value.mini_player_density,"compact");assert_eq!(value.display_name,"Zoë");assert_eq!(value.mini_player_view,"queue");atomic_write_json(&path,&value).unwrap();assert_eq!(load_from(&path,None),value);std::fs::write(&path,r#"{"mini_player_density":"slim"}"#).unwrap();assert_eq!(load_from(&path,None).mini_player_density,"standard");}
}
