use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::modules::catalog::adapters::sqlite::object::ReconcileObjectRow;
use crate::modules::library::adapters::sqlite::mods::ReconcileModRow;
use crate::modules::system::adapters::sqlite::utils::stable_ids::generate_stable_id_from_key;
use crate::modules::workspace::application::scanner::watcher::ModWatchEvent;

use super::disk_snapshot::DiskProjection;
use super::types::{RenameConfirmationGroup, RenameConfirmationKind, RenameConfirmationReason};
use super::watcher_batch::collect_rename_hints;

const OBJECT_TOP_LEVEL_SCOPE: &str = "__top_level__";
const MAX_RENAME_PATHS_PER_SIDE: usize = 16;
const MAX_RENAME_CANDIDATE_PAIRS: usize = 64;

#[derive(Clone)]
struct RuntimeEntry<'a> {
    path: &'a str,
    logical_key: String,
    protected_key: &'a str,
    scope_key: String,
    filesystem_identity: Option<&'a str>,
}

#[derive(Default)]
struct KindDetection {
    groups: Vec<RenameConfirmationGroup>,
    protected_keys: HashSet<String>,
}

pub(crate) struct RenameConfirmationDetection {
    pub(crate) groups: Vec<RenameConfirmationGroup>,
    pub(crate) protected_object_keys: HashSet<String>,
    pub(crate) protected_mod_keys: HashSet<String>,
}

fn scope_key(kind: &RenameConfirmationKind, path: &str) -> String {
    match kind {
        RenameConfirmationKind::Object => OBJECT_TOP_LEVEL_SCOPE.to_string(),
        RenameConfirmationKind::Mod => path
            .split(['/', '\\'])
            .find(|segment| !segment.is_empty())
            .map(|segment| crate::shared::path_key::folder_path_key(segment, None))
            .unwrap_or_default(),
    }
}

fn build_group(
    game_id: &str,
    kind: RenameConfirmationKind,
    reason: RenameConfirmationReason,
    scope_key: &str,
    mut previous_paths: Vec<String>,
    mut current_paths: Vec<String>,
    force_truncated: bool,
) -> RenameConfirmationGroup {
    previous_paths.sort_by_key(|path| path.to_ascii_lowercase());
    previous_paths.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    current_paths.sort_by_key(|path| path.to_ascii_lowercase());
    current_paths.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    let previous_path_count = previous_paths.len();
    let current_path_count = current_paths.len();
    let signature = format!(
        "rename-confirmation:{kind:?}:{reason:?}:{scope_key}:{}:{}",
        previous_paths.join("\u{0}"),
        current_paths.join("\u{0}")
    );
    previous_paths.truncate(MAX_RENAME_PATHS_PER_SIDE);
    current_paths.truncate(MAX_RENAME_PATHS_PER_SIDE);
    let candidates_truncated = force_truncated
        || previous_paths.len() != previous_path_count
        || current_paths.len() != current_path_count;

    RenameConfirmationGroup {
        group_id: generate_stable_id_from_key(game_id, &signature),
        kind,
        reason,
        scope_key: scope_key.to_string(),
        previous_paths,
        current_paths,
        previous_path_count,
        current_path_count,
        candidates_truncated,
    }
}

fn build_candidate_groups(
    game_id: &str,
    kind: RenameConfirmationKind,
    reason: RenameConfirmationReason,
    scope_key: &str,
    previous_paths: Vec<String>,
    current_paths: Vec<String>,
) -> Vec<RenameConfirmationGroup> {
    let candidate_pairs = previous_paths.len().saturating_mul(current_paths.len());
    let oversized = previous_paths.len() > MAX_RENAME_PATHS_PER_SIDE
        || current_paths.len() > MAX_RENAME_PATHS_PER_SIDE
        || candidate_pairs > MAX_RENAME_CANDIDATE_PAIRS;
    if oversized {
        return vec![build_group(
            game_id,
            kind,
            reason,
            scope_key,
            previous_paths,
            current_paths,
            true,
        )];
    }

    current_paths
        .into_iter()
        .map(|current_path| {
            build_group(
                game_id,
                kind.clone(),
                reason.clone(),
                scope_key,
                previous_paths.clone(),
                vec![current_path],
                false,
            )
        })
        .collect()
}

fn detect_kind(
    game_id: &str,
    kind: RenameConfirmationKind,
    previous: &[RuntimeEntry<'_>],
    current: &[RuntimeEntry<'_>],
) -> KindDetection {
    let mut previous_matched = vec![false; previous.len()];
    let mut current_matched = vec![false; current.len()];
    let mut previous_by_identity = BTreeMap::<&str, Vec<usize>>::new();
    let mut current_by_identity = BTreeMap::<&str, Vec<usize>>::new();
    for (index, entry) in previous.iter().enumerate() {
        if let Some(identity) = entry.filesystem_identity {
            previous_by_identity
                .entry(identity)
                .or_default()
                .push(index);
        }
    }
    for (index, entry) in current.iter().enumerate() {
        if let Some(identity) = entry.filesystem_identity {
            current_by_identity.entry(identity).or_default().push(index);
        }
    }

    let mut detection = KindDetection::default();
    let shared_identities = previous_by_identity
        .keys()
        .filter(|identity| current_by_identity.contains_key(**identity))
        .copied()
        .collect::<BTreeSet<_>>();
    for identity in shared_identities {
        let previous_indexes = &previous_by_identity[identity];
        let current_indexes = &current_by_identity[identity];
        if previous_indexes.len() == 1 && current_indexes.len() == 1 {
            previous_matched[previous_indexes[0]] = true;
            current_matched[current_indexes[0]] = true;
            continue;
        }

        for index in previous_indexes {
            previous_matched[*index] = true;
        }
        for index in current_indexes {
            current_matched[*index] = true;
        }
        let previous_paths = previous_indexes
            .iter()
            .map(|index| previous[*index].path.to_string())
            .collect::<Vec<_>>();
        detection.protected_keys.extend(
            previous_indexes
                .iter()
                .map(|index| previous[*index].protected_key.to_string()),
        );
        detection.protected_keys.extend(
            current_indexes
                .iter()
                .map(|index| current[*index].protected_key.to_string()),
        );
        let mut current_by_scope = BTreeMap::<String, Vec<String>>::new();
        for current_index in current_indexes {
            current_by_scope
                .entry(current[*current_index].scope_key.clone())
                .or_default()
                .push(current[*current_index].path.to_string());
        }
        for (scope_key, current_paths) in current_by_scope {
            detection.groups.extend(build_candidate_groups(
                game_id,
                kind.clone(),
                RenameConfirmationReason::AmbiguousIdentity,
                &scope_key,
                previous_paths.clone(),
                current_paths,
            ));
        }
    }

    // Filesystem identity is stronger evidence than a path key. This ordering
    // is what makes an offline A -> B replacement, and an A/B swap, follow the
    // physical folders instead of attaching the disk entry to the stale row
    // that previously occupied the destination path.
    let previous_by_key = previous.iter().enumerate().fold(
        BTreeMap::<String, Vec<usize>>::new(),
        |mut map, (index, entry)| {
            if !previous_matched[index] {
                map.entry(entry.logical_key.clone())
                    .or_default()
                    .push(index);
            }
            map
        },
    );
    for (current_index, entry) in current.iter().enumerate() {
        if current_matched[current_index] {
            continue;
        }
        let Some(matches) = previous_by_key.get(&entry.logical_key) else {
            continue;
        };
        if matches.len() == 1 {
            previous_matched[matches[0]] = true;
            current_matched[current_index] = true;
        }
    }

    let unmatched_previous = previous
        .iter()
        .enumerate()
        .filter(|(index, _)| !previous_matched[*index])
        .collect::<Vec<_>>();
    let unmatched_current = current
        .iter()
        .enumerate()
        .filter(|(index, _)| !current_matched[*index])
        .collect::<Vec<_>>();
    let mut previous_by_scope = BTreeMap::<&str, Vec<&RuntimeEntry<'_>>>::new();
    for (_, entry) in unmatched_previous {
        previous_by_scope
            .entry(entry.scope_key.as_str())
            .or_default()
            .push(entry);
    }
    let mut current_by_scope = BTreeMap::<&str, Vec<&RuntimeEntry<'_>>>::new();
    for (_, entry) in unmatched_current {
        current_by_scope
            .entry(entry.scope_key.as_str())
            .or_default()
            .push(entry);
    }
    for (scope_key, previous_entries) in previous_by_scope {
        let Some(current_entries) = current_by_scope.get(scope_key) else {
            continue;
        };
        let identity_missing = previous_entries
            .iter()
            .any(|entry| entry.filesystem_identity.is_none())
            || current_entries
                .iter()
                .any(|entry| entry.filesystem_identity.is_none());
        if !identity_missing {
            continue;
        }
        detection.protected_keys.extend(
            previous_entries
                .iter()
                .map(|entry| entry.protected_key.to_string()),
        );
        detection.protected_keys.extend(
            current_entries
                .iter()
                .map(|entry| entry.protected_key.to_string()),
        );
        detection.groups.extend(build_candidate_groups(
            game_id,
            kind.clone(),
            RenameConfirmationReason::MissingIdentity,
            scope_key,
            previous_entries
                .iter()
                .map(|entry| entry.path.to_string())
                .collect(),
            current_entries
                .iter()
                .map(|entry| entry.path.to_string())
                .collect(),
        ));
    }

    detection
}

#[cfg(test)]
fn detect_rename_confirmations_from_rows(
    game_id: &str,
    projection: &DiskProjection,
    objects: &[ReconcileObjectRow],
    mods: &[ReconcileModRow],
) -> Vec<RenameConfirmationGroup> {
    detect_rename_confirmations_with_protection_from_rows(game_id, projection, objects, mods).groups
}

fn detect_rename_confirmations_with_protection_from_rows(
    game_id: &str,
    projection: &DiskProjection,
    objects: &[ReconcileObjectRow],
    mods: &[ReconcileModRow],
) -> RenameConfirmationDetection {
    let object_kind = RenameConfirmationKind::Object;
    let previous_objects = objects
        .iter()
        .map(|entry| RuntimeEntry {
            path: &entry.folder_path,
            logical_key: crate::shared::path_key::folder_path_key(&entry.folder_path, None),
            protected_key: &entry.folder_path_key,
            scope_key: scope_key(&object_kind, &entry.folder_path),
            filesystem_identity: entry.filesystem_identity.as_deref(),
        })
        .collect::<Vec<_>>();
    let current_objects = projection
        .objects
        .iter()
        .map(|entry| RuntimeEntry {
            path: &entry.folder_path,
            logical_key: crate::shared::path_key::folder_path_key(&entry.folder_path, None),
            protected_key: &entry.folder_path_key,
            scope_key: scope_key(&object_kind, &entry.folder_path),
            filesystem_identity: entry.filesystem_identity.as_deref(),
        })
        .collect::<Vec<_>>();
    let mod_kind = RenameConfirmationKind::Mod;
    let previous_mods = mods
        .iter()
        .map(|entry| RuntimeEntry {
            path: &entry.folder_path,
            logical_key: crate::shared::path_key::folder_path_key(&entry.folder_path, None),
            protected_key: &entry.folder_path_key,
            scope_key: scope_key(&mod_kind, &entry.folder_path),
            filesystem_identity: entry.filesystem_identity.as_deref(),
        })
        .collect::<Vec<_>>();
    let current_mods = projection
        .mods
        .iter()
        .map(|entry| RuntimeEntry {
            path: &entry.folder_path,
            logical_key: crate::shared::path_key::folder_path_key(&entry.folder_path, None),
            protected_key: &entry.folder_path_key,
            scope_key: scope_key(&mod_kind, &entry.folder_path),
            filesystem_identity: entry.filesystem_identity.as_deref(),
        })
        .collect::<Vec<_>>();

    let objects = detect_kind(
        game_id,
        RenameConfirmationKind::Object,
        &previous_objects,
        &current_objects,
    );
    let mods = detect_kind(
        game_id,
        RenameConfirmationKind::Mod,
        &previous_mods,
        &current_mods,
    );
    let mut groups = objects.groups;
    groups.extend(mods.groups);
    groups.sort_by(|left, right| left.group_id.cmp(&right.group_id));
    RenameConfirmationDetection {
        groups,
        protected_object_keys: objects.protected_keys,
        protected_mod_keys: mods.protected_keys,
    }
}

fn path_is_covered(path: &str, evidence_root: &str) -> bool {
    path.eq_ignore_ascii_case(evidence_root)
        || crate::shared::path_key::strip_path_prefix_preserve_display(path, evidence_root, None)
            .is_some()
}

fn without_watcher_rename_evidence(
    mods_path: &std::path::Path,
    projection: &DiskProjection,
    objects: &[ReconcileObjectRow],
    mods: &[ReconcileModRow],
    watcher_events: &[ModWatchEvent],
) -> (
    DiskProjection,
    Vec<ReconcileObjectRow>,
    Vec<ReconcileModRow>,
) {
    let actual_rename_events = watcher_events
        .iter()
        .filter(|event| matches!(event, ModWatchEvent::Renamed { .. }))
        .cloned()
        .collect::<Vec<_>>();
    let hints = collect_rename_hints(mods_path, &actual_rename_events);
    let object_from = hints
        .object_renames
        .iter()
        .map(|(from, _)| from)
        .collect::<Vec<_>>();
    let object_to = hints
        .object_renames
        .iter()
        .map(|(_, to)| to)
        .collect::<Vec<_>>();
    let mod_from = hints
        .mod_renames
        .iter()
        .map(|(from, _)| from)
        .collect::<Vec<_>>();
    let mod_to = hints
        .mod_renames
        .iter()
        .map(|(_, to)| to)
        .collect::<Vec<_>>();

    let filtered_projection = DiskProjection {
        objects: projection
            .objects
            .iter()
            .filter(|entry| {
                !object_to
                    .iter()
                    .any(|root| path_is_covered(&entry.folder_path, root))
            })
            .cloned()
            .collect(),
        mods: projection
            .mods
            .iter()
            .filter(|entry| {
                !object_to
                    .iter()
                    .chain(mod_to.iter())
                    .any(|root| path_is_covered(&entry.folder_path, root))
            })
            .cloned()
            .collect(),
    };
    let filtered_objects = objects
        .iter()
        .filter(|entry| {
            !object_from
                .iter()
                .any(|root| path_is_covered(&entry.folder_path, root))
        })
        .cloned()
        .collect();
    let filtered_mods = mods
        .iter()
        .filter(|entry| {
            !object_from
                .iter()
                .chain(mod_from.iter())
                .any(|root| path_is_covered(&entry.folder_path, root))
        })
        .cloned()
        .collect();

    (filtered_projection, filtered_objects, filtered_mods)
}

fn validate_explicit_resolutions(
    groups: &[RenameConfirmationGroup],
    mods_path: &std::path::Path,
    watcher_events: &[ModWatchEvent],
) -> Result<bool, crate::shared::errors::AppError> {
    let resolutions = watcher_events
        .iter()
        .filter_map(|event| match event {
            ModWatchEvent::RenameResolution {
                group_id,
                from,
                to,
                apply_as_rename,
            } => Some((group_id, from, to, *apply_as_rename)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if resolutions.is_empty() {
        return Ok(false);
    }
    if resolutions.len() != groups.len() {
        return Err(crate::shared::errors::AppError::Validation(
            "Rename confirmation report changed; review every current group again".to_string(),
        ));
    }

    let mut seen = BTreeSet::new();
    let mut renamed_previous_paths = BTreeSet::new();
    for (group_id, from, to, apply_as_rename) in resolutions {
        if !seen.insert(group_id.as_str()) {
            return Err(crate::shared::errors::AppError::Validation(
                "Duplicate rename confirmation group".to_string(),
            ));
        }
        let group = groups
            .iter()
            .find(|group| group.group_id == *group_id)
            .ok_or_else(|| {
                crate::shared::errors::AppError::Validation(
                    "Rename confirmation report is stale; refresh and try again".to_string(),
                )
            })?;
        if !apply_as_rename {
            continue;
        }
        if group.candidates_truncated {
            return Err(crate::shared::errors::AppError::Validation(
                "Rename candidates exceed the safe review limit; keep them separate or reduce the scope"
                    .to_string(),
            ));
        }
        let (Some(from), Some(to)) = (from.as_deref(), to.as_deref()) else {
            return Err(crate::shared::errors::AppError::Validation(
                "Confirmed rename requires both the previous and current path".to_string(),
            ));
        };
        let from = std::path::Path::new(from)
            .strip_prefix(mods_path)
            .unwrap_or_else(|_| std::path::Path::new(from))
            .to_string_lossy()
            .to_string();
        let to = std::path::Path::new(to)
            .strip_prefix(mods_path)
            .unwrap_or_else(|_| std::path::Path::new(to))
            .to_string_lossy()
            .to_string();
        if !group
            .previous_paths
            .iter()
            .any(|path| path.eq_ignore_ascii_case(&from))
            || !group
                .current_paths
                .iter()
                .any(|path| path.eq_ignore_ascii_case(&to))
        {
            return Err(crate::shared::errors::AppError::Validation(
                "Confirmed rename paths are not part of the current report".to_string(),
            ));
        }
        if !renamed_previous_paths.insert(from.to_ascii_lowercase()) {
            return Err(crate::shared::errors::AppError::Validation(
                "The same previous folder cannot be assigned to more than one current folder"
                    .to_string(),
            ));
        }
    }
    Ok(true)
}

pub(crate) async fn detect_rename_confirmations(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &std::path::Path,
    projection: &DiskProjection,
    watcher_events: &[ModWatchEvent],
) -> Result<RenameConfirmationDetection, crate::shared::errors::AppError> {
    let mut conn = pool.acquire().await?;
    let objects = crate::modules::catalog::adapters::sqlite::object::get_rows_for_reconcile(
        &mut conn, game_id,
    )
    .await?;
    let mods =
        crate::modules::library::adapters::sqlite::mods::get_rows_for_reconcile(&mut conn, game_id)
            .await?;
    drop(conn);
    let (filtered_projection, filtered_objects, filtered_mods) =
        without_watcher_rename_evidence(mods_path, projection, &objects, &mods, watcher_events);
    let mut detection = detect_rename_confirmations_with_protection_from_rows(
        game_id,
        &filtered_projection,
        &filtered_objects,
        &filtered_mods,
    );
    if validate_explicit_resolutions(&detection.groups, mods_path, watcher_events)? {
        detection.groups.clear();
        detection.protected_object_keys.clear();
        detection.protected_mod_keys.clear();
    }
    Ok(detection)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::modules::games::domain::models::ItemStatus;
    use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::{
        DiskModEntry, DiskObjectEntry,
    };
    use crate::modules::reconciliation::application::disk_reconcile::types::{
        RenameConfirmationKind, RenameConfirmationReason,
    };

    use super::*;

    fn object(path: &str, identity: Option<&str>) -> ReconcileObjectRow {
        ReconcileObjectRow {
            id: format!("object-{path}"),
            name: path.to_string(),
            folder_path: path.to_string(),
            folder_path_key: crate::shared::path_key::folder_path_key(path, None),
            status: ItemStatus::Enabled,
            filesystem_identity: identity.map(str::to_string),
        }
    }

    fn disk_object(path: &str, identity: Option<&str>) -> DiskObjectEntry {
        DiskObjectEntry {
            folder_path: path.to_string(),
            folder_path_key: crate::shared::path_key::folder_path_key(path, None),
            name: path.to_string(),
            is_disabled: false,
            absolute_path: PathBuf::from("E:/Mods").join(path),
            filesystem_identity: identity.map(str::to_string),
        }
    }

    fn disk_mod(path: &str, identity: Option<&str>) -> DiskModEntry {
        let object_key = PathBuf::from(path)
            .components()
            .next()
            .map(|part| {
                crate::shared::path_key::folder_path_key(&part.as_os_str().to_string_lossy(), None)
            })
            .unwrap();
        DiskModEntry {
            folder_path: path.to_string(),
            folder_path_key: crate::shared::path_key::folder_path_key(path, None),
            object_folder_path_key: object_key,
            raw_name: PathBuf::from(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            absolute_path: PathBuf::from("E:/Mods").join(path),
            filesystem_identity: identity.map(str::to_string),
            size_bytes: None,
        }
    }

    fn db_mod(path: &str, identity: Option<&str>) -> ReconcileModRow {
        ReconcileModRow {
            id: format!("mod-{path}"),
            folder_path: path.to_string(),
            folder_path_key: crate::shared::path_key::folder_path_key(path, None),
            actual_name: PathBuf::from(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            status: ItemStatus::Enabled,
            object_id: Some("object-alice".to_string()),
            is_safe: false,
            safety_source: None,
            object_type: Some("Other".to_string()),
            filesystem_identity: identity.map(str::to_string),
            size_bytes: 0,
        }
    }

    #[test]
    fn unique_filesystem_identity_is_evidence_not_a_confirmation() {
        let projection = DiskProjection {
            objects: vec![disk_object("Alicia", Some("object-fs"))],
            mods: vec![disk_mod("Alicia/New", Some("mod-fs"))],
        };
        let groups = detect_rename_confirmations_from_rows(
            "game",
            &projection,
            &[object("Alice", Some("object-fs"))],
            &[db_mod("Alice/Old", Some("mod-fs"))],
        );
        assert!(groups.is_empty());
    }

    #[test]
    fn missing_identity_one_to_one_is_reported_without_guessing() {
        let projection = DiskProjection {
            objects: vec![disk_object("Alice", Some("object-fs"))],
            mods: vec![disk_mod("Alice/New", None)],
        };
        let groups = detect_rename_confirmations_from_rows(
            "game",
            &projection,
            &[object("Alice", Some("object-fs"))],
            &[db_mod("Alice/Old", None)],
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].kind, RenameConfirmationKind::Mod);
        assert_eq!(groups[0].reason, RenameConfirmationReason::MissingIdentity);
        assert_eq!(groups[0].previous_paths, ["Alice/Old"]);
        assert_eq!(groups[0].current_paths, ["Alice/New"]);
    }

    #[test]
    fn ambiguous_filesystem_identity_exposes_all_previous_candidates() {
        let projection = DiskProjection {
            objects: vec![disk_object("Alice", Some("object-fs"))],
            mods: vec![disk_mod("Alice/New", Some("duplicate-fs"))],
        };
        let groups = detect_rename_confirmations_from_rows(
            "game",
            &projection,
            &[object("Alice", Some("object-fs"))],
            &[
                db_mod("Alice/Old One", Some("duplicate-fs")),
                db_mod("Alice/Old Two", Some("duplicate-fs")),
            ],
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].reason,
            RenameConfirmationReason::AmbiguousIdentity
        );
        assert_eq!(groups[0].previous_paths.len(), 2);
        assert_eq!(groups[0].current_paths, ["Alice/New"]);
    }

    #[test]
    fn unrelated_add_or_delete_does_not_require_confirmation() {
        let added = DiskProjection {
            objects: vec![disk_object("Alice", Some("object-fs"))],
            mods: vec![disk_mod("Alice/New", None)],
        };
        assert!(detect_rename_confirmations_from_rows(
            "game",
            &added,
            &[object("Alice", Some("object-fs"))],
            &[],
        )
        .is_empty());

        let deleted = DiskProjection {
            objects: vec![disk_object("Alice", Some("object-fs"))],
            mods: vec![],
        };
        assert!(detect_rename_confirmations_from_rows(
            "game",
            &deleted,
            &[object("Alice", Some("object-fs"))],
            &[db_mod("Alice/Old", None)],
        )
        .is_empty());
    }

    #[test]
    fn multiple_missing_identity_candidates_create_one_decision_per_current_path() {
        let projection = DiskProjection {
            objects: vec![disk_object("Alice", Some("object-fs"))],
            mods: vec![
                disk_mod("Alice/New One", None),
                disk_mod("Alice/New Two", None),
            ],
        };
        let groups = detect_rename_confirmations_from_rows(
            "game",
            &projection,
            &[object("Alice", Some("object-fs"))],
            &[db_mod("Alice/Old One", None), db_mod("Alice/Old Two", None)],
        );

        assert_eq!(groups.len(), 2);
        assert!(groups.iter().all(|group| group.previous_paths.len() == 2));
        assert!(groups.iter().all(|group| group.current_paths.len() == 1));
    }

    #[test]
    fn missing_identity_candidates_are_partitioned_by_nearest_object_scope() {
        let projection = DiskProjection {
            objects: vec![
                disk_object("Alice", Some("object-alice")),
                disk_object("Bob", Some("object-bob")),
                disk_object("Charlie", Some("object-charlie")),
            ],
            mods: vec![disk_mod("Alice/New", None), disk_mod("Charlie/Fresh", None)],
        };
        let groups = detect_rename_confirmations_from_rows(
            "game",
            &projection,
            &[
                object("Alice", Some("object-alice")),
                object("Bob", Some("object-bob")),
            ],
            &[db_mod("Alice/Old", None), db_mod("Bob/Removed", None)],
        );

        assert_eq!(groups.len(), 1, "add/delete-only scopes are not renames");
        assert_eq!(groups[0].previous_paths, ["Alice/Old"]);
        assert_eq!(groups[0].current_paths, ["Alice/New"]);
        let payload = serde_json::to_value(&groups[0]).unwrap();
        assert_eq!(payload["scope_key"], "alice");
        assert_eq!(payload["candidates_truncated"], false);
    }

    #[test]
    fn oversized_missing_identity_scope_has_one_bounded_explicit_payload() {
        let previous = (0..100)
            .map(|index| db_mod(&format!("Alice/Old {index:03}"), None))
            .collect::<Vec<_>>();
        let projection = DiskProjection {
            objects: vec![disk_object("Alice", Some("object-alice"))],
            mods: (0..100)
                .map(|index| disk_mod(&format!("Alice/New {index:03}"), None))
                .collect(),
        };

        let groups = detect_rename_confirmations_from_rows(
            "game",
            &projection,
            &[object("Alice", Some("object-alice"))],
            &previous,
        );

        assert_eq!(groups.len(), 1, "oversized ambiguity must not expand N*M");
        assert!(groups[0].previous_paths.len() <= 16);
        assert!(groups[0].current_paths.len() <= 16);
        let payload = serde_json::to_value(&groups[0]).unwrap();
        assert_eq!(payload["scope_key"], "alice");
        assert_eq!(payload["previous_path_count"], 100);
        assert_eq!(payload["current_path_count"], 100);
        assert_eq!(payload["candidates_truncated"], true);
    }

    #[test]
    fn truncated_candidate_payload_cannot_be_applied_as_a_guessed_rename() {
        let previous = (0..100)
            .map(|index| db_mod(&format!("Alice/Old {index:03}"), None))
            .collect::<Vec<_>>();
        let projection = DiskProjection {
            objects: vec![disk_object("Alice", Some("object-alice"))],
            mods: (0..100)
                .map(|index| disk_mod(&format!("Alice/New {index:03}"), None))
                .collect(),
        };
        let groups = detect_rename_confirmations_from_rows(
            "game",
            &projection,
            &[object("Alice", Some("object-alice"))],
            &previous,
        );
        let resolution = ModWatchEvent::RenameResolution {
            group_id: groups[0].group_id.clone(),
            from: Some("Alice/Old 000".to_string()),
            to: Some("Alice/New 000".to_string()),
            apply_as_rename: true,
        };

        let error = validate_explicit_resolutions(
            &groups,
            std::path::Path::new("E:/Mods"),
            std::slice::from_ref(&resolution),
        )
        .expect_err("a truncated candidate set cannot safely choose a rename");

        assert!(error.to_string().contains("safe review limit"), "{error}");
    }
}
