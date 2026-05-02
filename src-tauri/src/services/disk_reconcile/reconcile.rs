//! Disk Reconcile keeps the runtime projection aligned with filesystem reality.
//! Do not add MasterDB matching logic here.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use sqlx::FromRow;

use crate::database::models::ItemStatus;
use crate::repo::object_repo::ObjectRuntimeDescriptor;
use crate::services::corridor_constants::{CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN};
use crate::services::disk_reconcile::helpers::{
    generate_stable_mod_id, is_disabled_runtime_name, load_runtime_mod_metadata,
    normalize_runtime_name,
};
use crate::services::disk_reconcile::path_classifier::{
    collect_changed_roots, collect_thumbnail_roots, is_runtime_relevant_file,
};
use crate::services::disk_reconcile::types::{
    DiskReconcileChangeCounts, DiskReconcileChangeSummary, DiskReconcilePathKind,
    DiskReconcilePathUpdate, DiskReconcileReason, DiskReconcileStatus,
};
use crate::services::disk_reconcile::watcher_batch::{collect_rename_hints, WatcherRenameHints};
use crate::services::scanner::watcher::ModWatchEvent;

#[derive(Debug, Clone)]
pub struct ReconcileOutcome {
    pub status: DiskReconcileStatus,
    pub error_message: Option<String>,
    pub changed_roots: Vec<String>,
    pub thumbnail_roots: Vec<String>,
    pub objects_changed: bool,
    pub folders_changed: bool,
    pub runtime_file_changed: bool,
    pub cleared_selection_paths: Vec<String>,
    pub path_updates: Vec<DiskReconcilePathUpdate>,
    pub change_summary: DiskReconcileChangeSummary,
}

#[derive(Debug, Clone)]
struct DiskObjectEntry {
    folder_path: String,
    folder_path_key: String,
    name: String,
    is_disabled: bool,
}

#[derive(Debug, Clone)]
struct DiskModEntry {
    folder_path: String,
    folder_path_key: String,
    object_folder_path_key: String,
    object_disabled: bool,
    raw_name: String,
    absolute_path: PathBuf,
}

#[derive(Debug, Clone, FromRow)]
struct DbObjectRow {
    id: String,
    folder_path: String,
    folder_path_key: String,
    status: ItemStatus,
    object_type: String,
}

#[derive(Debug, Clone, FromRow)]
struct DbModRow {
    id: String,
    folder_path: String,
    folder_path_key: String,
    actual_name: String,
    status: ItemStatus,
    object_id: Option<String>,
    is_safe: bool,
    corridor_source: Option<String>,
    object_type: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct DiskProjection {
    objects: Vec<DiskObjectEntry>,
    mods: Vec<DiskModEntry>,
}

#[derive(Debug, Default)]
struct ChangeSummaryBuilder {
    object_changes: DiskReconcileChangeCounts,
    mod_changes: DiskReconcileChangeCounts,
    object_sample_names: BTreeSet<String>,
    mod_sample_names: BTreeSet<String>,
}

impl ChangeSummaryBuilder {
    fn record_object_added(&mut self, name: &str) {
        self.object_changes.added += 1;
        self.record_object_sample(name);
    }

    fn record_object_removed(&mut self, name: &str) {
        self.object_changes.removed += 1;
        self.record_object_sample(name);
    }

    fn record_object_renamed(&mut self, name: &str) {
        self.object_changes.renamed += 1;
        self.record_object_sample(name);
    }

    fn record_mod_added(&mut self, name: &str) {
        self.mod_changes.added += 1;
        self.record_mod_sample(name);
    }

    fn record_mod_removed(&mut self, name: &str) {
        self.mod_changes.removed += 1;
        self.record_mod_sample(name);
    }

    fn record_mod_renamed(&mut self, name: &str) {
        self.mod_changes.renamed += 1;
        self.record_mod_sample(name);
    }

    fn record_mod_modified(&mut self, name: &str) {
        self.mod_changes.modified += 1;
        self.record_mod_sample(name);
    }

    fn record_object_sample(&mut self, name: &str) {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return;
        }

        self.object_sample_names.insert(trimmed.to_string());
    }

    fn record_mod_sample(&mut self, name: &str) {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return;
        }

        self.mod_sample_names.insert(trimmed.to_string());
    }

    fn build(self) -> DiskReconcileChangeSummary {
        let has_user_visible_changes = self.object_changes.added > 0
            || self.object_changes.removed > 0
            || self.object_changes.renamed > 0
            || self.object_changes.modified > 0
            || self.mod_changes.added > 0
            || self.mod_changes.removed > 0
            || self.mod_changes.renamed > 0
            || self.mod_changes.modified > 0;

        DiskReconcileChangeSummary {
            object_changes: self.object_changes,
            mod_changes: self.mod_changes,
            object_sample_names: self.object_sample_names.into_iter().take(8).collect(),
            mod_sample_names: self.mod_sample_names.into_iter().take(8).collect(),
            has_user_visible_changes,
        }
    }
}

fn should_run_scoped_disk_reconcile(
    reason: &DiskReconcileReason,
    changed_roots: &[String],
) -> bool {
    if changed_roots.is_empty() {
        return false;
    }

    matches!(
        reason,
        DiskReconcileReason::WatcherBatch | DiskReconcileReason::InternalMutation
    )
}

fn runtime_roots(descriptors: &[ObjectRuntimeDescriptor]) -> BTreeSet<String> {
    descriptors
        .iter()
        .map(|entry| entry.folder_path.clone())
        .collect()
}

fn diff_cleared_paths(
    before: &[ObjectRuntimeDescriptor],
    after: &[ObjectRuntimeDescriptor],
) -> Vec<String> {
    let before_roots = runtime_roots(before);
    let after_roots = runtime_roots(after);

    before_roots.difference(&after_roots).cloned().collect()
}

fn merge_changed_roots(
    changed_roots: &[String],
    before: &[ObjectRuntimeDescriptor],
    after: &[ObjectRuntimeDescriptor],
) -> Vec<String> {
    let mut roots: BTreeSet<String> = changed_roots.iter().cloned().collect();
    let before_roots = runtime_roots(before);
    let after_roots = runtime_roots(after);
    roots.extend(before_roots.symmetric_difference(&after_roots).cloned());
    roots.into_iter().collect()
}

fn collect_runtime_file_changed(changed_paths: &[String]) -> bool {
    changed_paths
        .iter()
        .any(|value| is_runtime_relevant_file(Path::new(value)))
}

fn push_path_update(
    path_updates: &mut Vec<DiskReconcilePathUpdate>,
    kind: DiskReconcilePathKind,
    from: &str,
    to: &str,
) {
    if from == to {
        return;
    }

    if path_updates
        .iter()
        .any(|entry| entry.kind == kind && entry.from == from && entry.to == to)
    {
        return;
    }

    path_updates.push(DiskReconcilePathUpdate {
        from: from.to_string(),
        to: to.to_string(),
        kind,
    });
}

fn runtime_mod_sample_name(mods_path: &Path, changed_path: &str) -> Option<String> {
    let relative = Path::new(changed_path).strip_prefix(mods_path).ok()?;
    let mut components = relative.components();
    components.next()?;
    let mod_component = components.next()?;
    Some(normalize_runtime_name(
        &mod_component.as_os_str().to_string_lossy(),
    ))
}

fn record_runtime_modifications(
    mods_path: &Path,
    changed_paths: &[String],
    change_summary: &mut ChangeSummaryBuilder,
) {
    let mut seen = BTreeSet::new();

    for changed_path in changed_paths {
        if !is_runtime_relevant_file(Path::new(changed_path)) {
            continue;
        }

        let relative = match Path::new(changed_path).strip_prefix(mods_path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let dedupe_key = relative
            .components()
            .take(2)
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        if dedupe_key.len() != 2 || !seen.insert(dedupe_key.join("/")) {
            continue;
        }

        if let Some(sample_name) = runtime_mod_sample_name(mods_path, changed_path) {
            change_summary.record_mod_modified(&sample_name);
        }
    }
}

fn root_key(root: &str) -> String {
    crate::services::path_key::canonical_name_key(root)
}

fn root_key_for_folder_path(folder_path: &str) -> Option<String> {
    let first = Path::new(folder_path).components().next()?;
    Some(root_key(&first.as_os_str().to_string_lossy()))
}

fn list_runtime_dirs(path: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let entries = std::fs::read_dir(path)
        .map_err(|error| format!("Failed to read directory '{}': {error}", path.display()))?;
    let mut result = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "Failed to read directory entry in '{}': {error}",
                path.display()
            )
        })?;
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "Failed to read file type for '{}' in '{}': {error}",
                entry.file_name().to_string_lossy(),
                path.display()
            )
        })?;
        if !file_type.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        result.push((name, entry.path()));
    }

    Ok(result)
}

fn collect_disk_projection(
    mods_path: &Path,
    changed_roots: &[String],
    scoped: bool,
) -> Result<DiskProjection, String> {
    if !mods_path.exists() || !mods_path.is_dir() {
        return Err(format!(
            "Disk Reconcile mods path is unavailable: {}",
            mods_path.display()
        ));
    }

    let mut projection = DiskProjection::default();
    let target_roots = if scoped {
        changed_roots
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|root| (root.clone(), mods_path.join(&root)))
            .collect::<Vec<_>>()
    } else {
        list_runtime_dirs(mods_path)?
    };

    for (root_name, root_path) in target_roots {
        if !root_path.exists() || !root_path.is_dir() {
            continue;
        }

        let object_entry = DiskObjectEntry {
            folder_path: root_name.clone(),
            folder_path_key: crate::services::path_key::folder_path_key(&root_name, None),
            name: normalize_runtime_name(&root_name),
            is_disabled: is_disabled_runtime_name(&root_name),
        };
        let object_folder_path_key = object_entry.folder_path_key.clone();
        let object_folder_path = object_entry.folder_path.clone();
        let object_disabled = object_entry.is_disabled;
        projection.objects.push(object_entry);

        for (mod_name, mod_path) in list_runtime_dirs(&root_path)? {
            let relative_path = PathBuf::from(&object_folder_path)
                .join(&mod_name)
                .to_string_lossy()
                .to_string();
            projection.mods.push(DiskModEntry {
                folder_path: relative_path.clone(),
                folder_path_key: crate::services::path_key::folder_path_key(&relative_path, None),
                object_folder_path_key: object_folder_path_key.clone(),
                object_disabled,
                raw_name: mod_name,
                absolute_path: mod_path,
            });
        }
    }

    Ok(projection)
}

async fn load_db_objects(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<DbObjectRow>, String> {
    sqlx::query_as::<_, DbObjectRow>(
        "SELECT id, folder_path, folder_path_key, status, object_type FROM objects WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(|error| error.to_string())
}

async fn load_db_mods(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<DbModRow>, String> {
    sqlx::query_as::<_, DbModRow>(
        "SELECT id, folder_path, folder_path_key, actual_name, status, object_id, COALESCE(is_safe, 1) as is_safe, corridor_source, object_type FROM mods WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(|error| error.to_string())
}

async fn load_object_type(
    conn: &mut sqlx::SqliteConnection,
    object_id: &str,
) -> Result<String, String> {
    sqlx::query_scalar::<_, Option<String>>("SELECT object_type FROM objects WHERE id = ?")
        .bind(object_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|error| error.to_string())?
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Disk Reconcile object type missing for object '{object_id}'"))
}

async fn load_existing_manual_safe(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    folder_path: &str,
    mods_path: &str,
) -> Result<Option<bool>, String> {
    let row = sqlx::query_as::<_, (bool, Option<String>)>(
        "SELECT COALESCE(is_safe, 1), corridor_source FROM mods WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(game_id)
    .bind(crate::services::path_key::folder_path_key(
        folder_path,
        Some(mods_path),
    ))
    .fetch_optional(&mut *conn)
    .await
    .map_err(|error| error.to_string())?;

    Ok(row.and_then(|(is_safe, corridor_source)| {
        (corridor_source.as_deref() == Some(CORRIDOR_SOURCE_MANUAL)).then_some(is_safe)
    }))
}

async fn apply_mod_rename_hints(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods_path: &Path,
    mods_root: &str,
    safe_mode_keywords: &[String],
    hints: &WatcherRenameHints,
    path_updates: &mut Vec<DiskReconcilePathUpdate>,
    change_summary: &mut ChangeSummaryBuilder,
) -> Result<(), String> {
    for (old_relative, new_relative) in &hints.mod_renames {
        let mod_exists =
            crate::repo::mod_repo::get_mod_id_and_status_by_path(&mut *conn, old_relative, game_id)
                .await
                .map_err(|error| error.to_string())?;
        if mod_exists.is_none() {
            continue;
        }
        let old_id = mod_exists.unwrap().0;

        let components = Path::new(new_relative).components().collect::<Vec<_>>();
        if components.len() != 2 {
            continue;
        }

        let object_folder = components[0].as_os_str().to_string_lossy().to_string();
        let mod_folder = components[1].as_os_str().to_string_lossy().to_string();
        let mut new_objects_count = 0usize;
        let object_id = crate::repo::object_repo::ensure_object_exists(
            &mut *conn,
            game_id,
            &object_folder,
            &normalize_runtime_name(&object_folder),
            "Other",
            None,
            "[]",
            "{}",
            None,
            None,
            &mut new_objects_count,
        )
        .await?;
        let object_type = load_object_type(&mut *conn, &object_id).await?;
        let existing_manual_safe =
            load_existing_manual_safe(&mut *conn, game_id, old_relative, mods_root).await?;
        let metadata = load_runtime_mod_metadata(
            &mods_path.join(new_relative),
            &mod_folder,
            is_disabled_runtime_name(&object_folder),
            safe_mode_keywords,
            existing_manual_safe,
        );
        let new_id = generate_stable_mod_id(game_id, new_relative);

        crate::repo::mod_repo::update_mod_identity_tx(
            &mut *conn,
            &new_id,
            new_relative,
            &metadata.actual_name,
            metadata.status,
            metadata.is_safe,
            metadata.corridor_source,
            &old_id,
            Some(mods_root),
        )
        .await
        .map_err(|error| error.to_string())?;

        crate::repo::mod_repo::update_mod_object_id_and_type_tx(
            &mut *conn,
            &new_id,
            &object_id,
            &object_type,
        )
        .await
        .map_err(|error| error.to_string())?;

        crate::services::collection_service::handle_mod_moved_or_renamed_tx(
            &mut *conn,
            old_relative,
            new_relative,
            Some(&object_id),
        )
        .await
        .map_err(|error| format!("Failed to heal mod rename in collections: {error}"))?;

        push_path_update(
            path_updates,
            DiskReconcilePathKind::Mod,
            old_relative,
            new_relative,
        );
        change_summary.record_mod_renamed(&metadata.actual_name);
    }

    Ok(())
}

async fn apply_object_rename_hints(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods_root: &str,
    hints: &WatcherRenameHints,
    path_updates: &mut Vec<DiskReconcilePathUpdate>,
    change_summary: &mut ChangeSummaryBuilder,
) -> Result<(), String> {
    for (old_folder, new_folder) in &hints.object_renames {
        let next_status = ItemStatus::from_is_disabled(is_disabled_runtime_name(new_folder));
        crate::repo::object_repo::update_object_runtime_state_by_path(
            &mut *conn,
            game_id,
            old_folder,
            new_folder,
            next_status,
        )
        .await
        .map_err(|error| format!("Failed to update object folder path: {error}"))?;

        for (old_sep, new_sep) in [
            (format!("{old_folder}\\"), format!("{new_folder}\\")),
            (format!("{old_folder}/"), format!("{new_folder}/")),
        ] {
            crate::repo::mod_repo::update_child_paths_tx(
                &mut *conn,
                game_id,
                &old_sep,
                &new_sep,
                Some(mods_root),
            )
            .await
            .map_err(|error| format!("Failed to update child paths: {error}"))?;
        }

        crate::repo::mod_repo::update_status_for_object(
            &mut *conn,
            game_id,
            new_folder,
            next_status,
        )
        .await
        .map_err(|error| format!("Failed to update object status: {error}"))?;

        crate::services::collection_service::handle_object_renamed_tx(
            &mut *conn, old_folder, new_folder,
        )
        .await
        .map_err(|error| format!("Failed to heal object rename in collections: {error}"))?;

        push_path_update(
            path_updates,
            DiskReconcilePathKind::Object,
            old_folder,
            new_folder,
        );
        change_summary.record_object_renamed(&normalize_runtime_name(new_folder));
    }

    Ok(())
}

async fn apply_watcher_rename_hints(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods_path: &Path,
    safe_mode_keywords: &[String],
    watcher_events: &[ModWatchEvent],
    path_updates: &mut Vec<DiskReconcilePathUpdate>,
    change_summary: &mut ChangeSummaryBuilder,
) -> Result<(), String> {
    let hints = collect_rename_hints(mods_path, watcher_events);
    if hints.mod_renames.is_empty() && hints.object_renames.is_empty() {
        return Ok(());
    }

    let mods_root = mods_path.to_string_lossy().to_string();
    apply_mod_rename_hints(
        &mut *conn,
        game_id,
        mods_path,
        &mods_root,
        safe_mode_keywords,
        &hints,
        path_updates,
        change_summary,
    )
    .await?;
    apply_object_rename_hints(
        &mut *conn,
        game_id,
        &mods_root,
        &hints,
        path_updates,
        change_summary,
    )
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn reconcile_projection_in_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods_path: &Path,
    safe_mode_keywords: &[String],
    projection: &DiskProjection,
    changed_roots: &[String],
    force_full: bool,
    path_updates: &mut Vec<DiskReconcilePathUpdate>,
    change_summary: &mut ChangeSummaryBuilder,
) -> Result<(bool, bool), String> {
    let db_objects = load_db_objects(&mut *conn, game_id).await?;
    let db_mods = load_db_mods(&mut *conn, game_id).await?;
    let db_objects_by_key = db_objects
        .iter()
        .cloned()
        .map(|row| (row.folder_path_key.clone(), row))
        .collect::<HashMap<_, _>>();
    let db_objects_by_id = db_objects
        .iter()
        .cloned()
        .map(|row| (row.id.clone(), row))
        .collect::<HashMap<_, _>>();
    let db_mods_by_key = db_mods
        .iter()
        .cloned()
        .map(|row| (row.folder_path_key.clone(), row))
        .collect::<HashMap<_, _>>();
    let db_mods_by_path_lower = db_mods
        .iter()
        .cloned()
        .map(|row| (row.folder_path.to_ascii_lowercase(), row))
        .collect::<HashMap<_, _>>();
    let scope_root_keys = changed_roots
        .iter()
        .map(|root| root_key(root))
        .collect::<HashSet<_>>();
    let mods_root = mods_path.to_string_lossy().to_string();

    let mut object_ids_by_key = HashMap::new();
    let mut object_types_by_key = HashMap::new();
    let mut seen_object_keys = HashSet::new();
    let mut seen_mod_keys = HashSet::new();
    let mut deleted_object_keys = HashSet::new();
    let mut objects_changed = false;
    let mut folders_changed = false;

    for disk_object in &projection.objects {
        let expected_status = ItemStatus::from_is_disabled(disk_object.is_disabled);
        let existing = db_objects_by_key.get(&disk_object.folder_path_key).cloned();
        if let Some(existing_object) = &existing {
            if existing_object.folder_path != disk_object.folder_path
                || existing_object.status != expected_status
            {
                crate::repo::object_repo::update_object_runtime_state_by_path(
                    &mut *conn,
                    game_id,
                    &existing_object.folder_path,
                    &disk_object.folder_path,
                    expected_status,
                )
                .await
                .map_err(|error| format!("Failed to update object runtime state: {error}"))?;

                if existing_object.folder_path != disk_object.folder_path {
                    crate::services::collection_service::handle_object_renamed_tx(
                        &mut *conn,
                        &existing_object.folder_path,
                        &disk_object.folder_path,
                    )
                    .await
                    .map_err(|error| {
                        format!("Failed to heal object rename in collections: {error}")
                    })?;

                    folders_changed = true;
                    push_path_update(
                        path_updates,
                        DiskReconcilePathKind::Object,
                        &existing_object.folder_path,
                        &disk_object.folder_path,
                    );
                    change_summary.record_object_renamed(&disk_object.name);
                }

                objects_changed = true;
            }
        }

        let mut new_objects_count = 0usize;
        let object_id = crate::repo::object_repo::ensure_object_exists(
            &mut *conn,
            game_id,
            &disk_object.folder_path,
            &disk_object.name,
            "Other",
            None,
            "[]",
            "{}",
            None,
            None,
            &mut new_objects_count,
        )
        .await?;
        crate::repo::object_repo::update_object_runtime_state_by_id(
            &mut *conn,
            &object_id,
            &disk_object.folder_path,
            expected_status,
        )
        .await
        .map_err(|error| format!("Failed to sync object runtime state: {error}"))?;
        if new_objects_count > 0 {
            objects_changed = true;
            change_summary.record_object_added(&disk_object.name);
        }

        if let Some(existing_object) = db_objects_by_id.get(&object_id) {
            seen_object_keys.insert(existing_object.folder_path_key.clone());
        }

        let object_type = db_objects_by_id
            .get(&object_id)
            .map(|existing_object| existing_object.object_type.clone())
            .unwrap_or_else(|| "Other".to_string());

        object_ids_by_key.insert(disk_object.folder_path_key.clone(), object_id);
        object_types_by_key.insert(disk_object.folder_path_key.clone(), object_type);
        seen_object_keys.insert(disk_object.folder_path_key.clone());
    }

    for disk_mod in &projection.mods {
        let existing = db_mods_by_key
            .get(&disk_mod.folder_path_key)
            .or_else(|| db_mods_by_path_lower.get(&disk_mod.folder_path.to_ascii_lowercase()))
            .cloned();
        let existing_manual_safe = existing.as_ref().and_then(|row| {
            (row.corridor_source.as_deref() == Some(CORRIDOR_SOURCE_MANUAL)).then_some(row.is_safe)
        });
        let metadata = load_runtime_mod_metadata(
            &disk_mod.absolute_path,
            &disk_mod.raw_name,
            disk_mod.object_disabled,
            safe_mode_keywords,
            existing_manual_safe,
        );
        let object_id = object_ids_by_key
            .get(&disk_mod.object_folder_path_key)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Disk Reconcile object mapping missing for '{}'",
                    disk_mod.folder_path
                )
            })?;
        let object_type = object_types_by_key
            .get(&disk_mod.object_folder_path_key)
            .cloned()
            .unwrap_or_else(|| "Other".to_string());
        let new_id = generate_stable_mod_id(game_id, &disk_mod.folder_path);

        if let Some(existing_mod) = &existing {
            let existing_corridor_source = existing_mod
                .corridor_source
                .as_deref()
                .unwrap_or(CORRIDOR_SOURCE_UNKNOWN);
            let path_changed = existing_mod.folder_path != disk_mod.folder_path;
            let name_changed = existing_mod.actual_name != metadata.actual_name;
            let status_changed = existing_mod.status != metadata.status;
            let safety_changed = existing_mod.is_safe != metadata.is_safe
                || existing_corridor_source != metadata.corridor_source;
            let object_changed = existing_mod.object_id.as_deref() != Some(&object_id);
            let type_changed = existing_mod.object_type.as_deref() != Some(object_type.as_str());
            let id_changed = existing_mod.id != new_id;

            if path_changed || name_changed || status_changed || safety_changed || id_changed {
                crate::repo::mod_repo::update_mod_identity_tx(
                    &mut *conn,
                    &new_id,
                    &disk_mod.folder_path,
                    &metadata.actual_name,
                    metadata.status,
                    metadata.is_safe,
                    metadata.corridor_source,
                    &existing_mod.id,
                    Some(&mods_root),
                )
                .await
                .map_err(|error| format!("Failed to update mod identity: {error}"))?;
                folders_changed = true;
                if path_changed {
                    push_path_update(
                        path_updates,
                        DiskReconcilePathKind::Mod,
                        &existing_mod.folder_path,
                        &disk_mod.folder_path,
                    );
                    change_summary.record_mod_renamed(&metadata.actual_name);
                }
            }

            if object_changed || type_changed {
                crate::repo::mod_repo::update_mod_object_id_and_type_tx(
                    &mut *conn,
                    &new_id,
                    &object_id,
                    &object_type,
                )
                .await
                .map_err(|error| format!("Failed to update mod object mapping: {error}"))?;
                folders_changed = true;
            }

            if path_changed {
                crate::services::collection_service::handle_mod_moved_or_renamed_tx(
                    &mut *conn,
                    &existing_mod.folder_path,
                    &disk_mod.folder_path,
                    Some(&object_id),
                )
                .await
                .map_err(|error| format!("Failed to heal mod rename in collections: {error}"))?;
            }
        } else {
            crate::repo::mod_repo::insert_mod_with_reason_tx(
                &mut *conn,
                &new_id,
                game_id,
                &object_id,
                &metadata.actual_name,
                &disk_mod.folder_path,
                Some(&mods_root),
                metadata.status,
                &object_type,
                false,
                metadata.is_safe,
                metadata.corridor_source,
                if metadata.status.is_enabled() {
                    None
                } else {
                    Some(crate::services::corridor_constants::DISABLED_REASON_USER)
                },
            )
            .await
            .map_err(|error| format!("Failed to insert mod: {error}"))?;
            folders_changed = true;
            change_summary.record_mod_added(&metadata.actual_name);
        }

        seen_mod_keys.insert(disk_mod.folder_path_key.clone());
    }

    for db_object in &db_objects {
        let in_scope = force_full || scope_root_keys.contains(&db_object.folder_path_key);
        if !in_scope || seen_object_keys.contains(&db_object.folder_path_key) {
            continue;
        }

        crate::repo::object_repo::delete_object_and_mods_by_folder(
            &mut *conn,
            game_id,
            &db_object.folder_path,
        )
        .await
        .map_err(|error| {
            format!(
                "Failed to delete object folder '{}': {error}",
                db_object.folder_path
            )
        })?;
        deleted_object_keys.insert(db_object.folder_path_key.clone());
        objects_changed = true;
        folders_changed = true;
        change_summary.record_object_removed(&normalize_runtime_name(&db_object.folder_path));
    }

    for db_mod in &db_mods {
        let Some(db_root_key) = root_key_for_folder_path(&db_mod.folder_path) else {
            continue;
        };
        let in_scope = force_full || scope_root_keys.contains(&db_root_key);
        if !in_scope
            || seen_mod_keys.contains(&db_mod.folder_path_key)
            || deleted_object_keys.contains(&db_root_key)
        {
            continue;
        }

        let exists_on_disk = mods_path.join(&db_mod.folder_path).exists();
        if exists_on_disk {
            continue;
        }

        crate::repo::mod_repo::delete_mod_tx(&mut *conn, &db_mod.id)
            .await
            .map_err(|error| {
                format!(
                    "Failed to delete stale mod '{}': {error}",
                    db_mod.folder_path
                )
            })?;
        folders_changed = true;
        change_summary.record_mod_removed(&db_mod.actual_name);
    }

    Ok((objects_changed, folders_changed))
}

#[allow(clippy::too_many_arguments)]
/// Disk Reconcile updates the runtime projection from filesystem reality only.
/// Runtime-discovered folders remain `Other` until the explicit Deep Match Scanner runs.
pub async fn reconcile_disk_projection(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &Path,
    safe_mode_keywords: &[String],
    reason: &DiskReconcileReason,
    changed_paths: &[String],
    force_full: bool,
    watcher_events: Option<&[ModWatchEvent]>,
) -> Result<ReconcileOutcome, String> {
    let mut changed_roots = collect_changed_roots(mods_path, changed_paths);
    let thumbnail_roots = collect_thumbnail_roots(mods_path, changed_paths);
    let runtime_file_changed = collect_runtime_file_changed(changed_paths);
    if !mods_path.exists() || !mods_path.is_dir() {
        return Ok(ReconcileOutcome {
            status: DiskReconcileStatus::SourceUnavailable,
            error_message: Some(format!(
                "Disk Reconcile mods path is unavailable: {}",
                mods_path.display()
            )),
            changed_roots,
            thumbnail_roots,
            objects_changed: false,
            folders_changed: false,
            runtime_file_changed,
            cleared_selection_paths: Vec::new(),
            path_updates: Vec::new(),
            change_summary: ChangeSummaryBuilder::default().build(),
        });
    }

    let should_reconcile = force_full
        || !matches!(reason, DiskReconcileReason::WatcherBatch)
        || !changed_roots.is_empty();

    let before_descriptors = crate::repo::object_repo::get_runtime_descriptors(pool, game_id)
        .await
        .map_err(|error| error.to_string())?;

    let mut objects_changed = false;
    let mut folders_changed = false;
    let mut cleared_selection_paths = Vec::new();
    let mut path_updates = Vec::new();
    let mut change_summary = ChangeSummaryBuilder::default();

    if runtime_file_changed {
        record_runtime_modifications(mods_path, changed_paths, &mut change_summary);
    }

    if should_reconcile {
        let scoped = !force_full && should_run_scoped_disk_reconcile(reason, &changed_roots);
        let projection = collect_disk_projection(mods_path, &changed_roots, scoped)?;
        let mut tx = pool.begin().await.map_err(|error| error.to_string())?;

        if let Some(events) = watcher_events {
            apply_watcher_rename_hints(
                &mut *tx,
                game_id,
                mods_path,
                safe_mode_keywords,
                events,
                &mut path_updates,
                &mut change_summary,
            )
            .await?;
        }

        let (objects_changed_tx, folders_changed_tx) = reconcile_projection_in_tx(
            &mut *tx,
            game_id,
            mods_path,
            safe_mode_keywords,
            &projection,
            &changed_roots,
            force_full,
            &mut path_updates,
            &mut change_summary,
        )
        .await?;

        tx.commit().await.map_err(|error| error.to_string())?;
        crate::services::runtime_projection_service::rebuild_game_projection(pool, game_id)
            .await
            .map_err(|error| error.to_string())?;

        objects_changed = objects_changed_tx;
        folders_changed = folders_changed_tx;

        let after_descriptors = crate::repo::object_repo::get_runtime_descriptors(pool, game_id)
            .await
            .map_err(|error| error.to_string())?;
        cleared_selection_paths = diff_cleared_paths(&before_descriptors, &after_descriptors);

        if objects_changed || folders_changed {
            changed_roots =
                merge_changed_roots(&changed_roots, &before_descriptors, &after_descriptors);
        }
    }
    Ok(ReconcileOutcome {
        status: DiskReconcileStatus::Applied,
        error_message: None,
        changed_roots,
        thumbnail_roots,
        objects_changed,
        folders_changed,
        runtime_file_changed,
        cleared_selection_paths,
        path_updates,
        change_summary: change_summary.build(),
    })
}
