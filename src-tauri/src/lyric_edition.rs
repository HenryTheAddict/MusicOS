//! Desktop-owned edition policy. Ratings are evidence, never musicOS star ratings.
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preference {
    #[default]
    Explicit,
    Clean,
    Ask,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Override {
    #[default]
    Automatic,
    Explicit,
    Clean,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rating {
    Explicit,
    Clean,
    #[default]
    Unknown,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub track_id: Option<i64>,
    pub preference: Option<Preference>,
    pub edition: Option<Override>,
}
fn tables(c: &Connection) -> Result<()> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS edition_settings(id INTEGER PRIMARY KEY CHECK(id=1),preference TEXT NOT NULL);CREATE TABLE IF NOT EXISTS edition_overrides(track_id INTEGER PRIMARY KEY,edition TEXT NOT NULL);CREATE TABLE IF NOT EXISTS rating_evidence(track_id INTEGER PRIMARY KEY,file_identity TEXT,rating TEXT);CREATE TABLE IF NOT EXISTS match_provenance(track_id INTEGER PRIMARY KEY,catalog_id TEXT,policy TEXT,manual INTEGER NOT NULL DEFAULT 0);")?;
    Ok(())
}
fn text<T: Serialize>(v: T) -> String {
    serde_json::to_value(v)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}
pub fn preference(c: &Connection) -> Result<Preference> {
    tables(c)?;
    Ok(c.query_row(
        "SELECT preference FROM edition_settings WHERE id=1",
        [],
        |r| r.get::<_, String>(0),
    )
    .optional()?
    .and_then(|s| serde_json::from_value(json!(s)).ok())
    .unwrap_or_default())
}
fn override_for(c: &Connection, id: i64) -> Result<Override> {
    Ok(c.query_row(
        "SELECT edition FROM edition_overrides WHERE track_id=?",
        [id],
        |r| r.get::<_, String>(0),
    )
    .optional()?
    .and_then(|s| serde_json::from_value(json!(s)).ok())
    .unwrap_or_default())
}
fn parse_rating(value: &str) -> Rating {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "4" | "explicit" => Rating::Explicit,
        "2" | "clean" => Rating::Clean,
        _ => Rating::Unknown,
    }
}
fn rating(c: &Connection, t: &musicos_init::database::Track) -> Result<Rating> {
    use lofty::{file::TaggedFileExt, tag::ItemKey};
    let identity = std::fs::metadata(&t.path)
        .ok()
        .map(|m| {
            format!(
                "{}:{}:{}",
                t.path,
                m.len(),
                m.modified()
                    .ok()
                    .and_then(|v| v.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|v| v.as_nanos())
                    .unwrap_or(0)
            )
        })
        .unwrap_or_else(|| format!("{}:unavailable", t.path));
    if let Some(s) = c
        .query_row(
            "SELECT rating FROM rating_evidence WHERE track_id=? AND file_identity=?",
            rusqlite::params![t.id, identity],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        return Ok(serde_json::from_value(json!(s)).unwrap_or_default());
    }
    // Read tags only when the original changes. Missing/zero/unrecognized flags are Unknown.
    let value = lofty::probe::Probe::open(&t.path)
        .and_then(|probe| {
            probe
                .options(
                    lofty::config::ParseOptions::new()
                        .read_cover_art(false)
                        .read_properties(false),
                )
                .read()
        })
        .ok()
        .and_then(|f| {
            f.tags().iter().find_map(|tag| {
                tag.get_string(&ItemKey::ParentalAdvisory)
                    .map(parse_rating)
                    .or_else(|| {
                        tag.items().find_map(|item| {
                            if let ItemKey::Unknown(key) = item.key() {
                                if [
                                    "ITUNESADVISORY",
                                    "CONTENTRATING",
                                    "CONTENT_RATING",
                                    "PARENTAL_ADVISORY",
                                ]
                                .contains(&key.to_ascii_uppercase().as_str())
                                {
                                    return item.value().text().map(parse_rating);
                                }
                            }
                            None
                        })
                    })
            })
        })
        .unwrap_or_default();
    c.execute(
        "INSERT OR REPLACE INTO rating_evidence VALUES(?,?,?)",
        rusqlite::params![t.id, identity, text(value)],
    )?;
    Ok(value)
}
#[derive(Debug, Clone, PartialEq)]
pub struct Policy {
    pub key: String,
    pub wanted: Rating,
}
pub fn policy(c: &Connection, t: &musicos_init::database::Track) -> Result<Policy> {
    let pref = preference(c)?;
    let over = override_for(c, t.id)?;
    let evidence = rating(c, t)?;
    let wanted = match over {
        Override::Explicit => Rating::Explicit,
        Override::Clean => Rating::Clean,
        Override::Automatic => match evidence {
            Rating::Unknown => match pref {
                Preference::Explicit => Rating::Explicit,
                Preference::Clean => Rating::Clean,
                Preference::Ask => Rating::Unknown,
            },
            known => known,
        },
    };
    Ok(Policy {
        key: format!("v1:{pref:?}:{over:?}:{evidence:?}"),
        wanted,
    })
}
pub fn valid(c: &Connection, id: i64, catalog: &str, p: &Policy) -> Result<bool> {
    tables(c)?;
    Ok(c.query_row(
        "SELECT manual=1 OR policy=? FROM match_provenance WHERE track_id=? AND catalog_id=?",
        rusqlite::params![p.key, id, catalog],
        |r| r.get(0),
    )
    .optional()?
    .unwrap_or(false))
}
pub fn record(c: &Connection, id: i64, catalog: &str, p: &Policy, manual: bool) -> Result<()> {
    c.execute(
        "INSERT OR REPLACE INTO match_provenance VALUES(?,?,?,?)",
        rusqlite::params![id, catalog, p.key, manual],
    )?;
    Ok(())
}
// Call only after recording/edition identity and duration have been filtered.
pub fn choose<'a>(
    rows: &'a std::collections::BTreeMap<&str, &Value>,
    p: &Policy,
) -> Option<(&'a str, &'a Value)> {
    let rated = rows
        .iter()
        .filter(|(_, v)| {
            match v
                .pointer("/attributes/contentRating")
                .and_then(Value::as_str)
            {
                Some("explicit") => p.wanted == Rating::Explicit,
                Some("clean") => p.wanted == Rating::Clean,
                _ => false,
            }
        })
        .map(|(id, v)| (*id, *v))
        .collect::<Vec<_>>();
    if p.wanted != Rating::Unknown {
        if rated.len() == 1 {
            return Some(rated[0]);
        }
        if !rated.is_empty() {
            return None;
        }
        let unknown = rows
            .iter()
            .filter(|(_, v)| {
                v.pointer("/attributes/contentRating")
                    .and_then(Value::as_str)
                    .is_none()
            })
            .collect::<Vec<_>>();
        return (unknown.len() == 1).then(|| (*unknown[0].0, *unknown[0].1));
    }
    // Ask never chooses between editions, even when they have the same title.
    if rows.len() == 1 {
        let (id, v) = rows.first_key_value()?;
        Some((*id, *v))
    } else {
        None
    }
}
pub fn action(db: &Path, req: Request) -> Result<Value> {
    let c = super::apple_enrichment::store(db)?;
    tables(&c)?;
    if let Some(pref) = req.preference {
        c.execute(
            "INSERT OR REPLACE INTO edition_settings VALUES(1,?)",
            [text(pref)],
        )?;
    }
    if let Some(id) = req.track_id {
        let track = musicos_init::database::Db::open(&db.to_string_lossy())?
            .get_track_by_id(id)?
            .context("Song unavailable")?;
        if let Some(over) = req.edition {
            c.execute(
                "INSERT OR REPLACE INTO edition_overrides VALUES(?,?)",
                rusqlite::params![id, text(over)],
            )?;
        }
        let p = policy(&c, &track)?;
        Ok(
            json!({"preference":preference(&c)?,"edition":override_for(&c,id)?,"evidence":rating(&c,&track)?,"resolved":p.wanted}),
        )
    } else {
        Ok(json!({"preference":preference(&c)?}))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    #[test]
    fn rating_flags_are_not_star_ratings() {
        assert_eq!(parse_rating("0"), Rating::Unknown);
        assert_eq!(parse_rating("2"), Rating::Clean);
        assert_eq!(parse_rating("4"), Rating::Explicit);
        assert_eq!(parse_rating("unrated"), Rating::Unknown);
    }
    #[test]
    fn counterparts_rank_without_guessing_ambiguous_recordings() {
        let a = json!({"attributes":{"contentRating":"explicit"}});
        let b = json!({"attributes":{"contentRating":"clean"}});
        let mut rows = BTreeMap::from([("1", &a), ("2", &b)]);
        let p = Policy {
            key: "test".into(),
            wanted: Rating::Explicit,
        };
        assert_eq!(choose(&rows, &p).unwrap().0, "1");
        rows.insert("3", &a);
        assert!(choose(&rows, &p).is_none());
        assert!(choose(&BTreeMap::from([("2", &b)]), &p).is_none());
        assert!(choose(
            &rows,
            &Policy {
                key: "ask".into(),
                wanted: Rating::Unknown
            }
        )
        .is_none());
    }
    #[test]
    fn manual_links_survive_policy_changes() {
        let c = Connection::open_in_memory().unwrap();
        tables(&c).unwrap();
        let p = Policy {
            key: "old".into(),
            wanted: Rating::Clean,
        };
        record(&c, 1, "100", &p, true).unwrap();
        record(&c, 2, "200", &p, false).unwrap();
        let new = Policy {
            key: "new".into(),
            wanted: Rating::Explicit,
        };
        assert!(valid(&c, 1, "100", &new).unwrap());
        assert!(!valid(&c, 2, "200", &new).unwrap());
        assert!(!valid(&c, 1, "other", &new).unwrap());
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    #[test]
    fn known_file_rating_wins_default_and_override_does_not_touch_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("clean.flac");
        let result = std::process::Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=400:duration=0.1",
                "-c:a",
                "flac",
                "-metadata",
                "ITUNESADVISORY=2",
            ])
            .arg(&audio)
            .status()
            .unwrap();
        assert!(result.success());
        let bytes = std::fs::read(&audio).unwrap();
        let db = dir.path().join("library.db");
        let d = musicos_init::database::Db::open(&db.to_string_lossy()).unwrap();
        let id = d
            .insert_track(
                audio.to_str().unwrap(),
                "Song",
                "Artist",
                "Album",
                1,
                60,
                100,
            )
            .unwrap();
        let result = action(
            &db,
            Request {
                track_id: Some(id),
                preference: None,
                edition: None,
            },
        )
        .unwrap();
        assert_eq!(result["preference"], "explicit");
        assert_eq!(result["resolved"], "clean");
        let r = action(
            &db,
            Request {
                track_id: Some(id),
                preference: None,
                edition: Some(Override::Explicit),
            },
        )
        .unwrap();
        assert_eq!(r["resolved"], "explicit");
        assert_eq!(r["evidence"], "clean");
        assert_eq!(std::fs::read(&audio).unwrap(), bytes);
    }
}
