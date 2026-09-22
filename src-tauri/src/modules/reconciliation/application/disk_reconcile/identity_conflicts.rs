use std::collections::BTreeMap;
use std::path::Path;

use crate::modules::system::adapters::sqlite::utils::stable_ids::generate_stable_id_from_key;
use crate::modules::workspace::domain::normalizer::{is_disabled_folder, normalize_display_name};

use super::disk_snapshot::{DiskIdentityCensus, DiskProjection};
use super::types::{FolderNameConflictCandidate, FolderNameConflictGroup};

/// A name conflict is actionable only when both folders are siblings. The
/// parent path stays physical so a `DISABLED` prefix on an ancestor creates a
/// separate root rather than merging two unrelated folder trees.
fn folder_conflict_identity(folder_path: &str) -> Option<String> {
    let path = Path::new(folder_path);
    let parent = path.parent().unwrap_or_else(|| Path::new(""));

    let parent_key = parent
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_string_lossy()
                .trim()
                .to_ascii_lowercase()
        })
        .collect::<Vec<_>>()
        .join("/");
    let base_name = path.file_name()?.to_string_lossy();
    let name_key = normalize_display_name(&base_name).to_ascii_lowercase();

    if parent_key.is_empty() {
        Some(name_key.to_string())
    } else {
        Some(format!("{parent_key}/{name_key}"))
    }
}

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
        let Some(identity) = folder_conflict_identity(&disk_object.folder_path) else {
            continue;
        };
        by_identity
            .entry(identity)
            .or_default()
            .push(FolderNameConflictCandidate {
                path: disk_object.absolute_path.to_string_lossy().to_string(),
                folder_name: disk_object.folder_path.clone(),
                base_name: disk_object.name.clone(),
                is_enabled: !disk_object.is_disabled,
            });
    }

    for disk_mod in &projection.mods {
        let Some(identity) = folder_conflict_identity(&disk_mod.folder_path) else {
            continue;
        };
        by_identity
            .entry(identity)
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
        let Some(identity) = folder_conflict_identity(&entry.folder_path) else {
            continue;
        };
        by_identity
            .entry(identity)
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::detect_folder_name_conflicts_from_census;
    use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::{
        DiskIdentityCensus, DiskIdentityCensusEntry,
    };
    use crate::shared::path_key::folder_path_key;

    fn census_entry(path: &str) -> DiskIdentityCensusEntry {
        DiskIdentityCensusEntry {
            folder_path: path.to_string(),
            folder_path_key: folder_path_key(path, None),
            raw_name: path.rsplit('/').next().unwrap_or_default().to_string(),
            absolute_path: PathBuf::from(path),
        }
    }

    #[test]
    fn reports_normalized_names_with_the_same_physical_parent_including_mods_root() {
        let census = DiskIdentityCensus {
            entries: vec![
                census_entry("Alice"),
                census_entry("DISABLED Alice"),
                census_entry("Alice/Blue"),
                census_entry("Alice/DISABLED Blue"),
                census_entry("DISABLED Alice/Blue"),
            ],
            top_level_roots: 2,
        };

        let groups = detect_folder_name_conflicts_from_census("game", &census);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].identity, "alice");
        assert_eq!(
            groups[0]
                .candidates
                .iter()
                .map(|candidate| candidate.path.as_str())
                .collect::<Vec<_>>(),
            vec!["Alice", "DISABLED Alice"],
        );
        assert_eq!(groups[1].identity, "alice/blue");
        assert_eq!(
            groups[1]
                .candidates
                .iter()
                .map(|candidate| candidate.path.as_str())
                .collect::<Vec<_>>(),
            vec!["Alice/Blue", "Alice/DISABLED Blue"],
        );
    }
}
