//! Bulk attribute updates: info.json fields plus favorite/pin flags.

use super::types::{BulkActionError, BulkResult};
use crate::domain::errors::AppError;
use crate::services::mods::info_json;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::Path;

struct InfoWriteBackup {
    folder_path: String,
    previous: Option<Vec<u8>>,
}

fn restore_info_writes(backups: &[InfoWriteBackup]) -> Vec<String> {
    let mut warnings = Vec::new();
    for backup in backups {
        let info_path = Path::new(&backup.folder_path).join("info.json");
        let restored = match backup.previous.as_deref() {
            Some(bytes) => crate::services::fs_utils::atomic_file::atomic_write(&info_path, bytes),
            None if info_path.exists() => std::fs::remove_file(&info_path).map_err(AppError::from),
            None => Ok(()),
        };
        if let Err(error) = restored {
            warnings.push(format!("{}: {error}", info_path.display()));
        }
    }
    warnings
}

fn write_info_with_backups(
    folder_paths: Vec<String>,
    update: &info_json::ModInfoUpdate,
) -> (BulkResult, Vec<InfoWriteBackup>) {
    let mut success = Vec::new();
    let mut failures = Vec::new();
    let mut backups = Vec::new();
    for folder_path in folder_paths {
        let info_path = Path::new(&folder_path).join("info.json");
        let previous = match std::fs::read(&info_path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                failures.push(BulkActionError {
                    path: folder_path,
                    error: AppError::Io(error.to_string()),
                });
                continue;
            }
        };
        match info_json::update_info_json(Path::new(&folder_path), update) {
            Ok(_) => {
                success.push(folder_path.clone());
                backups.push(InfoWriteBackup {
                    folder_path,
                    previous,
                });
            }
            Err(error) => failures.push(BulkActionError {
                path: folder_path,
                error: AppError::Metadata(error),
            }),
        }
    }
    (BulkResult::new(success, failures), backups)
}

pub async fn bulk_update_info(
    paths: &[crate::services::fs_utils::guard::ValidatedPath],
    update: info_json::ModInfoUpdate,
) -> Result<BulkResult, AppError> {
    let mut success = Vec::new();
    let mut failures = Vec::new();
    for path in paths {
        match info_json::update_info_json(path, &update) {
            Ok(_) => success.push(path.original().to_string()),
            Err(e) => failures.push(BulkActionError {
                path: path.original().to_string(),
                error: AppError::Metadata(e),
            }),
        }
    }
    Ok(BulkResult::new(success, failures))
}

#[derive(Debug)]
pub struct SafetyTarget {
    pub disk_path: String,
    pub stored_path: String,
}

#[derive(Debug)]
pub struct ResolvedSafetyTargets {
    pub targets: Vec<SafetyTarget>,
    pub failures: Vec<BulkActionError>,
}

/// Expand selected terminal mods or parent containers to indexed leaf mods.
/// The database projection decides what a mod is; this avoids treating object
/// folders as mods and keeps one traversal for deeply nested objects.
pub async fn resolve_safety_targets(
    pool: &SqlitePool,
    game_id: &str,
    selected_paths: &[crate::services::fs_utils::guard::ValidatedPath],
) -> Result<ResolvedSafetyTargets, AppError> {
    let mods_root = crate::repo::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found or has no mods_path".to_string()))?;
    let root = Path::new(&mods_root).canonicalize()?;
    let stored_paths = crate::repo::mods::get_folder_paths_for_game(pool, game_id).await?;
    let candidates = stored_paths
        .into_iter()
        .filter_map(|stored_path| {
            let stored = Path::new(&stored_path);
            let absolute = if stored.is_absolute() {
                stored.to_path_buf()
            } else {
                root.join(stored)
            };
            absolute
                .canonicalize()
                .ok()
                .map(|disk_path| (stored_path, disk_path))
        })
        .collect::<Vec<_>>();

    let mut seen = HashSet::new();
    let mut targets = Vec::new();
    let mut failures = Vec::new();
    for selected in selected_paths {
        let mut matched = false;
        for (stored_path, disk_path) in &candidates {
            if !disk_path.starts_with(selected.as_ref()) {
                continue;
            }
            matched = true;
            let disk_path_string = disk_path.to_string_lossy().to_string();
            if seen.insert(crate::common::path_key::folder_path_key(
                &disk_path_string,
                None,
            )) {
                targets.push(SafetyTarget {
                    disk_path: disk_path_string,
                    stored_path: stored_path.clone(),
                });
            }
        }
        if !matched {
            failures.push(BulkActionError {
                path: selected.original().to_string(),
                error: AppError::NotFound(
                    "No indexed mod exists at or below this folder".to_string(),
                ),
            });
        }
    }

    Ok(ResolvedSafetyTargets { targets, failures })
}

pub async fn bulk_set_safety(
    pool: &SqlitePool,
    game_id: &str,
    resolved: ResolvedSafetyTargets,
    safe: bool,
) -> Result<BulkResult, AppError> {
    let update = info_json::ModInfoUpdate {
        is_safe: Some(safe),
        ..Default::default()
    };
    let disk_paths = resolved
        .targets
        .iter()
        .map(|target| target.disk_path.clone())
        .collect::<Vec<_>>();
    let (mut result, backups) = write_info_with_backups(disk_paths, &update);
    result.failures.extend(resolved.failures);

    let successful = result.success.iter().collect::<HashSet<_>>();
    let stored_paths = resolved
        .targets
        .iter()
        .filter(|target| successful.contains(&target.disk_path))
        .map(|target| target.stored_path.clone())
        .collect::<Vec<_>>();
    if let Err(error) =
        crate::repo::mods::batch_set_safety(pool, game_id, &stored_paths, safe).await
    {
        let warnings = restore_info_writes(&backups);
        return Err(AppError::Io(format!(
            "Safety database update failed: {error}; file rollback: {}",
            if warnings.is_empty() {
                "ok".to_string()
            } else {
                warnings.join("; ")
            }
        )));
    }

    Ok(result)
}

pub async fn bulk_toggle_favorite(
    pool: &SqlitePool,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<BulkResult, AppError> {
    let update = info_json::ModInfoUpdate {
        is_favorite: Some(favorite),
        ..Default::default()
    };
    let (result, backups) = write_info_with_backups(folder_paths, &update);
    let relatives = relative_to_mods_root(pool, &game_id, &result.success).await?;
    if let Err(error) =
        crate::repo::mods::batch_set_favorite(pool, &game_id, &relatives, favorite).await
    {
        let warnings = restore_info_writes(&backups);
        return Err(AppError::Io(format!(
            "Favorite database update failed: {error}; file rollback: {}",
            if warnings.is_empty() {
                "ok".to_string()
            } else {
                warnings.join("; ")
            }
        )));
    }
    Ok(result)
}

pub async fn bulk_pin(
    pool: &SqlitePool,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<BulkResult, AppError> {
    let update = info_json::ModInfoUpdate {
        is_pinned: Some(pin),
        ..Default::default()
    };
    let (result, backups) = write_info_with_backups(folder_paths, &update);
    let relatives = relative_to_mods_root(pool, &game_id, &result.success).await?;
    if let Err(error) =
        crate::repo::mods::batch_set_pinned(pool, &game_id, &relatives, pin).await
    {
        let warnings = restore_info_writes(&backups);
        return Err(AppError::Io(format!(
            "Pin database update failed: {error}; file rollback: {}",
            if warnings.is_empty() {
                "ok".to_string()
            } else {
                warnings.join("; ")
            }
        )));
    }
    Ok(result)
}

/// Mod paths relative to the game's mods root, the form the DB stores.
async fn relative_to_mods_root(
    pool: &SqlitePool,
    game_id: &str,
    folder_paths: &[String],
) -> Result<Vec<String>, AppError> {
    let game_mod_path = crate::repo::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found or has no mods_path".to_string()))?;

    let base = Path::new(&game_mod_path);
    Ok(folder_paths
        .iter()
        .map(|folder_path| {
            let path = Path::new(folder_path);
            path.strip_prefix(base)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string()
        })
        .collect())
}

/// Mirror a flag into every mod's info.json in parallel, splitting the batch into
/// written and not-written. A folder that vanished mid-batch (renamed by a toggle,
/// deleted on disk) lands in `failures`: the DB row is already flagged, so dropping
/// the write error would leave disk and DB disagreeing with nothing to show for it.
#[cfg(test)]
fn partition_info_json_writes(
    folder_paths: Vec<String>,
    update: &info_json::ModInfoUpdate,
) -> BulkResult {
    write_info_with_backups(folder_paths, update).0
}

#[cfg(test)]
#[path = "../tests/bulk_attributes_tests.rs"]
mod tests;
