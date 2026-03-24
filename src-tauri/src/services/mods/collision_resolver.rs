use crate::domain::errors::AppError;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::core::types::{CollisionInfo, CollisionResolution};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use sqlx::SqlitePool;
use std::path::Path;

// Service for resolving folder name collisions during move/toggle operations.
pub async fn resolve_collision_service(
    pool: &SqlitePool,
    watcher: &WatcherState,
    op_lock: &OperationLock,
    _game_id: &str,
    collision: CollisionInfo,
    resolution: CollisionResolution,
) -> Result<String, AppError> {
    let _lock = op_lock.acquire().await.map_err(AppError::Internal)?;
    let _guard = SuppressionGuard::new(&watcher.suppressor);

    let src = Path::new(&collision.source_path);
    let tgt = Path::new(&collision.target_path);

    if !src.exists() {
        return Err(AppError::NotFound(format!(
            "Source path not found: {}",
            collision.source_path
        )));
    }

    match resolution {
        CollisionResolution::Skip => Ok(collision.source_path),
        CollisionResolution::Overwrite => {
            if tgt.exists() {
                if tgt.is_dir() {
                    std::fs::remove_dir_all(tgt).map_err(|e| AppError::Io(e.to_string()))?;
                } else {
                    std::fs::remove_file(tgt).map_err(|e| AppError::Io(e.to_string()))?;
                }
            }
            crate::services::fs_utils::file_utils::rename_cross_drive_fallback(src, tgt)
                .map_err(|e| AppError::Io(e.to_string()))?;

            // Sync DB
            let _ = crate::services::collection_service::handle_mod_moved_or_renamed(
                pool,
                &collision.source_path,
                &collision.target_path,
                None,
            )
            .await;

            Ok(collision.target_path)
        }
        CollisionResolution::Rename => {
            let mut suffix = 2;
            let mut new_tgt = tgt.to_path_buf();
            while new_tgt.exists() {
                let stem = tgt.file_stem().unwrap_or_default().to_string_lossy();
                let ext = tgt
                    .extension()
                    .map(|e| format!(".{}", e.to_string_lossy()))
                    .unwrap_or_default();

                let new_name = format!("{} ({}){}", stem, suffix, ext);
                new_tgt = tgt.parent().unwrap_or(Path::new("")).join(new_name);
                suffix += 1;
            }

            crate::services::fs_utils::file_utils::rename_cross_drive_fallback(src, &new_tgt)
                .map_err(|e| AppError::Io(e.to_string()))?;

            let new_path_str = new_tgt.to_string_lossy().to_string();
            // Sync DB
            let _ = crate::services::collection_service::handle_mod_moved_or_renamed(
                pool,
                &collision.source_path,
                &new_path_str,
                None,
            )
            .await;

            Ok(new_path_str)
        }
        CollisionResolution::Merge => {
            // Merge is complex, requirement says "if both are folders".
            // For now, let's treat it as Overwrite but we should ideally merge files.
            // Requirement 39.2.1 mentions Merge. Let's do a simple recursive copy/move.
            merge_folders(src, tgt).map_err(|e| AppError::Io(e))?;

            // Sync DB: src is gone
            let _ = crate::repo::mod_repo::delete_mod_by_path(pool, &collision.source_path).await;

            Ok(collision.target_path)
        }
    }
}

fn merge_folders(src: &Path, tgt: &Path) -> Result<(), String> {
    if !tgt.exists() {
        std::fs::create_dir_all(tgt).map_err(|e| e.to_string())?;
    }

    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let dest = tgt.join(entry.file_name());

        if entry.path().is_dir() {
            merge_folders(&entry.path(), &dest)?;
        } else {
            if dest.exists() {
                std::fs::remove_file(&dest).map_err(|e| e.to_string())?;
            }
            std::fs::rename(entry.path(), dest).map_err(|e| e.to_string())?;
        }
    }

    let _ = std::fs::remove_dir_all(src);
    Ok(())
}
