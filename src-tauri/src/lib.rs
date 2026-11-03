mod launch_palette;
mod product;
mod desktop_metadata;
pub mod app;
pub mod library;
pub mod preferences;
pub mod providers;
pub mod integrations;
pub mod metadata;

pub mod collections;

pub mod analysis;

pub mod spark_audio;

pub mod structure;

pub mod dj_plan;

pub mod spark_engine;

pub mod embeddings;

pub mod audio_collections;

pub mod spark_commands;
pub mod spark_recommendations;
pub mod home;

pub mod spark_install;

#[cfg(target_os="macos")]
pub mod system_media;
#[cfg(target_os="macos")]
pub mod native_playback_menu;

pub mod portrait;

pub mod beat;

pub mod depth;

pub mod spark_artwork;

mod onsets;

pub mod file_organization;

pub mod source_forget;

pub mod album_metadata;
pub mod file_tags;

pub mod artwork_renditions;

#[cfg(test)]mod reliability_tests;

pub mod wifi;

pub mod lyrics;

#[cfg(target_os="macos")]mod trackpad_phase;

pub mod lyric_formats;

pub mod apple_enrichment;

pub mod lyric_sidecars;

pub mod lyric_edition;

pub mod haptics;

pub mod visualizer;

pub mod player_shortcuts;
