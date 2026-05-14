use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::PathGuard;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::WatcherState;
use std::path::Path;
use tauri::State;

// Re-export from services layer for backward compat (tests use `super::*`)
pub use crate::services::mods::core_ops::{
    rename_mod_folder_inner, standardize_prefix, toggle_mod_inner, RenameResult,
};

#[specta::specta]
#[tauri::command]
pub async fn open_in_explorer(
    config: State<'_, ConfigService>,
    game_id: String,
    path: String,
) -> Result<(), AppError> {
    let canonical_path =
        PathGuard::validate_path(&config, &game_id, &path).map_err(AppError::Security)?;
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(canonical_path)
            .spawn()
            .map_err(|e| AppError::Io(format!("Failed to open explorer: {}", e)))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    Err(AppError::Io(
        "Open in explorer only supported on Windows".to_string(),
    ))
}

#[specta::specta]
#[tauri::command]
pub async fn reveal_object_in_explorer(
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    object_id: String,
    object_name: String,
) -> Result<String, AppError> {
    let mods_path = crate::repo::game_repo::get_mod_path(pool.inner(), &game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;

    if let Some(path_str) = resolve_and_heal_db_path(pool.inner(), &object_id).await {
        let canonical =
            PathGuard::validate_path(&config, &game_id, &path_str).map_err(AppError::Security)?;
        return open_explorer_select(&canonical.to_string_lossy());
    }

    let candidate_path = find_fallback_path(&mods_path, &object_name)?;
    let canonical =
        PathGuard::validate_path(&config, &game_id, &candidate_path).map_err(AppError::Security)?;
    open_explorer_select(&canonical.to_string_lossy())
}

async fn resolve_and_heal_db_path(pool: &sqlx::SqlitePool, object_id: &str) -> Option<String> {
    crate::services::mods::stale_mod_service::resolve_mod_path_for_object(pool, object_id).await
}

fn find_fallback_path(mods_path: &str, object_name: &str) -> Result<String, AppError> {
    let mods_dir = Path::new(mods_path);
    if !mods_dir.exists() || !mods_dir.is_dir() {
        return Err(AppError::Io(
            "Could not find any folder to reveal".to_string(),
        ));
    }

    let candidate = mods_dir.join(object_name);
    if candidate.exists() {
        return Ok(candidate.to_string_lossy().to_string());
    }

    let disabled_candidate = mods_dir.join(format!("{}{}", crate::DISABLED_PREFIX, object_name));
    if disabled_candidate.exists() {
        return Ok(disabled_candidate.to_string_lossy().to_string());
    }

    Ok(mods_path.to_string())
}

fn open_explorer_select(path: &str) -> Result<String, AppError> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", path])
            .spawn()
            .map_err(|e| AppError::Io(format!("Failed to open explorer: {}", e)))?;
        Ok(path.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    Err(AppError::Io(
        "Reveal in explorer only supported on Windows".to_string(),
    ))
}

#[specta::specta]
#[tauri::command]
pub async fn rename_mod_folder(
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    folder_path: String,
    new_name: String,
    game_id: String,
) -> Result<RenameResult, AppError> {
    let result = crate::services::mods::core_ops::rename_mod_folder_inner_service(
        &config,
        pool.inner(),
        &state,
        &op_lock,
        folder_path.clone(),
        new_name.clone(),
        &game_id,
    )
    .await?;

    Ok(result)
}

#[cfg(test)]
#[path = "tests/mod_core_cmds_tests.rs"]
mod tests;
