use super::acquire_current_snapshot_lease;
use crate::modules::collections::application::collection;
use crate::modules::collections::domain::collection::{ApplyPreview, CollectionPreview};
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use tauri::{AppHandle, State};
#[tauri::command]
#[specta::specta]
pub async fn get_collection_preview(
    app: AppHandle,
    op_lock: State<'_, MutationCoordinator>,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    collection_id: String,
    game_id: String,
) -> Result<CollectionPreview, AppError> {
    let settings = config.get_settings();
    let mods_path = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .map(|g| g.mod_path.to_string_lossy().to_string());

    let _snapshot_lease = if mods_path
        .as_deref()
        .is_some_and(|path| std::path::Path::new(path).is_dir())
    {
        Some(acquire_current_snapshot_lease(&app, pool.inner(), op_lock.inner(), &game_id).await?)
    } else {
        None
    };

    let result = collection::get_collection_preview(
        pool.inner(),
        &game_id,
        &collection_id,
        mods_path.as_deref(),
    )
    .await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn preview_apply_collection(
    app: AppHandle,
    op_lock: State<'_, MutationCoordinator>,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    game_id: String,
    collection_id: String,
) -> Result<ApplyPreview, AppError> {
    let _snapshot_lease =
        acquire_current_snapshot_lease(&app, pool.inner(), op_lock.inner(), &game_id).await?;
    let settings = config.get_settings();
    let mods_path = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .map(|g| g.mod_path.to_string_lossy().to_string());

    let safe_mode_enabled = settings.safety.runtime_safe_mode_for(&game_id);
    let result = collection::preview_apply(
        pool.inner(),
        &game_id,
        &collection_id,
        mods_path.as_deref(),
        safe_mode_enabled,
    )
    .await?;
    Ok(result)
}
