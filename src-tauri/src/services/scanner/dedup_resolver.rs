use crate::services::file_ops::trash;
use crate::services::operation_lock::OperationLock;
use crate::services::watcher::SuppressionGuard;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionRequest {
    pub group_id: String,
    pub action: ResolutionAction,
    pub folder_a: String,
    pub folder_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResolutionAction {
    KeepA,
    KeepB,
    Ignore,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionSummary {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub errors: Vec<ResolutionError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionError {
    pub group_id: String,
    pub action: ResolutionAction,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionProgress {
    pub current: usize,
    pub total: usize,
    pub group_id: String,
    pub action: ResolutionAction,
}

pub async fn resolve_batch<F>(
    requests: Vec<ResolutionRequest>,
    game_id: String,
    db: &SqlitePool,
    op_lock: &OperationLock,
    watcher_suppressor: &Arc<AtomicBool>,
    trash_dir: &Path,
    mut on_progress: F,
) -> Result<ResolutionSummary, String>
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

    let _lock = op_lock
        .acquire()
        .await
        .map_err(|error| format!("Operation in progress: {error}"))?;

    if !trash_dir.exists() {
        fs::create_dir_all(trash_dir)
            .map_err(|error| format!("Failed to create trash directory: {error}"))?;
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

        let outcome = resolve_one(request, &game_id, db, trash_dir).await;
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
                    message,
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
    trash_dir: &Path,
) -> Result<(), String> {
    match request.action {
        ResolutionAction::KeepA => {
            move_folder_to_trash(&request.folder_b, game_id, trash_dir)?;
            set_group_status(db, &request.group_id, "resolved").await?;
            Ok(())
        }
        ResolutionAction::KeepB => {
            move_folder_to_trash(&request.folder_a, game_id, trash_dir)?;
            set_group_status(db, &request.group_id, "resolved").await?;
            Ok(())
        }
        ResolutionAction::Ignore => {
            persist_whitelist_pair(db, game_id, &request.folder_a, &request.folder_b).await?;
            set_group_status(db, &request.group_id, "ignored").await?;
            Ok(())
        }
    }
}

fn move_folder_to_trash(folder_path: &str, game_id: &str, trash_dir: &Path) -> Result<(), String> {
    let source_path = Path::new(folder_path);
    trash::move_to_trash(source_path, trash_dir, Some(game_id.to_string())).map(|_| ())
}

async fn persist_whitelist_pair(
    db: &SqlitePool,
    game_id: &str,
    folder_a_path: &str,
    folder_b_path: &str,
) -> Result<(), String> {
    let folder_a_id = fetch_mod_id(db, game_id, folder_a_path).await?;
    let folder_b_id = fetch_mod_id(db, game_id, folder_b_path).await?;

    if folder_a_id == folder_b_id {
        return Err("Whitelist pair must reference two different folders".to_string());
    }

    let (canonical_a, canonical_b) = canonicalize_pair(&folder_a_id, &folder_b_id);

    sqlx::query(
        "INSERT OR IGNORE INTO duplicate_whitelist (id, game_id, folder_a_id, folder_b_id, reason)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(game_id)
    .bind(canonical_a)
    .bind(canonical_b)
    .bind("Manual duplicate ignore")
    .execute(db)
    .await
    .map_err(|error| format!("Failed to persist duplicate whitelist pair: {error}"))?;

    Ok(())
}

async fn fetch_mod_id(db: &SqlitePool, game_id: &str, folder_path: &str) -> Result<String, String> {
    sqlx::query_scalar::<_, String>("SELECT id FROM mods WHERE game_id = ? AND folder_path = ?")
        .bind(game_id)
        .bind(folder_path)
        .fetch_optional(db)
        .await
        .map_err(|error| format!("Failed to resolve mod id for '{folder_path}': {error}"))?
        .ok_or_else(|| {
            format!("Mod entry not found for game '{game_id}' and folder '{folder_path}'")
        })
}

async fn set_group_status(db: &SqlitePool, group_id: &str, status: &str) -> Result<(), String> {
    let query = if status == "resolved" || status == "ignored" {
        "UPDATE dedup_groups SET resolution_status = ?, resolved_at = CURRENT_TIMESTAMP WHERE id = ?"
    } else {
        "UPDATE dedup_groups SET resolution_status = ? WHERE id = ?"
    };

    let result = sqlx::query(query)
        .bind(status)
        .bind(group_id)
        .execute(db)
        .await
        .map_err(|error| format!("Failed to update resolution status for group '{group_id}': {error}"))?;

    if result.rows_affected() == 0 {
        log::warn!(
            "No dedup_groups row updated for group_id='{}' and status='{}'",
            group_id,
            status
        );
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
mod tests;
