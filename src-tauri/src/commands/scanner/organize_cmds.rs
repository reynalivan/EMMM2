//! Commands for reorganizing mod folders using Master DB info.

use crate::services::mods::bulk::BulkResult;
use crate::services::scanner::watcher::WatcherState;
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
    crate::services::mods::organizer_ext::auto_organize_mods_service(
        &pool,
        paths,
        target_root,
        db_json,
        &watcher,
    )
    .await
}

#[cfg(test)]
#[path = "tests/organize_cmds_tests.rs"]
mod tests;
