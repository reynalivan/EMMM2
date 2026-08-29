use super::types::{ImportBatch, ImportBatchStatus, ImportItemStatus};
use crate::shared::errors::AppError;
use crate::modules::ingestion::adapters::outbound::sqlite::import_batch::{self, StagedRootRecord};
use sqlx::SqlitePool;
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
        if item.status == ImportItemStatus::Failed {
            cleanup_item_staging(staging_root, batch_id, &item.id)?;
            if !import_batch::reset_failed_item_for_staging(db, &item.id).await? {
                return Err(AppError::Validation(format!(
                    "Import item '{}' changed while preparing a staging retry",
                    item.id
                )));
            }
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
            )
            .await
        } else {
            Err(AppError::Validation(format!(
                "Import source disappeared before staging: {}",
                source.display()
            )))
        };

        if let Err(error) = outcome {
            import_batch::set_item_failure(db, &item.id, &error.to_string()).await?;
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
            crate::modules::library::application::mods::archive::classify::find_mod_roots(&source_for_worker, 5);
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
                    planned_name: crate::modules::library::application::mods::core_ops::standardize_prefix(
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
) -> Result<(), AppError> {
    let extract_dir = staging_root
        .join(batch_id)
        .join(item_id)
        .join(uuid::Uuid::new_v4().to_string())
        .join("extracted");
    let archive = source.to_path_buf();
    let extract_for_worker = extract_dir.clone();
    let staged = tokio::task::spawn_blocking(move || {
        crate::modules::library::application::mods::archive::extract_archive_to_staging(&archive, &extract_for_worker)
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
                crate::modules::library::application::mods::core_ops::standardize_prefix(&planned_name, false);
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
    use super::cleanup_batch_staging;

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
}
