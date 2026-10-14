use crate::models::{LibrarySnapshot, TaskProgress};

#[derive(Debug, Clone)]
pub enum LocalEvent {
    LibraryLoaded(LibrarySnapshot),
    Progress {
        title: String,
        detail: Option<String>,
        progress: TaskProgress,
    },
    Notice(String),
    Error(String),
}
