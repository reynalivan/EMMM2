use std::collections::BTreeMap;
use std::path::Path;

use crate::common::normalizer::{is_disabled_folder, normalize_display_name};
use crate::repo::utils::stable_ids::generate_stable_id_from_key;

use super::disk_snapshot::{DiskIdentityCensus, DiskProjection};
use super::types::{FolderNameConflictCandidate, FolderNameConflictGroup};

fn path_is_enabled(path: &Path) -> bool {
    !path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(is_disabled_folder)
    })
}

pub fn detect_folder_name_conflicts(
    game_id: &str,
    projection: &DiskProjection,
) -> Vec<FolderNameConflictGroup> {
    let mut by_identity = BTreeMap::<String, Vec<FolderNameConflictCandidate>>::new();

    for disk_object in &projection.objects {
        by_identity
            .entry(disk_object.folder_path_key.clone())
            .or_default()
            .push(FolderNameConflictCandidate {
                path: disk_object.absolute_path.to_string_lossy().to_string(),
                folder_name: disk_object.folder_path.clone(),
                base_name: disk_object.name.clone(),
                is_enabled: !disk_object.is_disabled,
            });
    }

    for disk_mod in &projection.mods {
        by_identity
            .entry(disk_mod.folder_path_key.clone())
            .or_default()
            .push(FolderNameConflictCandidate {
                path: disk_mod.absolute_path.to_string_lossy().to_string(),
                folder_name: disk_mod.raw_name.clone(),
                base_name: normalize_display_name(&disk_mod.raw_name).into_owned(),
                is_enabled: path_is_enabled(&disk_mod.absolute_path),
            });
    }

    by_identity
        .into_iter()
        .filter_map(|(identity, mut candidates)| {
            if candidates.len() < 2 {
                return None;
            }
            candidates.sort_by_key(|candidate| candidate.path.to_ascii_lowercase());
            let display_name = candidates
                .first()
                .map(|candidate| candidate.base_name.clone())
                .unwrap_or_default();
            Some(FolderNameConflictGroup {
                group_id: generate_stable_id_from_key(game_id, &identity),
                identity,
                display_name,
                candidates,
            })
        })
        .collect()
}

/// The census is intentionally metadata-free. Its normalized path identities
/// are enough to construct the read-only conflict overlay before the scoped
/// classifier decides which changed roots must be projected.
pub fn detect_folder_name_conflicts_from_census(
    game_id: &str,
    census: &DiskIdentityCensus,
) -> Vec<FolderNameConflictGroup> {
    let mut by_identity = BTreeMap::<String, Vec<FolderNameConflictCandidate>>::new();

    for entry in &census.entries {
        by_identity
            .entry(entry.folder_path_key.clone())
            .or_default()
            .push(FolderNameConflictCandidate {
                path: entry.absolute_path.to_string_lossy().to_string(),
                folder_name: entry.raw_name.clone(),
                base_name: normalize_display_name(&entry.raw_name).into_owned(),
                is_enabled: path_is_enabled(&entry.absolute_path),
            });
    }

    by_identity
        .into_iter()
        .filter_map(|(identity, mut candidates)| {
            if candidates.len() < 2 {
                return None;
            }
            candidates.sort_by_key(|candidate| candidate.path.to_ascii_lowercase());
            let display_name = candidates
                .first()
                .map(|candidate| candidate.base_name.clone())
                .unwrap_or_default();
            Some(FolderNameConflictGroup {
                group_id: generate_stable_id_from_key(game_id, &identity),
                identity,
                display_name,
                candidates,
            })
        })
        .collect()
}
