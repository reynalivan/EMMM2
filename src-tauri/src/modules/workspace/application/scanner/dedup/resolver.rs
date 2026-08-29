use crate::shared::errors::AppError;
use crate::shared::errors::ScannerError;
use crate::modules::library::application::mods::trash;
use crate::modules::workspace::application::scanner::watcher::{SuppressionGuard, WatcherSuppressor};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod hardlink;

#[cfg(test)]
pub(crate) use hardlink::replace_file_with_hardlink_using;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionRequest {
    pub group_id: String,
    pub action: ResolutionAction,
    pub folder_a: String,
    pub folder_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ResolutionAction {
    KeepA,
    KeepB,
    Ignore,
    Hardlink,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionSummary {
    #[specta(type = f64)]
    pub total: usize,
    #[specta(type = f64)]
    pub successful: usize,
    #[specta(type = f64)]
    pub failed: usize,
    pub errors: Vec<ResolutionError>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionError {
    pub group_id: String,
    pub action: ResolutionAction,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionProgress {
    #[specta(type = f64)]
    pub current: usize,
    #[specta(type = f64)]
    pub total: usize,
    pub group_id: String,
    pub action: ResolutionAction,
}

pub async fn resolve_batch<F>(
    requests: Vec<ResolutionRequest>,
    game_id: String,
    db: &SqlitePool,
    _op_guard: &crate::platform::fs::operation_lock::OpGuard,
    watcher_suppressor: &Arc<WatcherSuppressor>,
    mut on_progress: F,
) -> Result<ResolutionSummary, AppError>
where
    F: FnMut(ResolutionProgress),
{
    if requests.is_empty() {
        return Ok(ResolutionSummary {
            total: 0,
            successful: 0,
            failed: 0,
            errors: Vec::new(),
        });
    }

    let _suppression_guard = SuppressionGuard::new(watcher_suppressor);

    let total = requests.len();
    let mut successful = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();

    for (index, request) in requests.iter().enumerate() {
        on_progress(ResolutionProgress {
            current: index + 1,
            total,
            group_id: request.group_id.clone(),
            action: request.action.clone(),
        });

        let outcome = resolve_one(request, &game_id, db).await;
        match outcome {
            Ok(()) => {
                successful += 1;
            }
            Err(message) => {
                failed += 1;
                let _ = set_group_status(db, &request.group_id, "partial").await;
                errors.push(ResolutionError {
                    group_id: request.group_id.clone(),
                    action: request.action.clone(),
                    message: message.to_string(),
                });
            }
        }
    }

    Ok(ResolutionSummary {
        total,
        successful,
        failed,
        errors,
    })
}

async fn resolve_one(
    request: &ResolutionRequest,
    game_id: &str,
    db: &SqlitePool,
) -> Result<(), ScannerError> {
    authorize_request(request, game_id, db).await?;

    match request.action {
        ResolutionAction::KeepA => {
            verify_full_folder_match(&request.folder_a, &request.folder_b)?;
            move_folder_to_trash(&request.folder_b)?;
            set_group_status(db, &request.group_id, "resolved").await?;
            Ok(())
        }
        ResolutionAction::KeepB => {
            verify_full_folder_match(&request.folder_a, &request.folder_b)?;
            move_folder_to_trash(&request.folder_a)?;
            set_group_status(db, &request.group_id, "resolved").await?;
            Ok(())
        }
        ResolutionAction::Ignore => {
            persist_whitelist_pair(db, game_id, &request.folder_a, &request.folder_b).await?;
            set_group_status(db, &request.group_id, "ignored").await?;
            Ok(())
        }
        ResolutionAction::Hardlink => {
            verify_full_folder_match(&request.folder_a, &request.folder_b)?;
            hardlink::apply_hardlinks(&request.folder_a, &request.folder_b)?;
            set_group_status(db, &request.group_id, "resolved").await?;
            Ok(())
        }
    }
}

async fn authorize_request(
    request: &ResolutionRequest,
    game_id: &str,
    db: &SqlitePool,
) -> Result<(), ScannerError> {
    let requested_paths = canonical_request_paths(request)?;
    let group = crate::modules::storage_optimizer::adapters::outbound::sqlite::dedup::load_pending_group(db, game_id, &request.group_id)
        .await?
        .ok_or_else(|| {
            ScannerError::Validation(format!(
                "Duplicate group is missing, stale, or already resolved: {}",
                request.group_id
            ))
        })?;
    let member_paths = group_member_paths(&group)?;
    if !member_paths.contains(&requested_paths.0) || !member_paths.contains(&requested_paths.1) {
        return Err(ScannerError::Validation(
            "Resolution paths are not members of the persisted duplicate group".to_string(),
        ));
    }
    if !matches!(request.action, ResolutionAction::Ignore) && group.confidence_score != 100 {
        return Err(ScannerError::Validation(
            "Destructive resolution requires a fully verified exact duplicate group".to_string(),
        ));
    }
    Ok(())
}

fn canonical_request_paths(
    request: &ResolutionRequest,
) -> Result<(PathBuf, PathBuf), ScannerError> {
    let left = canonical_folder_path(&request.folder_a)?;
    let right = canonical_folder_path(&request.folder_b)?;
    if left == right {
        return Err(ScannerError::Validation(
            "Duplicate resolution requires two distinct physical folders".to_string(),
        ));
    }
    Ok((left, right))
}

fn group_member_paths(
    group: &crate::types::dup_scan::DupScanGroup,
) -> Result<Vec<PathBuf>, ScannerError> {
    group
        .members
        .iter()
        .map(|member| canonical_folder_path(&member.folder_path))
        .collect()
}

fn canonical_folder_path(folder_path: &str) -> Result<PathBuf, ScannerError> {
    std::fs::canonicalize(folder_path).map_err(|error| {
        ScannerError::Io(format!(
            "failed to resolve duplicate folder '{folder_path}': {error}"
        ))
    })
}

fn verify_full_folder_match(left: &str, right: &str) -> Result<(), ScannerError> {
    let left_manifest = full_folder_manifest(Path::new(left))?;
    let right_manifest = full_folder_manifest(Path::new(right))?;

    if left_manifest != right_manifest {
        return Err(ScannerError::Validation(
            "Folders are not full-content duplicates; rescan before resolving".to_string(),
        ));
    }

    Ok(())
}

fn full_folder_manifest(folder: &Path) -> Result<BTreeMap<String, (u64, String)>, ScannerError> {
    if !folder.is_dir() {
        return Err(ScannerError::Validation(format!(
            "Duplicate folder does not exist: {}",
            folder.display()
        )));
    }

    let mut manifest = BTreeMap::new();
    for item in walkdir::WalkDir::new(folder)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || (!entry.file_type().is_dir()
                    || !entry.file_name().to_string_lossy().starts_with('.'))
        })
    {
        let entry = item.map_err(|error| ScannerError::Io(error.to_string()))?;
        if entry.depth() == 0 || entry.file_type().is_dir() {
            continue;
        }
        if super::snapshot::is_ignored_file_name(entry.file_name()) {
            continue;
        }
        if !entry.file_type().is_file() {
            return Err(ScannerError::Validation(format!(
                "Unsupported link or special file in duplicate folder: {}",
                entry.path().display()
            )));
        }

        let relative = entry
            .path()
            .strip_prefix(folder)
            .map_err(|error| ScannerError::Io(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        let size = entry
            .metadata()
            .map_err(|error| ScannerError::Io(error.to_string()))?
            .len();
        let hash = super::hashing::full_blake3_hash(entry.path())?;
        manifest.insert(relative, (size, hash));
    }

    Ok(manifest)
}

fn move_folder_to_trash(folder_path: &str) -> Result<(), ScannerError> {
    let source_path = Path::new(folder_path);
    trash::move_to_trash(source_path).map_err(|error| ScannerError::Io(error.to_string()))
}

async fn persist_whitelist_pair(
    db: &SqlitePool,
    game_id: &str,
    folder_a_path: &str,
    folder_b_path: &str,
) -> Result<(), ScannerError> {
    let folder_a_id = fetch_mod_id(db, game_id, folder_a_path).await?;
    let folder_b_id = fetch_mod_id(db, game_id, folder_b_path).await?;

    if folder_a_id == folder_b_id {
        return Err(ScannerError::Validation(
            "Whitelist pair must reference two different folders".to_string(),
        ));
    }

    let (canonical_a, canonical_b) = canonicalize_pair(&folder_a_id, &folder_b_id);

    crate::modules::storage_optimizer::adapters::outbound::sqlite::dedup::insert_whitelist_pair(db, game_id, canonical_a, canonical_b).await?;

    Ok(())
}

async fn fetch_mod_id(
    db: &SqlitePool,
    game_id: &str,
    folder_path: &str,
) -> Result<String, ScannerError> {
    crate::modules::library::adapters::outbound::sqlite::mods::get_mod_id_and_status_by_path(db, folder_path, game_id)
        .await?
        .map(|(id, _, _)| id)
        .ok_or_else(|| {
            ScannerError::Validation(format!(
                "mod entry not found for game '{game_id}' and folder '{folder_path}'"
            ))
        })
}

async fn set_group_status(
    db: &SqlitePool,
    group_id: &str,
    status: &str,
) -> Result<(), ScannerError> {
    let set_resolved_at = status == "resolved" || status == "ignored";
    let rows_affected =
        crate::modules::storage_optimizer::adapters::outbound::sqlite::dedup::update_group_status(db, group_id, status, set_resolved_at).await?;

    if rows_affected == 0 {
        return Err(ScannerError::Validation(format!(
            "Duplicate group is missing or stale: {group_id}"
        )));
    }

    Ok(())
}

fn canonicalize_pair<'a>(left: &'a str, right: &'a str) -> (&'a str, &'a str) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

#[cfg(test)]
#[path = "tests/dedup_resolver_tests.rs"]
mod tests;
