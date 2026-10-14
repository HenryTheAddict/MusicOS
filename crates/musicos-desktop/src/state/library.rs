use std::{cmp::Ordering, collections::BTreeMap, sync::Arc};

use crate::models::{
    DesktopSection, LibraryPresentation, LibraryScopeKey, SortDescriptor, SortField, Track,
};

pub type TrackId = i64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlbumId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtistId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryScope {
    Music,
    Songs,
    Albums,
    Artists,
    Genres,
    Favorites,
    History,
    Queue,
    Playlist(i64),
}

impl LibraryScope {
    pub fn key(&self) -> LibraryScopeKey {
        match self {
            Self::Music => LibraryScopeKey::Music,
            Self::Songs => LibraryScopeKey::Songs,
            Self::Albums => LibraryScopeKey::Albums,
            Self::Artists => LibraryScopeKey::Artists,
            Self::Genres => LibraryScopeKey::Genres,
            Self::Favorites => LibraryScopeKey::Favorites,
            Self::History => LibraryScopeKey::History,
            Self::Queue => LibraryScopeKey::Queue,
            Self::Playlist(_) => LibraryScopeKey::Playlist,
        }
    }

    pub fn from_section(section: DesktopSection) -> Option<Self> {
        match section {
            DesktopSection::Music => Some(Self::Music),
            DesktopSection::Songs => Some(Self::Songs),
            DesktopSection::Albums => Some(Self::Albums),
            DesktopSection::Artists => Some(Self::Artists),
            DesktopSection::Genres => Some(Self::Genres),
            DesktopSection::Favorites => Some(Self::Favorites),
            DesktopSection::Queue => Some(Self::Queue),
            _ => None,
        }
    }

    pub fn section(&self) -> DesktopSection {
        match self {
            Self::Music => DesktopSection::Music,
            Self::Songs => DesktopSection::Songs,
            Self::Albums => DesktopSection::Albums,
            Self::Artists => DesktopSection::Artists,
            Self::Genres => DesktopSection::Genres,
            Self::Favorites => DesktopSection::Favorites,
            Self::History => DesktopSection::Songs,
            Self::Queue => DesktopSection::Queue,
            Self::Playlist(_) => DesktopSection::Playlists,
        }
    }

    pub fn supports(&self, presentation: LibraryPresentation) -> bool {
        match self {
            Self::Music | Self::Albums => true,
            Self::Artists | Self::Genres => presentation != LibraryPresentation::Flow,
            Self::Songs | Self::Favorites | Self::History | Self::Queue | Self::Playlist(_) => {
                presentation == LibraryPresentation::Table
            }
        }
    }

    pub fn default_presentation(&self) -> LibraryPresentation {
        match self {
            Self::Albums | Self::Artists => LibraryPresentation::Grid,
            _ => LibraryPresentation::Table,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryDrilldown {
    Album(AlbumId),
    Artist(ArtistId),
    Genre(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryRoute {
    pub scope: LibraryScope,
    pub presentation: LibraryPresentation,
    pub drilldown: Option<LibraryDrilldown>,
}

impl LibraryRoute {
    pub fn new(scope: LibraryScope, presentation: LibraryPresentation) -> Self {
        let presentation = if scope.supports(presentation) {
            presentation
        } else {
            scope.default_presentation()
        };
        Self {
            scope,
            presentation,
            drilldown: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppRoute {
    Library(LibraryRoute),
    Section(DesktopSection),
}

impl AppRoute {
    pub fn section(&self) -> DesktopSection {
        match self {
            Self::Library(route) => route.scope.section(),
            Self::Section(section) => *section,
        }
    }

    pub fn library(&self) -> Option<&LibraryRoute> {
        match self {
            Self::Library(route) => Some(route),
            Self::Section(_) => None,
        }
    }

    pub fn library_mut(&mut self) -> Option<&mut LibraryRoute> {
        match self {
            Self::Library(route) => Some(route),
            Self::Section(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlbumCollection {
    pub id: AlbumId,
    pub title: String,
    pub artist: String,
    pub year: Option<i32>,
    pub track_indices: Vec<usize>,
    pub representative_track: TrackId,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct ArtistCollection {
    pub id: ArtistId,
    pub name: String,
    pub album_ids: Vec<AlbumId>,
    pub track_indices: Vec<usize>,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct GenreCollection {
    pub name: String,
    pub album_ids: Vec<AlbumId>,
    pub track_indices: Vec<usize>,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, Default)]
pub struct LibraryIndex {
    pub revision: u64,
    pub tracks: Arc<[Track]>,
    pub track_by_id: BTreeMap<TrackId, usize>,
    pub filtered_tracks: Vec<usize>,
    pub albums: Vec<AlbumCollection>,
    pub artists: Vec<ArtistCollection>,
    pub genres: Vec<GenreCollection>,
}

impl LibraryIndex {
    pub fn build(
        tracks: Arc<[Track]>,
        scope: &LibraryScope,
        query: &str,
        sort: SortDescriptor,
        revision: u64,
    ) -> Self {
        let track_by_id = tracks
            .iter()
            .enumerate()
            .map(|(index, track)| (track.id, index))
            .collect();
        let query = query.trim().to_ascii_lowercase();
        let mut filtered_tracks = tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| {
                let scope_matches = match scope {
                    LibraryScope::Favorites => track.favorited,
                    _ => true,
                };
                scope_matches
                    && (query.is_empty()
                        || track.title.to_ascii_lowercase().contains(&query)
                        || track.artist.to_ascii_lowercase().contains(&query)
                        || track.album.to_ascii_lowercase().contains(&query)
                        || track.genre.to_ascii_lowercase().contains(&query))
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        filtered_tracks
            .sort_by(|left, right| compare_tracks(&tracks[*left], &tracks[*right], sort.field));
        if sort.descending {
            filtered_tracks.reverse();
        }

        let mut album_groups = BTreeMap::<(String, String), Vec<usize>>::new();
        for index in &filtered_tracks {
            let track = &tracks[*index];
            album_groups
                .entry((
                    normalized_collection_key(&track.album),
                    normalized_collection_key(primary_album_artist(&track.artist)),
                ))
                .or_default()
                .push(*index);
        }
        let mut albums = album_groups
            .into_iter()
            .map(|((title_key, artist_key), mut track_indices)| {
                track_indices.sort_by(|left, right| {
                    let left = &tracks[*left];
                    let right = &tracks[*right];
                    left.disc_number
                        .cmp(&right.disc_number)
                        .then_with(|| left.track_number.cmp(&right.track_number))
                        .then_with(|| {
                            left.title
                                .to_ascii_lowercase()
                                .cmp(&right.title.to_ascii_lowercase())
                        })
                        .then_with(|| left.id.cmp(&right.id))
                });
                let representative_track = track_indices
                    .iter()
                    .find(|index| tracks[**index].cover_art_path.is_some())
                    .copied()
                    .unwrap_or(track_indices[0]);
                let title = preferred_label(
                    track_indices
                        .iter()
                        .map(|index| tracks[*index].album.as_str()),
                    "Unknown album",
                );
                let artist = preferred_label(
                    track_indices
                        .iter()
                        .map(|index| primary_album_artist(&tracks[*index].artist)),
                    "Unknown artist",
                );
                AlbumCollection {
                    id: AlbumId(format!("{title_key}\u{1f}{artist_key}")),
                    title,
                    artist,
                    year: track_indices
                        .iter()
                        .map(|index| tracks[*index].year)
                        .find(|year| *year > 0),
                    representative_track: tracks[representative_track].id,
                    duration_seconds: track_indices
                        .iter()
                        .map(|index| tracks[*index].duration.max(0) as u64)
                        .sum(),
                    track_indices,
                }
            })
            .collect::<Vec<_>>();
        albums.sort_by(|left, right| {
            left.title
                .to_ascii_lowercase()
                .cmp(&right.title.to_ascii_lowercase())
                .then_with(|| {
                    left.artist
                        .to_ascii_lowercase()
                        .cmp(&right.artist.to_ascii_lowercase())
                })
        });

        let album_id_by_track = albums
            .iter()
            .flat_map(|album| {
                album
                    .track_indices
                    .iter()
                    .map(move |track_index| (*track_index, album.id.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        let mut artist_groups = BTreeMap::<String, Vec<usize>>::new();
        let mut genre_groups = BTreeMap::<String, Vec<usize>>::new();
        for index in &filtered_tracks {
            let track = &tracks[*index];
            artist_groups
                .entry(normalized_collection_key(&track.artist))
                .or_default()
                .push(*index);
            let genre = if track.genre.trim().is_empty() {
                "Unknown genre".to_string()
            } else {
                track.genre.clone()
            };
            genre_groups.entry(genre).or_default().push(*index);
        }
        let artists = artist_groups
            .into_iter()
            .map(|(id, track_indices)| ArtistCollection {
                name: preferred_label(
                    track_indices
                        .iter()
                        .map(|index| tracks[*index].artist.as_str()),
                    "Unknown artist",
                ),
                id: ArtistId(id),
                album_ids: unique_album_ids(&track_indices, &album_id_by_track),
                duration_seconds: track_indices
                    .iter()
                    .map(|index| tracks[*index].duration.max(0) as u64)
                    .sum(),
                track_indices,
            })
            .collect();
        let genres = genre_groups
            .into_iter()
            .map(|(name, track_indices)| GenreCollection {
                album_ids: unique_album_ids(&track_indices, &album_id_by_track),
                duration_seconds: track_indices
                    .iter()
                    .map(|index| tracks[*index].duration.max(0) as u64)
                    .sum(),
                name,
                track_indices,
            })
            .collect();

        Self {
            revision,
            tracks,
            track_by_id,
            filtered_tracks,
            albums,
            artists,
            genres,
        }
    }
}

fn primary_album_artist(value: &str) -> &str {
    let lower = value.to_ascii_lowercase();
    [
        " featuring ",
        " feat. ",
        " feat ",
        " ft. ",
        " ft ",
        " & ",
        ";",
    ]
    .into_iter()
    .filter_map(|marker| lower.find(marker))
    .min()
    .map(|index| value[..index].trim())
    .filter(|value| !value.is_empty())
    .unwrap_or(value.trim())
}

fn normalized_collection_key(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut pending_space = false;
    for character in value.trim().chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if pending_space && !output.is_empty() {
                output.push(' ');
            }
            output.push(character);
            pending_space = false;
        } else {
            pending_space = true;
        }
    }
    if output.is_empty() {
        "unknown".to_string()
    } else {
        output
    }
}

fn preferred_label<'a>(values: impl Iterator<Item = &'a str>, fallback: &str) -> String {
    let mut counts = BTreeMap::<String, (usize, String)>::new();
    for value in values.map(str::trim).filter(|value| !value.is_empty()) {
        let key = normalized_collection_key(value);
        let entry = counts.entry(key).or_insert((0, value.to_string()));
        entry.0 += 1;
        if value.len() < entry.1.len() {
            entry.1 = value.to_string();
        }
    }
    counts
        .into_values()
        .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(&left.1)))
        .map(|(_, label)| label)
        .unwrap_or_else(|| fallback.to_string())
}

fn unique_album_ids(
    track_indices: &[usize],
    album_id_by_track: &BTreeMap<usize, AlbumId>,
) -> Vec<AlbumId> {
    let mut ids = track_indices
        .iter()
        .filter_map(|index| album_id_by_track.get(index).cloned())
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn compare_tracks(left: &Track, right: &Track, field: SortField) -> Ordering {
    let order = match field {
        SortField::Title => left
            .title
            .to_ascii_lowercase()
            .cmp(&right.title.to_ascii_lowercase()),
        SortField::Artist => left
            .artist
            .to_ascii_lowercase()
            .cmp(&right.artist.to_ascii_lowercase()),
        SortField::Album => left
            .album
            .to_ascii_lowercase()
            .cmp(&right.album.to_ascii_lowercase()),
        SortField::Genre => left
            .genre
            .to_ascii_lowercase()
            .cmp(&right.genre.to_ascii_lowercase()),
        SortField::Year => left.year.cmp(&right.year),
        SortField::TrackNumber => left
            .disc_number
            .cmp(&right.disc_number)
            .then_with(|| left.track_number.cmp(&right.track_number)),
        SortField::Duration => left.duration.cmp(&right.duration),
        SortField::Rating => left.rating.cmp(&right.rating),
        SortField::PlayCount => left.play_count.cmp(&right.play_count),
        SortField::DateAdded => left.indexed_at.cmp(&right.indexed_at),
    };
    order.then_with(|| left.id.cmp(&right.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn albums_are_unique_and_tracks_use_disc_order() {
        let tracks: Arc<[Track]> = vec![
            Track {
                id: 2,
                title: "Second".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                disc_number: 1,
                track_number: 2,
                duration: 20,
                ..Track::default()
            },
            Track {
                id: 1,
                title: "First".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                disc_number: 1,
                track_number: 1,
                duration: 10,
                ..Track::default()
            },
        ]
        .into();
        let index = LibraryIndex::build(
            tracks,
            &LibraryScope::Albums,
            "",
            SortDescriptor::default(),
            1,
        );
        assert_eq!(index.albums.len(), 1);
        assert_eq!(index.albums[0].track_indices, vec![1, 0]);
        assert_eq!(index.albums[0].duration_seconds, 30);
    }

    #[test]
    fn presentation_compatibility_is_explicit() {
        assert!(LibraryScope::Albums.supports(LibraryPresentation::Flow));
        assert!(!LibraryScope::Artists.supports(LibraryPresentation::Flow));
        assert!(!LibraryScope::Songs.supports(LibraryPresentation::Grid));
    }

    #[test]
    fn album_identity_merges_case_punctuation_and_featured_artist_variants() {
        let tracks: Arc<[Track]> = vec![
            Track {
                id: 1,
                title: "One".into(),
                artist: "Artist feat. Guest".into(),
                album: "A  Great—Album".into(),
                ..Track::default()
            },
            Track {
                id: 2,
                title: "Two".into(),
                artist: "artist".into(),
                album: "a great album".into(),
                ..Track::default()
            },
        ]
        .into();
        let index = LibraryIndex::build(
            tracks,
            &LibraryScope::Albums,
            "",
            SortDescriptor::default(),
            1,
        );
        assert_eq!(index.albums.len(), 1);
        assert_eq!(index.albums[0].track_indices.len(), 2);
    }
}
