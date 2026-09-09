use super::types::{ImportBatch, ImportBatchStatus, ImportItemStatus};
use crate::modules::ingestion::adapters::sqlite::import_batch::{self, StagedRootRecord};
use crate::modules::library::application::mods::archive::StagingExtractOptions;
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn cleanup_batch_staging(staging_root: &Path, batch_id: &str) -> Result<bool, AppError> {
    if uuid::Uuid::parse_str(batch_id).is_err() || !staging_root.exists() {
        return Ok(false);
    }
    let root = staging_root.canonicalize()?;
    let target = root.join(batch_id);
    if !target.exists() {
        return Ok(false);
    }
    let canonical_target = target.canonicalize()?;
    if canonical_target.parent() != Some(root.as_path()) {
        return Err(AppError::Security(
            "Import staging cleanup target escaped its owned root".to_string(),
        ));
    }
    std::fs::remove_dir_all(canonical_target)?;
    Ok(true)
}

/// Removes incomplete staging attempts left by a crash without touching roots
/// still referenced by a resumable import batch. The UUID-only traversal keeps
/// cleanup confined to the layout owned by the import pipeline.
pub fn cleanup_orphaned_staging(
    staging_root: &Path,
    referenced_staging_paths: &[String],
) -> Result<usize, AppError> {
    if !staging_root.exists() {
        return Ok(0);
    }

    let root = staging_root.canonicalize()?;
    let retained_attempts = referenced_attempt_directories(&root, referenced_staging_paths)?;
    let mut removed = 0;

    for batch in owned_uuid_directories(&root)? {
        for item in owned_uuid_directories(&batch)? {
            for attempt in owned_uuid_directories(&item)? {
                if retained_attempts.contains(&attempt) {
                    continue;
                }
                std::fs::remove_dir_all(&attempt)?;
                removed += 1;
            }
            remove_dir_if_empty(&item)?;
        }
        remove_dir_if_empty(&batch)?;
    }

    Ok(removed)
}

fn referenced_attempt_directories(
    root: &Path,
    referenced_staging_paths: &[String],
) -> Result<HashSet<PathBuf>, AppError> {
    let mut attempts = HashSet::new();

    for staging_path in referenced_staging_paths {
        let path = Path::new(staging_path);
        if !path.exists() {
            continue;
        }

        let canonical_path = path.canonicalize()?;
        let relative = canonical_path.strip_prefix(root).map_err(|_| {
            AppError::Security("Import staging reference escaped its owned root".to_string())
        })?;
        let components = relative.components().collect::<Vec<_>>();
        let [std::path::Component::Normal(batch_id), std::path::Component::Normal(item_id), std::path::Component::Normal(attempt_id), std::path::Component::Normal(kind), ..] =
            components.as_slice()
        else {
            return Err(AppError::Security(
                "Import staging reference has an invalid owned layout".to_string(),
            ));
        };
        if uuid::Uuid::parse_str(&batch_id.to_string_lossy()).is_err()
            || uuid::Uuid::parse_str(&item_id.to_string_lossy()).is_err()
            || uuid::Uuid::parse_str(&attempt_id.to_string_lossy()).is_err()
            || !matches!(kind.to_str(), Some("extracted" | "roots"))
        {
            return Err(AppError::Security(
                "Import staging reference has an invalid owned layout".to_string(),
            ));
        }

        let attempt = root
            .join(batch_id)
            .join(item_id)
            .join(attempt_id)
            .canonicalize()?;
        attempts.insert(attempt);
    }

    Ok(attempts)
}

fn owned_uuid_directories(parent: &Path) -> Result<Vec<PathBuf>, AppError> {
    std::fs::read_dir(parent)?
        .filter_map(|entry| match entry {
            Ok(entry)
                if uuid::Uuid::parse_str(&entry.file_name().to_string_lossy()).is_ok()
                    && entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) =>
            {
                Some(Ok(entry))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .map(|entry| {
            let entry = entry?;
            let canonical = entry.path().canonicalize()?;
            if canonical.parent() != Some(parent) {
                return Err(AppError::Security(
                    "Import staging directory escaped its owned root".to_string(),
                ));
            }
            Ok(canonical)
        })
        .collect()
}

fn remove_dir_if_empty(path: &Path) -> Result<(), AppError> {
    if std::fs::read_dir(path)?.next().is_none() {
        std::fs::remove_dir(path)?;
    }
    Ok(())
}

fn cleanup_item_staging(
    staging_root: &Path,
    batch_id: &str,
    item_id: &str,
) -> Result<bool, AppError> {
    if uuid::Uuid::parse_str(batch_id).is_err()
        || uuid::Uuid::parse_str(item_id).is_err()
        || !staging_root.exists()
    {
        return Ok(false);
    }
    let root = staging_root.canonicalize()?;
    let batch = root.join(batch_id);
    let target = batch.join(item_id);
    if !target.exists() {
        return Ok(false);
    }
    let canonical_batch = batch.canonicalize()?;
    let canonical_target = target.canonicalize()?;
    if canonical_batch.parent() != Some(root.as_path())
        || canonical_target.parent() != Some(canonical_batch.as_path())
    {
        return Err(AppError::Security(
            "Import item staging cleanup target escaped its owned root".to_string(),
        ));
    }
    std::fs::remove_dir_all(canonical_target)?;
    Ok(true)
}

pub async fn stage_import_batch_sources(
    db: &SqlitePool,
    batch_id: &str,
    staging_root: &Path,
) -> Result<ImportBatch, AppError> {
    stage_import_batch_sources_with_options(
        db,
        batch_id,
        staging_root,
        &StagingExtractOptions::default(),
    )
    .await
}

pub async fn stage_import_batch_sources_with_options(
    db: &SqlitePool,
    batch_id: &str,
    staging_root: &Path,
    options: &StagingExtractOptions,
) -> Result<ImportBatch, AppError> {
    let batch = import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    if !matches!(
        batch.status,
        ImportBatchStatus::Draft | ImportBatchStatus::Failed | ImportBatchStatus::Partial
    ) {
        return Err(AppError::Validation(format!(
            "Import batch '{batch_id}' cannot be staged from status {:?}",
            batch.status
        )));
    }
    std::fs::create_dir_all(staging_root)?;
    import_batch::set_batch_status(db, batch_id, ImportBatchStatus::Analyzing).await?;

    for item in batch.items.iter().filter(|item| {
        matches!(
            item.status,
            ImportItemStatus::Discovered | ImportItemStatus::Failed
        )
    }) {
        if super::is_cancelled(&options.cancel_token) {
            return Err(AppError::Cancelled);
        }
        cleanup_item_staging(staging_root, batch_id, &item.id)?;
        if item.status == ImportItemStatus::Failed
            && !import_batch::reset_failed_item_for_staging(db, &item.id).await?
        {
            return Err(AppError::Validation(format!(
                "Import item '{}' changed while preparing a staging retry",
                item.id
            )));
        }
        let source = PathBuf::from(&item.source_path);
        let outcome = if source.is_dir() {
            if batch.flow == super::types::ImportFlow::ReadyToMove {
                stage_ready_to_move_folder(db, batch_id, &item.id, &source, staging_root).await
            } else {
                transition_folder_to_staged(db, &item.id).await
            }
        } else if source.is_file() {
            stage_archive_item(
                db,
                batch_id,
                &item.id,
                &source,
                &item.planned_name,
                staging_root,
                options,
            )
            .await
        } else {
            Err(AppError::Validation(format!(
                "Import source disappeared before staging: {}",
                source.display()
            )))
        };

        if let Err(error) = outcome {
            if matches!(error, AppError::Cancelled) {
                return Err(error);
            }
            let persisted_error = match &error {
                AppError::ArchiveUnsupported { reason } => reason.storage_code().to_string(),
                _ => error.to_string(),
            };
            import_batch::set_item_failure(db, &item.id, &persisted_error).await?;
            import_batch::set_batch_status(db, batch_id, ImportBatchStatus::Partial).await?;
            return Err(error);
        }
    }

    import_batch::set_batch_status(db, batch_id, ImportBatchStatus::AwaitingReview).await?;
    import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::Internal("Staged import batch could not be reloaded".to_string()))
}

async fn transition_folder_to_staged(db: &SqlitePool, item_id: &str) -> Result<(), AppError> {
    let updated = import_batch::transition_item_status(
        db,
        item_id,
        ImportItemStatus::Discovered,
        ImportItemStatus::Staged,
    )
    .await?;
    updated.then_some(()).ok_or_else(|| {
        AppError::Validation(format!("Import item '{item_id}' changed while staging"))
    })
}

async fn stage_ready_to_move_folder(
    db: &SqlitePool,
    batch_id: &str,
    item_id: &str,
    source: &Path,
    staging_root: &Path,
) -> Result<(), AppError> {
    let source_for_worker = source.to_path_buf();
    let item_staging = staging_root
        .join(batch_id)
        .join(item_id)
        .join(uuid::Uuid::new_v4().to_string())
        .join("roots");
    let records = tokio::task::spawn_blocking(move || {
        let mut roots =
            crate::modules::library::application::mods::archive::classify::find_mod_roots(
                &source_for_worker,
                5,
            );
        roots.sort();
        if roots.is_empty() || (roots.len() == 1 && roots[0] == source_for_worker) {
            return Ok(Vec::new());
        }

        std::fs::create_dir_all(&item_staging)?;
        roots
            .into_iter()
            .enumerate()
            .map(|(index, root)| {
                let root_name = root
                    .file_name()
                    .map(|value| value.to_string_lossy().into_owned())
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| format!("mod-{}", index + 1));
                let target = item_staging.join(format!("{}-{}", index + 1, root_name));
                std::fs::create_dir_all(&target)?;
                let options = fs_extra::dir::CopyOptions::new()
                    .copy_inside(true)
                    .content_only(true);
                fs_extra::dir::copy(&root, &target, &options)
                    .map_err(|error| AppError::Io(error.to_string()))?;
                Ok(StagedRootRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    staging_path: target.to_string_lossy().into_owned(),
                    planned_name:
                        crate::modules::library::application::mods::core_ops::standardize_prefix(
                            &root_name, false,
                        ),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()
    })
    .await??;

    if records.is_empty() {
        return transition_folder_to_staged(db, item_id).await;
    }
    if !import_batch::replace_archive_item_with_roots(db, item_id, &records).await? {
        return Err(AppError::Validation(format!(
            "Import item '{item_id}' changed while folder-pack staging completed"
        )));
    }
    Ok(())
}

async fn stage_archive_item(
    db: &SqlitePool,
    batch_id: &str,
    item_id: &str,
    source: &Path,
    fallback_name: &str,
    staging_root: &Path,
    options: &StagingExtractOptions,
) -> Result<(), AppError> {
    let extract_dir = staging_root
        .join(batch_id)
        .join(item_id)
        .join(uuid::Uuid::new_v4().to_string())
        .join("extracted");
    let archive = source.to_path_buf();
    let extract_for_worker = extract_dir.clone();
    let options = options.clone();
    let staged = tokio::task::spawn_blocking(move || {
        crate::modules::library::application::mods::archive::extract_archive_to_staging_with_options(
            &archive,
            &extract_for_worker,
            options,
        )
    })
    .await??;
    let mut roots = staged.mod_roots;
    roots.sort();
    let records = roots
        .into_iter()
        .map(|root| {
            let planned_name = if root == extract_dir {
                fallback_name.to_string()
            } else {
                root.file_name()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| fallback_name.to_string())
            };
            let planned_name =
                crate::modules::library::application::mods::core_ops::standardize_prefix(
                    &planned_name,
                    false,
                );
            StagedRootRecord {
                id: uuid::Uuid::new_v4().to_string(),
                staging_path: root.to_string_lossy().into_owned(),
                planned_name,
            }
        })
        .collect::<Vec<_>>();
    if !import_batch::replace_archive_item_with_roots(db, item_id, &records).await? {
        return Err(AppError::Validation(format!(
            "Import item '{item_id}' changed while archive staging completed"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod cleanup_tests {
    use super::{cleanup_batch_staging, cleanup_orphaned_staging};

    #[test]
    fn cleanup_removes_only_a_uuid_owned_batch_directory() {
        let root = tempfile::tempdir().unwrap();
        let batch_id = uuid::Uuid::new_v4().to_string();
        let owned = root.path().join(&batch_id);
        let sibling = root.path().join("keep-me");
        std::fs::create_dir_all(owned.join("item/extracted")).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();

        assert!(cleanup_batch_staging(root.path(), &batch_id).unwrap());
        assert!(!owned.exists());
        assert!(sibling.exists());
        assert!(!cleanup_batch_staging(root.path(), "../keep-me").unwrap());
        assert!(sibling.exists());
    }

    #[test]
    fn cleanup_orphans_preserves_referenced_attempts() {
        let root = tempfile::tempdir().unwrap();
        let batch_id = uuid::Uuid::new_v4().to_string();
        let item_id = uuid::Uuid::new_v4().to_string();
        let retained_attempt = root
            .path()
            .join(&batch_id)
            .join(&item_id)
            .join(uuid::Uuid::new_v4().to_string());
        let orphaned_attempt = root
            .path()
            .join(&batch_id)
            .join(&item_id)
            .join(uuid::Uuid::new_v4().to_string());
        let retained_root = retained_attempt.join("extracted/Mod");
        std::fs::create_dir_all(&retained_root).unwrap();
        std::fs::create_dir_all(orphaned_attempt.join("extracted")).unwrap();

        let removed =
            cleanup_orphaned_staging(root.path(), &[retained_root.to_string_lossy().into_owned()])
                .unwrap();

        assert_eq!(removed, 1);
        assert!(retained_attempt.exists());
        assert!(!orphaned_attempt.exists());
    }
}
