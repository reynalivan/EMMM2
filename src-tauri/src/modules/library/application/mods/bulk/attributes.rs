//! Bulk attribute updates: info.json fields plus favorite/pin flags.

use super::types::{BulkActionError, BulkResult};
use crate::modules::library::application::mods::info_json;
use crate::platform::fs::guard::ValidatedPath;
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

struct InfoWriteTarget {
    disk_path: PathBuf,
    reported_path: String,
}

impl From<&ValidatedPath> for InfoWriteTarget {
    fn from(path: &ValidatedPath) -> Self {
        Self {
            disk_path: path.to_path_buf(),
            reported_path: path.original().to_string(),
        }
    }
}

impl InfoWriteTarget {
    fn absolute(path: String) -> Self {
        Self {
            disk_path: PathBuf::from(&path),
            reported_path: path,
        }
    }
}

struct InfoWriteBackup {
    folder_path: String,
    previous: Option<Vec<u8>>,
}

#[derive(Clone, Copy)]
enum BackupRetention {
    Required,
    Discard,
}

struct InfoWriteBatch {
    result: BulkResult,
    backups: Vec<InfoWriteBackup>,
}

fn restore_info_writes(backups: &[InfoWriteBackup]) -> Vec<String> {
    let mut warnings = Vec::new();
    for backup in backups {
        let info_path = Path::new(&backup.folder_path).join("info.json");
        let restored = match backup.previous.as_deref() {
            Some(bytes) => crate::platform::fs::atomic_file::atomic_write(&info_path, bytes),
            None if info_path.exists() => std::fs::remove_file(&info_path).map_err(AppError::from),
            None => Ok(()),
        };
        if let Err(error) = restored {
            warnings.push(format!("{}: {error}", info_path.display()));
        }
    }
    warnings
}

fn write_info_batch(
    targets: Vec<InfoWriteTarget>,
    update: &info_json::ModInfoUpdate,
    backup_retention: BackupRetention,
) -> InfoWriteBatch {
    let mut success = Vec::new();
    let mut failures = Vec::new();
    let mut backups = Vec::new();
    for target in targets {
        let info_path = target.disk_path.join("info.json");
        let previous = match std::fs::read(&info_path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                failures.push(BulkActionError {
                    path: target.reported_path,
                    error: AppError::Io(error.to_string()),
                });
                continue;
            }
        };
        match info_json::update_info_json_from_snapshot(
            &target.disk_path,
            previous.as_deref(),
            update,
        ) {
            Ok(_) => {
                success.push(target.reported_path);
                if matches!(backup_retention, BackupRetention::Required) {
                    backups.push(InfoWriteBackup {
                        folder_path: target.disk_path.to_string_lossy().to_string(),
                        previous,
                    });
                }
            }
            Err(error) => failures.push(BulkActionError {
                path: target.reported_path,
                error: AppError::Metadata(error),
            }),
        }
    }
    InfoWriteBatch {
        result: BulkResult::new(success, failures),
        backups,
    }
}

async fn write_info_batch_in_worker(
    targets: Vec<InfoWriteTarget>,
    update: info_json::ModInfoUpdate,
    backup_retention: BackupRetention,
) -> Result<InfoWriteBatch, AppError> {
    tokio::task::spawn_blocking(move || write_info_batch(targets, &update, backup_retention))
        .await
        .map_err(AppError::from)
}

async fn restore_info_writes_in_worker(
    backups: Vec<InfoWriteBackup>,
) -> Result<Vec<String>, AppError> {
    tokio::task::spawn_blocking(move || restore_info_writes(&backups))
        .await
        .map_err(AppError::from)
}

pub async fn bulk_update_info(
    paths: &[crate::platform::fs::guard::ValidatedPath],
    update: info_json::ModInfoUpdate,
) -> Result<BulkResult, AppError> {
    let folder_paths = paths.iter().map(InfoWriteTarget::from).collect();
    Ok(
        write_info_batch_in_worker(folder_paths, update, BackupRetention::Discard)
            .await?
            .result,
    )
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
    selected_paths: &[crate::platform::fs::guard::ValidatedPath],
) -> Result<ResolvedSafetyTargets, AppError> {
    let mods_root = crate::modules::games::adapters::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found or has no mods_path".to_string()))?;
    let root = Path::new(&mods_root).canonicalize()?;
    let stored_paths =
        crate::modules::library::adapters::sqlite::mods::get_folder_paths_for_game(pool, game_id)
            .await?;
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
            if seen.insert(crate::shared::path_key::folder_path_key(
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
        .map(|target| InfoWriteTarget::absolute(target.disk_path.clone()))
        .collect::<Vec<_>>();
    let InfoWriteBatch {
        mut result,
        backups,
    } = write_info_batch_in_worker(disk_paths, update, BackupRetention::Required).await?;
    result.failures.extend(resolved.failures);

    let successful = result.success.iter().collect::<HashSet<_>>();
    let stored_paths = resolved
        .targets
        .iter()
        .filter(|target| successful.contains(&target.disk_path))
        .map(|target| target.stored_path.clone())
        .collect::<Vec<_>>();
    if let Err(error) = crate::modules::library::adapters::sqlite::mods::batch_set_safety(
        pool,
        game_id,
        &stored_paths,
        safe,
    )
    .await
    {
        let warnings = restore_info_writes_in_worker(backups).await?;
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
    paths: &[ValidatedPath],
    favorite: bool,
) -> Result<BulkResult, AppError> {
    let update = info_json::ModInfoUpdate {
        is_favorite: Some(favorite),
        ..Default::default()
    };
    let InfoWriteBatch { result, backups } = write_info_batch_in_worker(
        paths.iter().map(InfoWriteTarget::from).collect(),
        update,
        BackupRetention::Required,
    )
    .await?;
    let relatives = relative_to_mods_root(pool, &game_id, paths, &result.success).await?;
    if let Err(error) = crate::modules::library::adapters::sqlite::mods::batch_set_favorite(
        pool, &game_id, &relatives, favorite,
    )
    .await
    {
        let warnings = restore_info_writes_in_worker(backups).await?;
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
    paths: &[ValidatedPath],
    pin: bool,
) -> Result<BulkResult, AppError> {
    let update = info_json::ModInfoUpdate {
        is_pinned: Some(pin),
        ..Default::default()
    };
    let InfoWriteBatch { result, backups } = write_info_batch_in_worker(
        paths.iter().map(InfoWriteTarget::from).collect(),
        update,
        BackupRetention::Required,
    )
    .await?;
    let relatives = relative_to_mods_root(pool, &game_id, paths, &result.success).await?;
    if let Err(error) = crate::modules::library::adapters::sqlite::mods::batch_set_pinned(
        pool, &game_id, &relatives, pin,
    )
    .await
    {
        let warnings = restore_info_writes_in_worker(backups).await?;
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
    paths: &[ValidatedPath],
    successful_paths: &[String],
) -> Result<Vec<String>, AppError> {
    let game_mod_path = crate::modules::games::adapters::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found or has no mods_path".to_string()))?;

    let base = Path::new(&game_mod_path).canonicalize()?;
    let successes: HashSet<&str> = successful_paths.iter().map(String::as_str).collect();
    paths
        .iter()
        .filter(|path| successes.contains(path.original()))
        .map(|path| {
            path.strip_prefix(&base)
                .map(|relative| relative.to_string_lossy().to_string())
                .map_err(|_| {
                    AppError::Security(
                        "Validated metadata path does not belong to the game's Mods root"
                            .to_string(),
                    )
                })
        })
        .collect()
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
    write_info_batch(
        folder_paths
            .into_iter()
            .map(InfoWriteTarget::absolute)
            .collect(),
        update,
        BackupRetention::Discard,
    )
    .result
}

#[cfg(test)]
#[path = "../tests/bulk_attributes_tests.rs"]
mod tests;
