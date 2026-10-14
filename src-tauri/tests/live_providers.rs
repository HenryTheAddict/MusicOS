//! Explicit network smoke check; excluded from offline CI tests.
use musicos_tauri::providers;
use std::sync::atomic::{AtomicBool, AtomicU64};
#[test]
#[ignore = "requires live Apple, MusicBrainz and Wikimedia services"]
fn public_catalog_and_verified_artist() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let rows = runtime
        .block_on(providers::discover(
            "Daft Punk",
            "albums",
            None,
            &AtomicU64::new(1),
            1,
        ))
        .unwrap();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| !r.local && r.id.starts_with("apple/")));
    if let Some(cover) = rows.iter().find_map(|r| r.cover.as_deref()) {
        assert!(providers::cache_artwork(cover).unwrap().is_file());
    }
    let artist = providers::artist_info(
        std::path::Path::new("unused"),
        "Daft Punk",
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(artist.artist_mbid.is_some());
    assert!(artist.biography.is_some());
    assert!(artist.source_url.is_some());
    if artist.portrait.is_some() {
        assert!(artist.portrait_source_url.is_some());
    }
    println!(
        "{} online albums; artist identity {:?}; biography {:?}; photo {:?}",
        rows.len(),
        artist.artist_mbid,
        artist.source_url,
        artist.portrait_source_url
    );
}
