use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::disk_reconcile::emit::emit_internal_disk_reconcile;
use crate::services::fs_utils::guard::validate_path;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::{info_json, metadata};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};

#[specta::specta]
#[tauri::command]
pub async fn toggle_mod_safe(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    safe: bool,
) -> Result<(), AppError> {
    let folder = validate_path(&config, &game_id, &folder_path)?;
    metadata::toggle_mod_safe(&config, pool.inner(), &watcher, &game_id, &folder, safe).await?;
    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn suggest_random_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    config: tauri::State<'_, ConfigService>,
    game_id: String,
) -> Result<Vec<metadata::RandomModProposal>, AppError> {
    metadata::suggest_random_mods(pool.inner(), &game_id, config.current_corridor()).await
}

#[specta::specta]
#[tauri::command]
pub async fn get_active_mod_conflicts(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<crate::services::scanner::conflict::ConflictInfo>, AppError> {
    metadata::get_active_mod_conflicts(pool.inner(), &game_id).await
}

#[specta::specta]
#[tauri::command]
pub async fn read_mod_info(
    config: tauri::State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<Option<info_json::ModInfo>, AppError> {
    let path = validate_path(&config, &game_id, &folder_path)?;
    Ok(info_json::read_info_json(&path)?)
}

#[specta::specta]
#[tauri::command]
pub async fn update_mod_info(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    update: info_json::ModInfoUpdate,
) -> Result<info_json::ModInfo, AppError> {
    let path = validate_path(&config, &game_id, &folder_path)?;
    let changed_path = path.join("info.json").to_string_lossy().to_string();
    let _guard = SuppressionGuard::new(&state.suppressor);
    let info = info_json::update_info_json(&path, &update)?;
    emit_internal_disk_reconcile(&app, pool.inner(), &game_id, vec![changed_path]).await?;

    Ok(info)
}

#[specta::specta]
#[tauri::command]
pub async fn set_mod_category(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_path: String,
    category: String,
) -> Result<(), AppError> {
    let folder = validate_path(&config, &game_id, &folder_path)?;
    metadata::set_mod_category(&pool, &game_id, &folder, &category).await?;
    let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
        crate::services::app::runtime_effects::RuntimeSideEffects {
            pool: &pool,
            config: &config,
            game_id: &game_id,
            collections_dirty: false,
            overlay_refresh: true,
        },
    )
    .await;

    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn set_object_mods_category(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    object_id: String,
    category: String,
) -> Result<usize, AppError> {
    let updated =
        crate::repo::mod_repo::set_object_type_for_object(&pool, &game_id, &object_id, &category)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))? as usize;

    let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
        crate::services::app::runtime_effects::RuntimeSideEffects {
            pool: &pool,
            config: &config,
            game_id: &game_id,
            collections_dirty: false,
            overlay_refresh: true,
        },
    )
    .await;

    Ok(updated)
}

#[derive(serde::Deserialize, specta::Type)]
pub struct MoveModsToObjectInput {
    pub game_id: String,
    pub folder_paths: Vec<String>,
    pub target_object_id: String,
    pub target_subpath: Option<String>,
    pub status: Option<String>,
}

#[specta::specta]
#[tauri::command]
pub async fn list_move_targets_for_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    object_id: String,
) -> Result<Vec<crate::services::mods::organizer_ext::WorkspaceMoveTarget>, AppError> {
    crate::services::mods::organizer_ext::list_move_targets_for_object_service(
        pool.inner(),
        &game_id,
        &object_id,
    )
    .await
}

#[specta::specta]
#[tauri::command]
pub async fn move_mods_to_object(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    op_lock: tauri::State<'_, OperationLock>,
    watcher: tauri::State<'_, WatcherState>,
    input: MoveModsToObjectInput,
) -> Result<crate::services::mods::bulk::BulkResult, AppError> {
    let op_guard = op_lock.acquire().await?;
    let folders = crate::services::fs_utils::guard::validate_paths(
        &config,
        &input.game_id,
        &input.folder_paths,
    )?;
    let result = crate::services::mods::organizer_ext::move_mods_to_object_service(
        pool.inner(),
        &op_guard,
        &watcher,
        crate::services::mods::organizer_ext::MoveModsToObjectParams {
            game_id: &input.game_id,
            folder_paths: &folders,
            target_object_id: &input.target_object_id,
            target_subpath: input.target_subpath.as_deref(),
            status: input.status.as_deref(),
        },
    )
    .await?;

    // Convergence: reconcile source and destination roots after the move.
    // The target root is included explicitly: a partial failure can leave a
    // folder already renamed under the target while its path is absent from
    // `success`, and reconciling only the sources would prune its row.
    let mut changed_paths = input.folder_paths.clone();
    changed_paths.extend(result.success.iter().cloned());
    if let Some(target_obj) =
        crate::repo::object_repo::get_game_object_by_id(pool.inner(), &input.target_object_id)
            .await?
    {
        if let Some(mods_path) =
            crate::repo::game_repo::get_mod_path(pool.inner(), &input.game_id).await?
        {
            changed_paths.push(
                std::path::Path::new(&mods_path)
                    .join(&target_obj.folder_path)
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }
    // Quiet: the move's caller publishes its own refresh from the result.
    if let Err(error) = crate::services::disk_reconcile::emit::run_internal_disk_reconcile(
        &app,
        pool.inner(),
        &input.game_id,
        changed_paths,
    )
    .await
    {
        log::warn!("Post-move disk reconcile failed: {error}");
    }

    Ok(result)
}

#[cfg(test)]
#[path = "tests/mod_meta_cmds_tests.rs"]
mod tests;
