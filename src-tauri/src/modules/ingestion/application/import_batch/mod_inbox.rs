use super::types::{
    CreateImportBatchInput, ImportBatch, ImportFlow, ImportSourceInput, ImportSourceKind,
    ModInboxEntry, ModInboxEntryKind, ModInboxLayout, ModInboxRootState, ModInboxSnapshot,
    ProcessedModInboxDestination, ProcessedModInboxSource, TargetMode,
};
use crate::modules::ingestion::adapters::sqlite::import_batch;
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

pub async fn build_mod_inbox_snapshot(
    db: &SqlitePool,
    game_id: &str,
    root: &Path,
) -> Result<ModInboxSnapshot, AppError> {
    let processed_sources = build_processed_sources(db, game_id).await?;
    if !root.exists() {
        return Ok(ModInboxSnapshot {
            game_id: game_id.to_string(),
            root_path: root.to_string_lossy().into_owned(),
            root_state: ModInboxRootState::Missing,
            ready_entries: Vec::new(),
            processed_sources,
        });
    }
    if !root.is_dir() {
        return Err(AppError::Validation(format!(
            "Mod Inbox path is not a directory: {}",
            root.display()
        )));
    }

    let canonical_root = root.canonicalize()?;
    let active = import_batch::list_active_mod_inbox_sources(db, game_id).await?;
    let active_by_path = active
        .into_iter()
        .map(|source| (source.source_path, source.batch_id))
        .collect::<BTreeMap<_, _>>();
    let root_for_worker = canonical_root.clone();
    let mut ready_entries =
        tokio::task::spawn_blocking(move || discover_entries(&root_for_worker, &active_by_path))
            .await??;
    ready_entries.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
    });

    Ok(ModInboxSnapshot {
        game_id: game_id.to_string(),
        root_path: canonical_root.to_string_lossy().into_owned(),
        root_state: ModInboxRootState::Ready,
        ready_entries,
        processed_sources,
    })
}

pub async fn create_mod_inbox_batch(
    db: &SqlitePool,
    game_id: &str,
    root: &Path,
    entry_keys: &[String],
) -> Result<ImportBatch, AppError> {
    if entry_keys.is_empty() {
        return Err(AppError::Validation(
            "Select at least one Mod Inbox entry".to_string(),
        ));
    }
    let requested = entry_keys.iter().collect::<BTreeSet<_>>();
    if requested.len() != entry_keys.len() {
        return Err(AppError::Validation(
            "Mod Inbox selection contains duplicate entries".to_string(),
        ));
    }
    if !root.exists() {
        return Err(AppError::Validation(
            "Create the Mod Inbox folder before importing".to_string(),
        ));
    }
    if !root.is_dir() {
        return Err(AppError::Validation(format!(
            "Mod Inbox path is not a directory: {}",
            root.display()
        )));
    }

    let canonical_root = root.canonicalize()?;
    let requested_keys = entry_keys.to_vec();
    let root_for_worker = canonical_root.clone();
    let mut entries = tokio::task::spawn_blocking(move || {
        discover_selected_entries(&root_for_worker, &requested_keys)
    })
    .await??;
    let selected_paths = entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    let active =
        import_batch::list_active_mod_inbox_sources_for_paths(db, game_id, &selected_paths).await?;
    let active_by_path = active
        .into_iter()
        .map(|source| (source.source_path, source.batch_id))
        .collect::<BTreeMap<_, _>>();
    for entry in &mut entries {
        entry.pending_batch_id = active_by_path.get(&entry.path).cloned();
    }
    entries.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
    });
    if entries.iter().any(|entry| entry.pending_batch_id.is_some()) {
        return Err(AppError::Validation(
            "One or more Mod Inbox entries already belong to an active import batch".to_string(),
        ));
    }

    super::coordinator::create_mod_inbox_import_batch(
        db,
        CreateImportBatchInput {
            game_id: game_id.to_string(),
            flow: ImportFlow::ReadyToMove,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: entries
                .into_iter()
                .map(|entry| ImportSourceInput {
                    path: entry.path,
                    source_kind: Some(ImportSourceKind::ReadyToMove),
                })
                .collect(),
        },
    )
    .await
}

pub async fn delete_processed_sources(
    db: &SqlitePool,
    game_id: &str,
    root: &Path,
    source_ids: &[String],
) -> Result<ModInboxSnapshot, AppError> {
    if source_ids.is_empty() {
        return build_mod_inbox_snapshot(db, game_id, root).await;
    }
    let snapshot = build_mod_inbox_snapshot(db, game_id, root).await?;
    let requested = source_ids.iter().collect::<std::collections::BTreeSet<_>>();
    let selected = snapshot
        .processed_sources
        .iter()
        .filter(|source| requested.contains(&source.source_id))
        .collect::<Vec<_>>();
    if selected.len() != requested.len() {
        return Err(AppError::Validation(
            "One or more Processed sources are stale; refresh and try again".to_string(),
        ));
    }
    let processed_root = root.join("Processed").canonicalize().map_err(|error| {
        AppError::Validation(format!("Processed folder is unavailable: {error}"))
    })?;
    let targets = selected
        .into_iter()
        .map(|source| {
            if source.source_deleted_at.is_some() {
                return Err(AppError::Validation(format!(
                    "Processed source '{}' was already deleted",
                    source.name
                )));
            }
            let stored_path = source.processed_path.as_deref().ok_or_else(|| {
                AppError::Validation(format!(
                    "Processed source '{}' is history-only and has no retained file",
                    source.name
                ))
            })?;
            let canonical = Path::new(stored_path).canonicalize().map_err(|error| {
                AppError::Validation(format!(
                    "Processed source '{}' is unavailable: {error}",
                    source.name
                ))
            })?;
            if canonical.parent() != Some(processed_root.as_path()) {
                return Err(AppError::Security(format!(
                    "Processed source escaped the Mod Inbox: {}",
                    canonical.display()
                )));
            }
            Ok((source, canonical))
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    for (source, canonical) in targets {
        crate::platform::fs::recycle_bin::move_path_to_recycle_bin(&canonical)?;
        if import_batch::mark_mod_inbox_source_deleted(db, game_id, &source.source_id).await? == 0 {
            return Err(AppError::Internal(format!(
                "Processed source history '{}' could not be updated",
                source.source_id
            )));
        }
    }
    build_mod_inbox_snapshot(db, game_id, root).await
}

fn discover_entries(
    root: &Path,
    active_by_path: &BTreeMap<String, String>,
) -> Result<Vec<ModInboxEntry>, AppError> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.eq_ignore_ascii_case("Processed") {
            continue;
        }
        let path = entry.path();
        let canonical_path = path.canonicalize()?;
        validate_inbox_entry_containment(root, &canonical_path)?;
        let metadata = entry.metadata()?;
        if let Some(entry) =
            build_inbox_entry(name.clone(), name, canonical_path, metadata, active_by_path)
        {
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn discover_selected_entries(
    root: &Path,
    entry_keys: &[String],
) -> Result<Vec<ModInboxEntry>, AppError> {
    let mut entries = Vec::with_capacity(entry_keys.len());
    let mut canonical_paths = BTreeSet::new();
    let no_active_entries = BTreeMap::new();
    for entry_key in entry_keys {
        let canonical_path = canonicalize_selected_entry(root, entry_key)?;
        let path_string = canonical_path.to_string_lossy().into_owned();
        if !canonical_paths.insert(path_string.clone()) {
            return Err(AppError::Validation(
                "Mod Inbox selection contains duplicate entries".to_string(),
            ));
        }
        let metadata = canonical_path.metadata().map_err(|_| stale_entry_error())?;
        let Some(entry) = build_inbox_entry(
            entry_key.clone(),
            entry_key.clone(),
            canonical_path,
            metadata,
            &no_active_entries,
        ) else {
            return Err(stale_entry_error());
        };
        entries.push(entry);
    }
    Ok(entries)
}

fn canonicalize_selected_entry(root: &Path, entry_key: &str) -> Result<PathBuf, AppError> {
    let mut components = Path::new(entry_key).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(AppError::Security(format!(
            "Mod Inbox entry is not a direct child of the configured inbox: {entry_key}"
        )));
    }
    if entry_key.starts_with('.') || entry_key.eq_ignore_ascii_case("Processed") {
        return Err(stale_entry_error());
    }
    let canonical_path = root
        .join(entry_key)
        .canonicalize()
        .map_err(|_| stale_entry_error())?;
    validate_inbox_entry_containment(root, &canonical_path)?;
    Ok(canonical_path)
}

fn stale_entry_error() -> AppError {
    AppError::Validation(
        "One or more Mod Inbox entries are stale; refresh and try again".to_string(),
    )
}

fn build_inbox_entry(
    entry_key: String,
    name: String,
    canonical_path: PathBuf,
    metadata: std::fs::Metadata,
    active_by_path: &BTreeMap<String, String>,
) -> Option<ModInboxEntry> {
    let kind = if canonical_path.is_dir() {
        ModInboxEntryKind::Folder
    } else if canonical_path.is_file()
        && crate::modules::library::application::mods::archive::ArchiveFormat::detect(
            &canonical_path,
        )
        .is_some()
    {
        ModInboxEntryKind::Archive
    } else {
        return None;
    };
    let modified_unix_ms = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_millis().to_string())
        .unwrap_or_else(|| "0".to_string());
    let (layout, detected_root_count) = classify_layout(&canonical_path, kind);
    let path = canonical_path.to_string_lossy().into_owned();
    Some(ModInboxEntry {
        entry_key,
        name,
        path: path.clone(),
        kind,
        archive_format: archive_format_label(&canonical_path),
        size_bytes: (kind == ModInboxEntryKind::Archive).then_some(metadata.len()),
        modified_unix_ms,
        layout,
        detected_root_count,
        pending_batch_id: active_by_path.get(&path).cloned(),
    })
}

fn validate_inbox_entry_containment(root: &Path, canonical_entry: &Path) -> Result<(), AppError> {
    if canonical_entry.parent() == Some(root) {
        Ok(())
    } else {
        Err(AppError::Security(format!(
            "Mod Inbox entry escapes the configured inbox: {}",
            canonical_entry.display()
        )))
    }
}

fn classify_layout(path: &Path, kind: ModInboxEntryKind) -> (ModInboxLayout, u32) {
    if kind == ModInboxEntryKind::Archive {
        return (ModInboxLayout::Unknown, 0);
    }
    let roots = crate::modules::library::application::mods::archive::classify::find_mod_roots(
        path,
        crate::modules::library::application::mods::archive::classify::MOD_ROOT_MAX_DEPTH,
    );
    let count = roots.len() as u32;
    match roots.as_slice() {
        [] => (ModInboxLayout::Unknown, 0),
        [root] if root == path => (ModInboxLayout::DirectMod, 1),
        [_] => (ModInboxLayout::Wrapper, 1),
        _ => (ModInboxLayout::FolderPack, count),
    }
}

fn archive_format_label(path: &Path) -> Option<String> {
    use crate::modules::library::application::mods::archive::ArchiveFormat;
    match ArchiveFormat::detect(path)? {
        ArchiveFormat::Zip => Some("zip".to_string()),
        ArchiveFormat::SevenZ => Some("7z".to_string()),
        ArchiveFormat::Rar => Some("rar".to_string()),
    }
}

async fn build_processed_sources(
    db: &SqlitePool,
    game_id: &str,
) -> Result<Vec<ProcessedModInboxSource>, AppError> {
    let rows = import_batch::list_mod_inbox_history_rows(db, game_id).await?;
    let mut grouped = BTreeMap::<String, ProcessedModInboxSource>::new();
    for row in rows {
        let name = Path::new(&row.source_path)
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| row.source_path.clone());
        let kind = inbox_kind_from_path(
            row.processed_source_path
                .as_deref()
                .unwrap_or(&row.source_path),
        );
        let source = grouped
            .entry(row.source_group_id.clone())
            .or_insert_with(|| ProcessedModInboxSource {
                source_id: row.source_group_id,
                name,
                source_kind: kind,
                original_path: row.source_path,
                processed_path: row.processed_source_path,
                processed_at: row.source_processed_at,
                source_deleted_at: row.source_deleted_at,
                destinations: Vec::new(),
            });
        source.destinations.push(ProcessedModInboxDestination {
            object_id: row.destination_object_id,
            object_name: row.object_name,
            placed_path: row.placed_path,
            planned_name: row.planned_name,
            status: row.status,
        });
    }
    let mut sources = grouped.into_values().collect::<Vec<_>>();
    sources.sort_by(|left, right| right.processed_at.cmp(&left.processed_at));
    Ok(sources)
}

fn inbox_kind_from_path(path: &str) -> ModInboxEntryKind {
    let path = PathBuf::from(path);
    if crate::modules::library::application::mods::archive::ArchiveFormat::detect(&path).is_some() {
        ModInboxEntryKind::Archive
    } else {
        ModInboxEntryKind::Folder
    }
}

#[cfg(test)]
mod containment_tests {
    use super::*;

    #[test]
    fn accepts_only_direct_children_of_the_canonical_inbox() {
        let inbox = Path::new("C:/Downloads/mods/GIMI");
        assert!(validate_inbox_entry_containment(inbox, &inbox.join("Ayaka Pack")).is_ok());

        let error = validate_inbox_entry_containment(inbox, Path::new("C:/External/Ayaka Pack"))
            .unwrap_err();
        assert!(matches!(error, AppError::Security(_)));
    }

    #[test]
    fn rejects_selected_paths_that_are_not_direct_inbox_children() {
        let inbox = tempfile::tempdir().unwrap();
        let error = canonicalize_selected_entry(inbox.path(), "../outside").unwrap_err();

        assert!(matches!(error, AppError::Security(_)));
    }
}
