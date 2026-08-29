use super::types::{
    CreateImportBatchInput, ImportBatch, ImportFlow, ImportSourceInput, ImportSourceKind,
    ModInboxEntry, ModInboxEntryKind, ModInboxLayout, ModInboxRootState, ModInboxSnapshot,
    ProcessedModInboxDestination, ProcessedModInboxSource, TargetMode,
};
use crate::domain::errors::AppError;
use crate::repo::import_batch;
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
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
    let snapshot = build_mod_inbox_snapshot(db, game_id, root).await?;
    if snapshot.root_state != ModInboxRootState::Ready {
        return Err(AppError::Validation(
            "Create the Mod Inbox folder before importing".to_string(),
        ));
    }
    let requested = entry_keys.iter().collect::<std::collections::BTreeSet<_>>();
    if requested.len() != entry_keys.len() {
        return Err(AppError::Validation(
            "Mod Inbox selection contains duplicate entries".to_string(),
        ));
    }
    let entries = snapshot
        .ready_entries
        .into_iter()
        .filter(|entry| requested.contains(&entry.entry_key))
        .collect::<Vec<_>>();
    if entries.len() != requested.len() {
        return Err(AppError::Validation(
            "One or more Mod Inbox entries are stale; refresh and try again".to_string(),
        ));
    }
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
        crate::services::fs_utils::recycle_bin::move_path_to_recycle_bin(&canonical)?;
        if import_batch::mark_mod_inbox_source_deleted(db, game_id, &source.source_id).await?
            == 0
        {
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
        let kind = if canonical_path.is_dir() {
            ModInboxEntryKind::Folder
        } else if canonical_path.is_file()
            && crate::services::mods::archive::ArchiveFormat::detect(&canonical_path).is_some()
        {
            ModInboxEntryKind::Archive
        } else {
            continue;
        };
        let metadata = entry.metadata()?;
        let modified_unix_ms = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_millis().to_string())
            .unwrap_or_else(|| "0".to_string());
        let (layout, detected_root_count) = classify_layout(&canonical_path, kind);
        let path_string = canonical_path.to_string_lossy().into_owned();
        entries.push(ModInboxEntry {
            entry_key: name.clone(),
            name,
            path: path_string.clone(),
            kind,
            archive_format: archive_format_label(&canonical_path),
            size_bytes: (kind == ModInboxEntryKind::Archive).then_some(metadata.len()),
            modified_unix_ms,
            layout,
            detected_root_count,
            pending_batch_id: active_by_path.get(&path_string).cloned(),
        });
    }
    Ok(entries)
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
    let roots = crate::services::mods::archive::classify::find_mod_roots(path, 5);
    let count = roots.len() as u32;
    match roots.as_slice() {
        [] => (ModInboxLayout::Unknown, 0),
        [root] if root == path => (ModInboxLayout::DirectMod, 1),
        [_] => (ModInboxLayout::Wrapper, 1),
        _ => (ModInboxLayout::FolderPack, count),
    }
}

fn archive_format_label(path: &Path) -> Option<String> {
    use crate::services::mods::archive::ArchiveFormat;
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
    if crate::services::mods::archive::ArchiveFormat::detect(&path).is_some() {
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
}
