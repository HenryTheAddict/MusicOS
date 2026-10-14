use std::borrow::Cow;

use anyhow::Result;
use gpui::{AssetSource, SharedString};

#[derive(Debug, Default, Clone, Copy)]
pub struct DesktopAssets;

impl AssetSource for DesktopAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes: &'static [u8] = match path {
            "icons/desktop/previous.svg" => {
                include_bytes!("../../../assets/icons/desktop/previous.svg")
            }
            "icons/desktop/play.svg" => include_bytes!("../../../assets/icons/desktop/play.svg"),
            "icons/desktop/pause.svg" => include_bytes!("../../../assets/icons/desktop/pause.svg"),
            "icons/desktop/next.svg" => include_bytes!("../../../assets/icons/desktop/next.svg"),
            "icons/desktop/shuffle.svg" => {
                include_bytes!("../../../assets/icons/desktop/shuffle.svg")
            }
            "icons/desktop/repeat.svg" => {
                include_bytes!("../../../assets/icons/desktop/repeat.svg")
            }
            "icons/desktop/repeat-one.svg" => {
                include_bytes!("../../../assets/icons/desktop/repeat-one.svg")
            }
            "icons/desktop/search.svg" => {
                include_bytes!("../../../assets/icons/desktop/search.svg")
            }
            "icons/desktop/activity.svg" => {
                include_bytes!("../../../assets/icons/desktop/activity.svg")
            }
            "icons/desktop/import.svg" => {
                include_bytes!("../../../assets/icons/desktop/import.svg")
            }
            "icons/desktop/music.svg" => include_bytes!("../../../assets/icons/desktop/music.svg"),
            "icons/desktop/album.svg" => include_bytes!("../../../assets/icons/desktop/album.svg"),
            "icons/desktop/artist.svg" => {
                include_bytes!("../../../assets/icons/desktop/artist.svg")
            }
            "icons/desktop/genre.svg" => include_bytes!("../../../assets/icons/desktop/genre.svg"),
            "icons/desktop/song.svg" => include_bytes!("../../../assets/icons/desktop/song.svg"),
            "icons/desktop/favorite.svg" => {
                include_bytes!("../../../assets/icons/desktop/favorite.svg")
            }
            "icons/desktop/playlist.svg" => {
                include_bytes!("../../../assets/icons/desktop/playlist.svg")
            }
            "icons/desktop/queue.svg" => include_bytes!("../../../assets/icons/desktop/queue.svg"),
            "icons/desktop/device.svg" => {
                include_bytes!("../../../assets/icons/desktop/device.svg")
            }
            "icons/desktop/sync.svg" => include_bytes!("../../../assets/icons/desktop/sync.svg"),
            "icons/desktop/update.svg" => {
                include_bytes!("../../../assets/icons/desktop/update.svg")
            }
            "icons/desktop/diagnostics.svg" => {
                include_bytes!("../../../assets/icons/desktop/diagnostics.svg")
            }
            "icons/desktop/settings.svg" => {
                include_bytes!("../../../assets/icons/desktop/settings.svg")
            }
            "icons/desktop/record-placeholder.svg" => {
                include_bytes!("../../../assets/icons/desktop/record-placeholder.svg")
            }
            "icons/desktop/disclosure.svg" => {
                include_bytes!("../../../assets/icons/desktop/disclosure.svg")
            }
            "icons/desktop/check.svg" => include_bytes!("../../../assets/icons/desktop/check.svg"),
            "icons/desktop/warning.svg" => {
                include_bytes!("../../../assets/icons/desktop/warning.svg")
            }
            "icons/desktop/error.svg" => include_bytes!("../../../assets/icons/desktop/error.svg"),
            "icons/desktop/expand.svg" => {
                include_bytes!("../../../assets/icons/desktop/expand.svg")
            }
            "icons/desktop/trash.svg" => include_bytes!("../../../assets/icons/desktop/trash.svg"),
            "icons/desktop/volume.svg" => {
                include_bytes!("../../../assets/icons/desktop/volume.svg")
            }

            // Sketch design icons
            "icons/sketch/3dots(incircle).svg" => {
                include_bytes!("../../../assets/icons/sketch/3dots(incircle).svg")
            }
            "icons/sketch/3dots_(nocircle).svg" => {
                include_bytes!("../../../assets/icons/sketch/3dots_(nocircle).svg")
            }
            "icons/sketch/Album.svg" => include_bytes!("../../../assets/icons/sketch/Album.svg"),
            "icons/sketch/Artist.svg" => include_bytes!("../../../assets/icons/sketch/Artist.svg"),
            "icons/sketch/Delete.svg" => include_bytes!("../../../assets/icons/sketch/Delete.svg"),
            "icons/sketch/File.svg" => include_bytes!("../../../assets/icons/sketch/File.svg"),
            "icons/sketch/Genre.svg" => include_bytes!("../../../assets/icons/sketch/Genre.svg"),
            "icons/sketch/Lossless.svg" => {
                include_bytes!("../../../assets/icons/sketch/Lossless.svg")
            }
            "icons/sketch/Lossy.svg" => include_bytes!("../../../assets/icons/sketch/Lossy.svg"),
            "icons/sketch/NetSearch.svg" => {
                include_bytes!("../../../assets/icons/sketch/NetSearch.svg")
            }
            "icons/sketch/Playlist.svg" => {
                include_bytes!("../../../assets/icons/sketch/Playlist.svg")
            }
            "icons/sketch/RepeatOFF.svg" => {
                include_bytes!("../../../assets/icons/sketch/RepeatOFF.svg")
            }
            "icons/sketch/RepeatON.svg" => {
                include_bytes!("../../../assets/icons/sketch/RepeatON.svg")
            }
            "icons/sketch/RepeatONE.svg" => {
                include_bytes!("../../../assets/icons/sketch/RepeatONE.svg")
            }
            "icons/sketch/ShuffleOFF.svg" => {
                include_bytes!("../../../assets/icons/sketch/ShuffleOFF.svg")
            }
            "icons/sketch/ShuffleON.svg" => {
                include_bytes!("../../../assets/icons/sketch/ShuffleON.svg")
            }
            "icons/sketch/ShufflePlay.svg" => {
                include_bytes!("../../../assets/icons/sketch/ShufflePlay.svg")
            }
            "icons/sketch/Song.svg" => include_bytes!("../../../assets/icons/sketch/Song.svg"),
            "icons/sketch/Sparklist.svg" => {
                include_bytes!("../../../assets/icons/sketch/Sparklist.svg")
            }
            "icons/sketch/autostart.svg" => {
                include_bytes!("../../../assets/icons/sketch/autostart.svg")
            }
            "icons/sketch/device.svg" => include_bytes!("../../../assets/icons/sketch/device.svg"),
            "icons/sketch/hearted.svg" => include_bytes!("../../../assets/icons/sketch/hearted.svg"),
            "icons/sketch/next.svg" => include_bytes!("../../../assets/icons/sketch/next.svg"),
            "icons/sketch/noARTIST.svg" => {
                include_bytes!("../../../assets/icons/sketch/noARTIST.svg")
            }
            "icons/sketch/noCOVER.svg" => include_bytes!("../../../assets/icons/sketch/noCOVER.svg"),
            "icons/sketch/noheart.svg" => include_bytes!("../../../assets/icons/sketch/noheart.svg"),
            "icons/sketch/notFOUND.svg" => {
                include_bytes!("../../../assets/icons/sketch/notFOUND.svg")
            }
            "icons/sketch/pause.svg" => include_bytes!("../../../assets/icons/sketch/pause.svg"),
            "icons/sketch/play.svg" => include_bytes!("../../../assets/icons/sketch/play.svg"),
            "icons/sketch/prev.svg" => include_bytes!("../../../assets/icons/sketch/prev.svg"),
            "icons/sketch/queue.svg" => include_bytes!("../../../assets/icons/sketch/queue.svg"),
            "icons/sketch/sync.svg" => include_bytes!("../../../assets/icons/sketch/sync.svg"),
            "icons/sketch/that_one_i_in_a_circle_icon.svg" => {
                include_bytes!("../../../assets/icons/sketch/that_one_i_in_a_circle_icon.svg")
            }
            "icons/sketch/tinyMaximize.svg" => {
                include_bytes!("../../../assets/icons/sketch/tinyMaximize.svg")
            }
            "icons/sketch/tinyMinimize.svg" => {
                include_bytes!("../../../assets/icons/sketch/tinyMinimize.svg")
            }
            "icons/sketch/tinyNext.svg" => {
                include_bytes!("../../../assets/icons/sketch/tinyNext.svg")
            }
            "icons/sketch/tinyPause.svg" => {
                include_bytes!("../../../assets/icons/sketch/tinyPause.svg")
            }
            "icons/sketch/tinyPlay.svg" => {
                include_bytes!("../../../assets/icons/sketch/tinyPlay.svg")
            }
            "icons/sketch/tinyPrev.svg" => {
                include_bytes!("../../../assets/icons/sketch/tinyPrev.svg")
            }
            "icons/sketch/tinySync.svg" => {
                include_bytes!("../../../assets/icons/sketch/tinySync.svg")
            }
            "icons/sketch/tinyX.svg" => include_bytes!("../../../assets/icons/sketch/tinyX.svg"),

            // Mockup covers
            "covers/sky.png" => include_bytes!("../../../assets/covers/sky.png"),
            "covers/tritower.png" => include_bytes!("../../../assets/covers/tritower.png"),
            "covers/Overcast.png" => include_bytes!("../../../assets/covers/Overcast.png"),
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        if path == "icons/desktop" {
            Ok([
                "previous.svg",
                "play.svg",
                "pause.svg",
                "next.svg",
                "shuffle.svg",
                "repeat.svg",
                "repeat-one.svg",
                "search.svg",
                "activity.svg",
                "import.svg",
                "music.svg",
                "album.svg",
                "artist.svg",
                "genre.svg",
                "song.svg",
                "favorite.svg",
                "playlist.svg",
                "queue.svg",
                "device.svg",
                "sync.svg",
                "update.svg",
                "diagnostics.svg",
                "settings.svg",
                "record-placeholder.svg",
                "disclosure.svg",
                "check.svg",
                "warning.svg",
                "error.svg",
                "expand.svg",
            ]
            .into_iter()
            .map(SharedString::from)
            .collect())
        } else if path == "icons/sketch" {
            Ok([
                "3dots(incircle).svg",
                "3dots_(nocircle).svg",
                "Album.svg",
                "Artist.svg",
                "Delete.svg",
                "File.svg",
                "Genre.svg",
                "Lossless.svg",
                "Lossy.svg",
                "NetSearch.svg",
                "Playlist.svg",
                "RepeatOFF.svg",
                "RepeatON.svg",
                "RepeatONE.svg",
                "ShuffleOFF.svg",
                "ShuffleON.svg",
                "ShufflePlay.svg",
                "Song.svg",
                "Sparklist.svg",
                "autostart.svg",
                "device.svg",
                "hearted.svg",
                "next.svg",
                "noARTIST.svg",
                "noCOVER.svg",
                "noheart.svg",
                "notFOUND.svg",
                "pause.svg",
                "play.svg",
                "prev.svg",
                "queue.svg",
                "sync.svg",
                "that_one_i_in_a_circle_icon.svg",
                "tinyMaximize.svg",
                "tinyMinimize.svg",
                "tinyNext.svg",
                "tinyPause.svg",
                "tinyPlay.svg",
                "tinyPrev.svg",
                "tinySync.svg",
                "tinyX.svg",
            ]
            .into_iter()
            .map(SharedString::from)
            .collect())
        } else {
            Ok(Vec::new())
        }
    }
}
