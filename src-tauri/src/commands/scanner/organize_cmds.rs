//! Commands for reorganizing mod folders using Master DB info.

use crate::services::mods::bulk::BulkResult;
use crate::services::scanner::watcher::WatcherState;
use tauri::State;

/// Bulk Auto-Organize mods.
/// Moves selected mods to `Mods/{Category}/{ObjectName}/{ModName}`.
#[tauri::command]
#[specta::specta]
pub async fn auto_organize_mods(
    config: tauri::State<'_, crate::services::config::ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    paths: Vec<String>,
    db_json: String,
    watcher: State<'_, WatcherState>,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    crate::services::mods::organizer_ext::auto_organize_mods_service(
        &config,
        pool.inner(),
        &game_id,
        paths,
        &db_json,
        &watcher,
    )
    .await
}

#[cfg(test)]
#[path = "tests/organize_cmds_tests.rs"]
mod tests;
