use super::acquire_current_snapshot_guard;
use crate::modules::collections::application::collection;
use crate::modules::collections::domain::collection::CollectionSummary;
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use tauri::{AppHandle, State};
#[tauri::command]
#[specta::specta]
pub async fn save_collection_changes(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    collection_id: String,
    confirm_remove_missing: bool,
) -> Result<CollectionSummary, AppError> {
    let _guard =
        acquire_current_snapshot_guard(&app, pool.inner(), op_lock.inner(), &game_id).await?;
    let mods_path = config
        .get_settings()
        .games
        .into_iter()
        .find(|game| game.id == game_id)
        .map(|game| game.mod_path.to_string_lossy().to_string());
    let preview = collection::get_collection_preview(
        pool.inner(),
        &game_id,
        &collection_id,
        mods_path.as_deref(),
    )
    .await?;
    let missing_paths = preview
        .projected_state
        .active_roots
        .iter()
        .filter(|root| root.is_missing)
        .map(|root| root.source_path.clone())
        .collect::<Vec<_>>();
    if !confirm_remove_missing && !missing_paths.is_empty() {
        return Err(crate::shared::errors::CollectionError::MissingMods {
            count: missing_paths.len(),
            paths: missing_paths,
        }
        .into());
    }
    Ok(
        collection::replace_collection_with_current_state(pool.inner(), &game_id, &collection_id)
            .await?,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn clear_last_changes(
    pool: State<'_, SqlitePool>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
) -> Result<(), AppError> {
    let _guard = op_lock
        .acquire_exempt(
            crate::modules::mutation::coordinator::MutationExemption::CollectionMetadata,
        )
        .await?;
    collection::clear_last_changes(pool.inner(), &game_id).await?;
    Ok(())
}
