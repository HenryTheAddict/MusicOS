use serde::{Deserialize, Serialize};

use crate::models::Track;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartRuleField {
    Genre,
    Artist,
    Album,
    Year,
    Rating,
    PlayCount,
    Duration,
    Favorited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartRuleOp {
    Contains,
    Equals,
    GreaterThan,
    LessThan,
    IsTrue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartRule {
    pub field: SmartRuleField,
    pub op: SmartRuleOp,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartMatch {
    All,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartPlaylistDef {
    pub name: String,
    pub match_mode: SmartMatch,
    pub rules: Vec<SmartRule>,
    pub limit: Option<usize>,
}

impl SmartPlaylistDef {
    pub fn evaluate(&self, tracks: &[Track]) -> Vec<Track> {
        let mut result: Vec<Track> = tracks
            .iter()
            .filter(|track| self.matches(track))
            .cloned()
            .collect();
        if let Some(limit) = self.limit {
            result.truncate(limit);
        }
        result
    }

    fn matches(&self, track: &Track) -> bool {
        if self.rules.is_empty() {
            return true;
        }
        match self.match_mode {
            SmartMatch::All => self.rules.iter().all(|rule| rule.matches(track)),
            SmartMatch::Any => self.rules.iter().any(|rule| rule.matches(track)),
        }
    }
}

impl SmartRule {
    fn matches(&self, track: &Track) -> bool {
        match self.field {
            SmartRuleField::Genre => self.match_str(&track.genre),
            SmartRuleField::Artist => self.match_str(&track.artist),
            SmartRuleField::Album => self.match_str(&track.album),
            SmartRuleField::Year => self.match_i64(track.year as i64),
            SmartRuleField::Rating => self.match_i64(track.rating as i64),
            SmartRuleField::PlayCount => self.match_i64(track.play_count as i64),
            SmartRuleField::Duration => self.match_i64(track.duration as i64),
            SmartRuleField::Favorited => matches!(self.op, SmartRuleOp::IsTrue) && track.favorited,
        }
    }

    fn match_str(&self, haystack: &str) -> bool {
        let value = self.value.trim();
        match self.op {
            SmartRuleOp::Contains => haystack
                .to_ascii_lowercase()
                .contains(&value.to_ascii_lowercase()),
            SmartRuleOp::Equals => haystack.eq_ignore_ascii_case(value),
            _ => false,
        }
    }

    fn match_i64(&self, actual: i64) -> bool {
        let expected: i64 = self.value.trim().parse().unwrap_or_default();
        match self.op {
            SmartRuleOp::Equals => actual == expected,
            SmartRuleOp::GreaterThan => actual > expected,
            SmartRuleOp::LessThan => actual < expected,
            _ => false,
        }
    }
}

pub fn default_smart_playlists() -> Vec<SmartPlaylistDef> {
    vec![
        SmartPlaylistDef {
            name: "Most Played".to_string(),
            match_mode: SmartMatch::All,
            rules: vec![SmartRule {
                field: SmartRuleField::PlayCount,
                op: SmartRuleOp::GreaterThan,
                value: "5".to_string(),
            }],
            limit: Some(50),
        },
        SmartPlaylistDef {
            name: "Favorites".to_string(),
            match_mode: SmartMatch::All,
            rules: vec![SmartRule {
                field: SmartRuleField::Favorited,
                op: SmartRuleOp::IsTrue,
                value: String::new(),
            }],
            limit: None,
        },
        SmartPlaylistDef {
            name: "Highly Rated".to_string(),
            match_mode: SmartMatch::All,
            rules: vec![SmartRule {
                field: SmartRuleField::Rating,
                op: SmartRuleOp::GreaterThan,
                value: "3".to_string(),
            }],
            limit: Some(100),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(title: &str, genre: &str, play_count: i32, favorited: bool) -> Track {
        Track {
            id: 1,
            title: title.to_string(),
            genre: genre.to_string(),
            play_count,
            favorited,
            ..Track::default()
        }
    }

    #[test]
    fn smart_playlist_filters_by_genre_contains() {
        let def = SmartPlaylistDef {
            name: "Rock".to_string(),
            match_mode: SmartMatch::All,
            rules: vec![SmartRule {
                field: SmartRuleField::Genre,
                op: SmartRuleOp::Contains,
                value: "rock".to_string(),
            }],
            limit: None,
        };
        let tracks = vec![
            track("A", "Classic Rock", 0, false),
            track("B", "Jazz", 0, false),
            track("C", "Progressive Rock", 0, false),
        ];
        let result = def.evaluate(&tracks);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, "A");
        assert_eq!(result[1].title, "C");
    }

    #[test]
    fn smart_playlist_respects_limit() {
        let def = SmartPlaylistDef {
            name: "Top".to_string(),
            match_mode: SmartMatch::All,
            rules: vec![SmartRule {
                field: SmartRuleField::PlayCount,
                op: SmartRuleOp::GreaterThan,
                value: "0".to_string(),
            }],
            limit: Some(2),
        };
        let tracks = vec![
            track("A", "", 10, false),
            track("B", "", 5, false),
            track("C", "", 3, false),
        ];
        assert_eq!(def.evaluate(&tracks).len(), 2);
    }

    #[test]
    fn smart_playlist_any_mode() {
        let def = SmartPlaylistDef {
            name: "Mixed".to_string(),
            match_mode: SmartMatch::Any,
            rules: vec![
                SmartRule {
                    field: SmartRuleField::Genre,
                    op: SmartRuleOp::Equals,
                    value: "Jazz".to_string(),
                },
                SmartRule {
                    field: SmartRuleField::Favorited,
                    op: SmartRuleOp::IsTrue,
                    value: String::new(),
                },
            ],
            limit: None,
        };
        let tracks = vec![
            track("A", "Jazz", 0, false),
            track("B", "Rock", 0, true),
            track("C", "Pop", 0, false),
        ];
        assert_eq!(def.evaluate(&tracks).len(), 2);
    }
}
