//! Commands for reorganizing mod folders using Master DB info.

use crate::commands::mods::mod_bulk_cmds::{BulkActionError, BulkResult};
use crate::services::scanner::deep_matcher::MasterDb;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::path::Path;
use tauri::State;

/// Bulk Auto-Organize mods.
/// Moves selected mods to `Mods/{Category}/{ObjectName}/{ModName}`.
#[tauri::command]
pub async fn auto_organize_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    paths: Vec<String>,
    target_root: String,
    db_json: String,
    watcher: State<'_, WatcherState>,
) -> Result<BulkResult, String> {
    use crate::services::scanner::core::organizer;

    let db = MasterDb::from_json(&db_json)?;
    let root = Path::new(&target_root);
    let mut success = Vec::new();
    let mut failures = Vec::new();

    for path_str in paths {
        let path = Path::new(&path_str);

        // Suppress watcher
        let _guard = SuppressionGuard::new(&watcher.suppressor);

        match organizer::organize_mod(path, root, &db) {
            Ok(res) => {
                let new_path = res.new_path.to_string_lossy().to_string();
                // Sync DB: update folder_path to new location
                let _ = sqlx::query(
                    "UPDATE mods SET folder_path = ? WHERE folder_path = ?",
                )
                .bind(&new_path)
                .bind(&path_str)
                .execute(&*pool)
                .await;
                success.push(new_path);
            }
            Err(e) => {
                let error_str = match e {
                    organizer::OrganizeError::Duplicate { dest } => format!("DUPLICATE|{}", dest),
                    organizer::OrganizeError::Generic(msg) => msg,
                };
                failures.push(BulkActionError {
                    path: path_str,
                    error: error_str,
                });
            }
        }
    }

    Ok(BulkResult { success, failures })
}
