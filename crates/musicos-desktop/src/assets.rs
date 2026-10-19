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
            "icons/ui/3dots(incircle).svg" => {
                include_bytes!("../../../assets/icons/ui/3dots(incircle).svg")
            }
            "icons/ui/3dots_(nocircle).svg" => {
                include_bytes!("../../../assets/icons/ui/3dots_(nocircle).svg")
            }
            "icons/ui/Album.svg" => include_bytes!("../../../assets/icons/ui/Album.svg"),
            "icons/ui/Artist.svg" => include_bytes!("../../../assets/icons/ui/Artist.svg"),
            "icons/ui/Delete.svg" => include_bytes!("../../../assets/icons/ui/Delete.svg"),
            "icons/ui/File.svg" => include_bytes!("../../../assets/icons/ui/File.svg"),
            "icons/ui/Genre.svg" => include_bytes!("../../../assets/icons/ui/Genre.svg"),
            "icons/ui/Lossless.svg" => {
                include_bytes!("../../../assets/icons/ui/Lossless.svg")
            }
            "icons/ui/Lossy.svg" => include_bytes!("../../../assets/icons/ui/Lossy.svg"),
            "icons/ui/NetSearch.svg" => {
                include_bytes!("../../../assets/icons/ui/NetSearch.svg")
            }
            "icons/ui/Playlist.svg" => {
                include_bytes!("../../../assets/icons/ui/Playlist.svg")
            }
            "icons/ui/RepeatOFF.svg" => {
                include_bytes!("../../../assets/icons/ui/RepeatOFF.svg")
            }
            "icons/ui/RepeatON.svg" => {
                include_bytes!("../../../assets/icons/ui/RepeatON.svg")
            }
            "icons/ui/RepeatONE.svg" => {
                include_bytes!("../../../assets/icons/ui/RepeatONE.svg")
            }
            "icons/ui/ShuffleOFF.svg" => {
                include_bytes!("../../../assets/icons/ui/ShuffleOFF.svg")
            }
            "icons/ui/ShuffleON.svg" => {
                include_bytes!("../../../assets/icons/ui/ShuffleON.svg")
            }
            "icons/ui/ShufflePlay.svg" => {
                include_bytes!("../../../assets/icons/ui/ShufflePlay.svg")
            }
            "icons/ui/Song.svg" => include_bytes!("../../../assets/icons/ui/Song.svg"),
            "icons/ui/Sparklist.svg" => {
                include_bytes!("../../../assets/icons/ui/Sparklist.svg")
            }
            "icons/ui/autostart.svg" => {
                include_bytes!("../../../assets/icons/ui/autostart.svg")
            }
            "icons/ui/device.svg" => include_bytes!("../../../assets/icons/ui/device.svg"),
            "icons/ui/hearted.svg" => include_bytes!("../../../assets/icons/ui/hearted.svg"),
            "icons/ui/next.svg" => include_bytes!("../../../assets/icons/ui/next.svg"),
            "icons/ui/noARTIST.svg" => {
                include_bytes!("../../../assets/icons/ui/noARTIST.svg")
            }
            "icons/ui/noCOVER.svg" => include_bytes!("../../../assets/icons/ui/noCOVER.svg"),
            "icons/ui/noheart.svg" => include_bytes!("../../../assets/icons/ui/noheart.svg"),
            "icons/ui/notFOUND.svg" => {
                include_bytes!("../../../assets/icons/ui/notFOUND.svg")
            }
            "icons/ui/pause.svg" => include_bytes!("../../../assets/icons/ui/pause.svg"),
            "icons/ui/play.svg" => include_bytes!("../../../assets/icons/ui/play.svg"),
            "icons/ui/prev.svg" => include_bytes!("../../../assets/icons/ui/prev.svg"),
            "icons/ui/openMiniplayer.svg" => include_bytes!("../../../assets/icons/ui/openMiniplayer.svg"),
            "icons/ui/CloseMiniplayer.svg" => include_bytes!("../../../assets/icons/ui/CloseMiniplayer.svg"),
            "icons/ui/Light mode.svg" => include_bytes!("../../../assets/icons/ui/Light mode.svg"),
            "icons/ui/Dark Mode.svg" => include_bytes!("../../../assets/icons/ui/Dark Mode.svg"),
            "icons/ui/System.svg" => include_bytes!("../../../assets/icons/ui/System.svg"),
            "icons/ui/Pin.svg" => include_bytes!("../../../assets/icons/ui/Pin.svg"),
            "icons/ui/unPin.svg" => include_bytes!("../../../assets/icons/ui/unPin.svg"),
            "icons/ui/queue.svg" => include_bytes!("../../../assets/icons/ui/queue.svg"),
            "icons/ui/sync.svg" => include_bytes!("../../../assets/icons/ui/sync.svg"),
            "icons/ui/that_one_i_in_a_circle_icon.svg" => {
                include_bytes!("../../../assets/icons/ui/that_one_i_in_a_circle_icon.svg")
            }
            "icons/ui/tinyMaximize.svg" => {
                include_bytes!("../../../assets/icons/ui/tinyMaximize.svg")
            }
            "icons/ui/tinyMinimize.svg" => {
                include_bytes!("../../../assets/icons/ui/tinyMinimize.svg")
            }
            "icons/ui/tinyNext.svg" => {
                include_bytes!("../../../assets/icons/ui/tinyNext.svg")
            }
            "icons/ui/tinyPause.svg" => {
                include_bytes!("../../../assets/icons/ui/tinyPause.svg")
            }
            "icons/ui/tinyPlay.svg" => {
                include_bytes!("../../../assets/icons/ui/tinyPlay.svg")
            }
            "icons/ui/tinyPrev.svg" => {
                include_bytes!("../../../assets/icons/ui/tinyPrev.svg")
            }
            "icons/ui/tinySync.svg" => {
                include_bytes!("../../../assets/icons/ui/tinySync.svg")
            }
            "icons/ui/tinyX.svg" => include_bytes!("../../../assets/icons/ui/tinyX.svg"),

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
        } else if path == "icons/ui" {
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
