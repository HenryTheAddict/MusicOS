pub mod deck_playback;
pub mod decoded_audio;
pub mod dj_audio;
pub mod listening;
pub mod dj_diagnostics;
pub mod artist_artwork;
pub mod artwork;
pub mod enrichment;
pub mod events;
pub mod library;
pub mod playback;
pub mod playlists;
pub mod smart_playlists;
pub mod source_audit;

pub use enrichment::{
    accept_metadata_review, dismiss_metadata_review, enrich_library, load_metadata_review_items,
    EnrichmentProgress, EnrichmentRunReport, MetadataCandidate, MetadataReviewItem,
};
pub use library::{
    LibraryQuery, LibraryRepository, LocalLibraryRepository, RemoteLibraryRepository,
};
pub use playback::{
    LocalPlaybackClient, LocalPlaybackCommand, LocalPlaybackService, LocalPlaybackSnapshot,
};
pub use source_audit::{
    cleanup_hidden_records, open_and_audit, HiddenRootSummary, HiddenTrackRecord,
    SourceAuditReport, SourceCleanupReport, TrustedRoot, TrustedSourceSet,
};

