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
            Ok(res) => success.push(res.new_path.to_string_lossy().to_string()),
            Err(e) => failures.push(BulkActionError {
                path: path_str,
                error: e,
            }),
        }
    }

    Ok(BulkResult { success, failures })
}
