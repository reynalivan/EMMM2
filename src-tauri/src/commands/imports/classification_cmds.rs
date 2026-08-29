use crate::domain::errors::AppError;
use crate::services::objects::classification_batch::{
    ApplyObjectClassificationBatchInput, ApplyObjectClassificationBatchResult,
    ObjectClassificationPreviewItem, PreviewObjectClassificationBatchInput,
};
use tauri::{Manager, State};

#[tauri::command]
#[specta::specta]
pub async fn preview_object_classification_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    input: PreviewObjectClassificationBatchInput,
) -> Result<Vec<ObjectClassificationPreviewItem>, AppError> {
    let game_type = crate::repo::game_repo::get_game_type(pool.inner(), &input.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", input.game_id)))?
        as i32;
    let master_db = crate::services::scanner::master_db::get_cached(&app, game_type)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let schema = crate::services::game::schema_loader::load_schema(&resource_dir, game_type);
    let filters = crate::services::scanner::master_db::ini_filters(Some(&resource_dir), game_type);
    crate::services::objects::classification_batch::preview_object_classification_batch(
        pool.inner(),
        &input,
        &master_db,
        &filters,
        &schema.match_extensions,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn apply_object_classification_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    input: ApplyObjectClassificationBatchInput,
) -> Result<ApplyObjectClassificationBatchResult, AppError> {
    let game_type = crate::repo::game_repo::get_game_type(pool.inner(), &input.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", input.game_id)))?
        as i32;
    let master_db = crate::services::scanner::master_db::get_cached(&app, game_type)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let schema = crate::services::game::schema_loader::load_schema(&resource_dir, game_type);
    let game_id = input.game_id.clone();
    let disable_after_apply = input.disable_after_apply;
    let object_ids = input
        .items
        .iter()
        .map(|item| item.object_id.clone())
        .collect::<Vec<_>>();
    let mut result =
        crate::services::objects::classification_batch::apply_object_classification_batch(
            pool.inner(),
            input,
            &master_db,
            &schema.match_extensions,
        )
        .await?;
    if result.aliases_changed {
        crate::services::scanner::master_db::MasterDbCache::invalidate(&app).await;
    }
    if disable_after_apply {
        match crate::services::workspace_mutation::object_status::disable_object_roots(
            &app,
            pool.inner(),
            &game_id,
            &object_ids,
        )
        .await
        {
            Ok(disabled) => {
                result.disabled_objects = disabled.disabled_objects;
                result.disable_warning = disabled.warning;
            }
            Err(error) => {
                result.disable_warning = Some(format!(
                    "Classification was saved, but object folders could not be disabled: {error}"
                ));
            }
        }
    }
    settle_classification_runtime_effects(&app, pool.inner(), &game_id).await;
    Ok(result)
}

async fn settle_classification_runtime_effects(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
) {
    let Some(config) = app.try_state::<crate::services::config::ConfigService>() else {
        log::warn!("Classification completed but ConfigService is unavailable");
        return;
    };
    let Some(state) =
        app.try_state::<crate::services::disk_reconcile::orchestrator::DiskReconcileState>()
    else {
        log::warn!("Classification completed but DiskReconcileState is unavailable");
        return;
    };
    let settlement = crate::services::app::runtime_effects::settle_committed_runtime_effects(
        state.inner(),
        crate::services::app::runtime_effects::RuntimeSideEffects {
            pool,
            config: config.inner(),
            game_id,
            collections_dirty: true,
            overlay_refresh: true,
        },
    )
    .await;
    if let Some(warning) = settlement.warning {
        log::warn!("Classification runtime effects pending: {warning}");
    }
}

#[tauri::command]
#[specta::specta]
pub async fn preview_relocation_batch(
    pool: State<'_, sqlx::SqlitePool>,
    input: crate::services::import_batch::relocation::PreviewRelocationBatchInput,
) -> Result<Vec<crate::services::import_batch::relocation::RelocationPreviewItem>, AppError> {
    crate::services::import_batch::relocation::preview_relocation_batch(pool.inner(), input).await
}
