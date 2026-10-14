use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

pub struct ReferenceSourceWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<notify::Result<Event>>,
    watched: BTreeMap<PathBuf, bool>,
    last_event: Option<Instant>,
    changed_roots: BTreeMap<PathBuf, ()>,
}

impl ReferenceSourceWatcher {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let watcher = notify::recommended_watcher(move |event| {
            let _ = sender.send(event);
        })?;
        Ok(Self {
            watcher,
            receiver,
            watched: BTreeMap::new(),
            last_event: None,
            changed_roots: BTreeMap::new(),
        })
    }

    pub fn replace_roots(&mut self, roots: impl IntoIterator<Item = PathBuf>) -> Result<()> {
        let next: BTreeMap<PathBuf, bool> = roots
            .into_iter()
            .map(|path| {
                let available = path.is_dir();
                (path, available)
            })
            .collect();
        for (root, was_available) in &self.watched {
            let remains_available = next.get(root).copied().unwrap_or(false);
            if *was_available && !remains_available {
                let _ = self.watcher.unwatch(root);
            }
        }
        for (root, available) in &next {
            let was_available = self.watched.get(root).copied().unwrap_or(false);
            if *available && !was_available {
                self.watcher.watch(root, RecursiveMode::Recursive)?;
            }
        }
        self.watched = next;
        Ok(())
    }

    pub fn availability(&self, path: &Path) -> bool {
        self.watched
            .get(path)
            .copied()
            .unwrap_or_else(|| path.is_dir())
    }

    pub fn changed_after_debounce(&mut self, debounce: Duration) -> bool {
        !self.changed_roots_after_debounce(debounce).is_empty()
    }

    pub fn changed_roots_after_debounce(&mut self, debounce: Duration) -> Vec<PathBuf> {
        while let Ok(event) = self.receiver.try_recv() {
            let Ok(event) = event else {
                continue;
            };
            for path in event.paths {
                if let Some(root) = self
                    .watched
                    .iter()
                    .filter(|(root, available)| **available && path.starts_with(root))
                    .map(|(root, _)| root)
                    .max_by_key(|root| root.components().count())
                {
                    self.changed_roots.insert(root.clone(), ());
                }
            }
            self.last_event = Some(Instant::now());
        }
        if !self
            .last_event
            .is_some_and(|instant| instant.elapsed() >= debounce)
        {
            return Vec::new();
        }
        self.last_event = None;
        std::mem::take(&mut self.changed_roots)
            .into_keys()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_offline_and_reconnected_reference_roots() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("removable-music");
        let mut watcher = ReferenceSourceWatcher::new().unwrap();

        watcher.replace_roots([root.clone()]).unwrap();
        assert!(!watcher.availability(&root));

        std::fs::create_dir(&root).unwrap();
        watcher.replace_roots([root.clone()]).unwrap();
        assert!(watcher.availability(&root));

        std::fs::remove_dir(&root).unwrap();
        watcher.replace_roots([root.clone()]).unwrap();
        assert!(!watcher.availability(&root));
    }
}
