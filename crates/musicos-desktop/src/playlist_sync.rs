use serde_json::{json, Value};

use crate::models::{PlaylistConflict, PlaylistManifest, PlaylistSyncItem, PlaylistSyncPlan};

pub fn build_plan(
    desktop: &[PlaylistManifest],
    device: &[PlaylistManifest],
    snapshots: &[PlaylistManifest],
) -> PlaylistSyncPlan {
    let mut items = Vec::new();
    let mut consumed = vec![false; device.len()];
    for desktop_playlist in desktop {
        let match_index = device.iter().position(|candidate| {
            (!desktop_playlist.stable_id.is_empty()
                && desktop_playlist.stable_id == candidate.stable_id)
                || desktop_playlist.name.eq_ignore_ascii_case(&candidate.name)
        });
        let device_playlist = match match_index {
            Some(index) => {
                consumed[index] = true;
                device[index].clone()
            }
            None => empty_peer(desktop_playlist),
        };
        items.push(reconcile_item(
            desktop_playlist.clone(),
            device_playlist,
            snapshots,
        ));
    }
    for (index, device_playlist) in device.iter().enumerate() {
        if !consumed[index] {
            items.push(reconcile_item(
                empty_peer(device_playlist),
                device_playlist.clone(),
                snapshots,
            ));
        }
    }
    let conflicts = items
        .iter()
        .flat_map(|item| item.conflicts.iter().cloned())
        .collect();
    PlaylistSyncPlan {
        generated_at: timestamp(),
        items,
        conflicts,
    }
}

fn reconcile_item(
    desktop: PlaylistManifest,
    device: PlaylistManifest,
    snapshots: &[PlaylistManifest],
) -> PlaylistSyncItem {
    let key = if !desktop.stable_id.is_empty() {
        desktop.stable_id.clone()
    } else if !device.stable_id.is_empty() {
        device.stable_id.clone()
    } else if !desktop.name.is_empty() {
        desktop.name.clone()
    } else {
        device.name.clone()
    };
    let base = snapshots
        .iter()
        .find(|snapshot| snapshot.stable_id == key || snapshot.name.eq_ignore_ascii_case(&key))
        .cloned()
        .unwrap_or_else(|| {
            empty_peer(if desktop.name.is_empty() {
                &device
            } else {
                &desktop
            })
        });
    let mut conflicts = Vec::new();
    let name = choose(
        &key,
        display_name(&desktop, &device),
        "name",
        json!(base.name),
        json!(desktop.name),
        json!(device.name),
        &mut conflicts,
    )
    .as_str()
    .unwrap_or_default()
    .to_string();
    let track_ids = choose(
        &key,
        &name,
        "order",
        json!(base.track_ids),
        json!(desktop.track_ids),
        json!(device.track_ids),
        &mut conflicts,
    );
    let deleted = choose(
        &key,
        &name,
        "deleted",
        json!(base.deleted),
        json!(desktop.deleted),
        json!(device.deleted),
        &mut conflicts,
    );
    let merged = PlaylistManifest {
        stable_id: if desktop.stable_id.is_empty() {
            device.stable_id.clone()
        } else {
            desktop.stable_id.clone()
        },
        name,
        revision: desktop.revision.max(device.revision) + 1,
        track_ids: serde_json::from_value(track_ids).unwrap_or_default(),
        deleted: deleted.as_bool().unwrap_or(false),
    };
    PlaylistSyncItem {
        key,
        base,
        desktop,
        device,
        merged,
        conflicts,
    }
}

fn choose(
    key: &str,
    name: &str,
    field: &str,
    base: Value,
    desktop: Value,
    device: Value,
    conflicts: &mut Vec<PlaylistConflict>,
) -> Value {
    if desktop == device {
        return desktop;
    }
    if desktop == base {
        return device;
    }
    if device == base {
        return desktop;
    }
    conflicts.push(PlaylistConflict {
        key: key.to_string(),
        name: name.to_string(),
        field: field.to_string(),
        desktop: desktop.clone(),
        device,
    });
    desktop
}

fn display_name<'a>(desktop: &'a PlaylistManifest, device: &'a PlaylistManifest) -> &'a str {
    if desktop.name.is_empty() {
        &device.name
    } else {
        &desktop.name
    }
}

fn empty_peer(source: &PlaylistManifest) -> PlaylistManifest {
    PlaylistManifest {
        stable_id: source.stable_id.clone(),
        name: source.name.clone(),
        ..Default::default()
    }
}

fn timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playlist(name: &str, revision: i64, tracks: &[&str]) -> PlaylistManifest {
        PlaylistManifest {
            stable_id: format!("id-{name}"),
            name: name.to_string(),
            revision,
            track_ids: tracks.iter().map(|value| value.to_string()).collect(),
            deleted: false,
        }
    }

    #[test]
    fn takes_one_sided_changes_without_conflict() {
        let base = playlist("Commute", 1, &["a"]);
        let desktop = playlist("Commute", 2, &["a", "b"]);
        let plan = build_plan(&[desktop], &[base.clone()], &[base]);
        assert!(plan.conflicts.is_empty());
        assert_eq!(plan.items[0].merged.track_ids, ["a", "b"]);
    }

    #[test]
    fn reports_two_sided_order_conflict_and_defaults_to_desktop() {
        let base = playlist("Commute", 1, &["a"]);
        let desktop = playlist("Commute", 2, &["a", "b"]);
        let device = playlist("Commute", 2, &["a", "c"]);
        let plan = build_plan(&[desktop], &[device], &[base]);
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.conflicts[0].field, "order");
        assert_eq!(plan.items[0].merged.track_ids, ["a", "b"]);
    }
}
