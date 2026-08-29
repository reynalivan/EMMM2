use crate::shared::errors::AppError;
use crate::modules::ingestion::application::import_batch::types::{
    CommitImportBatchInput, CreateImportBatchInput, CreateModInboxBatchInput,
    DeleteProcessedModInboxSourcesInput, ImportBatch, ImportBatchReport, ModInboxSnapshot,
    RenameImportItemInput, SetImportItemClassificationInput, SetImportItemDecisionInput,
};
use tauri::{Manager, State};

#[tauri::command]
#[specta::specta]
pub async fn create_import_batch(
    pool: State<'_, sqlx::SqlitePool>,
    input: CreateImportBatchInput,
) -> Result<ImportBatch, AppError> {
    crate::modules::ingestion::application::import_batch::coordinator::create_import_batch(pool.inner(), input).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_import_batch(
    pool: State<'_, sqlx::SqlitePool>,
    batch_id: String,
) -> Result<ImportBatch, AppError> {
    crate::modules::ingestion::adapters::outbound::sqlite::import_batch::get_batch(pool.inner(), &batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))
}

#[tauri::command]
#[specta::specta]
pub async fn list_import_batches(
    pool: State<'_, sqlx::SqlitePool>,
    game_id: Option<String>,
) -> Result<Vec<ImportBatch>, AppError> {
    Ok(crate::modules::ingestion::adapters::outbound::sqlite::import_batch::list_batches(pool.inner(), game_id.as_deref()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn analyze_import_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    batch_id: String,
) -> Result<ImportBatch, AppError> {
    crate::modules::ingestion::application::import_batch::analyze::analyze_import_batch_for_app(
        &app,
        pool.inner(),
        &batch_id,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn set_import_item_classification(
    pool: State<'_, sqlx::SqlitePool>,
    input: SetImportItemClassificationInput,
) -> Result<crate::modules::ingestion::application::import_batch::types::ImportItem, AppError> {
    crate::modules::ingestion::application::import_batch::coordinator::set_import_item_classification(pool.inner(), input)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_import_item_suggestions(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    item_id: String,
) -> Result<crate::modules::ingestion::application::import_batch::types::ImportItem, AppError> {
    let item = crate::modules::ingestion::adapters::outbound::sqlite::import_batch::get_item(pool.inner(), &item_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import item '{item_id}'")))?;
    let batch = crate::modules::ingestion::adapters::outbound::sqlite::import_batch::get_batch(pool.inner(), &item.batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{}'", item.batch_id)))?;
    let game_type = crate::modules::games::adapters::outbound::sqlite::game::get_game_type(pool.inner(), &batch.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", batch.game_id)))?
        as i32;
    let master_db = crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let filters = crate::modules::workspace::application::scanner::master_db::ini_filters(Some(&resource_dir), game_type);
    crate::modules::ingestion::application::import_batch::coordinator::refresh_import_item_suggestions(
        pool.inner(),
        &item_id,
        &master_db,
        &filters,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn set_import_item_decision(
    pool: State<'_, sqlx::SqlitePool>,
    input: SetImportItemDecisionInput,
) -> Result<crate::modules::ingestion::application::import_batch::types::ImportItem, AppError> {
    crate::modules::ingestion::application::import_batch::coordinator::set_import_item_decision(pool.inner(), input).await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_import_item_plan(
    pool: State<'_, sqlx::SqlitePool>,
    input: RenameImportItemInput,
) -> Result<crate::modules::ingestion::application::import_batch::types::ImportItem, AppError> {
    crate::modules::ingestion::application::import_batch::coordinator::rename_import_item_plan(pool.inner(), input).await
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_import_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    batch_id: String,
) -> Result<(), AppError> {
    if crate::modules::ingestion::adapters::outbound::sqlite::import_batch::cancel_batch(pool.inner(), &batch_id).await? {
        let staging_root = app
            .path()
            .app_data_dir()
            .map_err(AppError::from)?
            .join("import-staging");
        crate::modules::ingestion::application::import_batch::staging::cleanup_batch_staging(&staging_root, &batch_id)?;
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Import batch '{batch_id}' can no longer be cancelled"
        )))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn commit_import_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    input: CommitImportBatchInput,
) -> Result<ImportBatchReport, AppError> {
    let batch = crate::modules::ingestion::adapters::outbound::sqlite::import_batch::get_batch(pool.inner(), &input.batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{}'", input.batch_id)))?;
    let game_type = crate::modules::games::adapters::outbound::sqlite::game::get_game_type(pool.inner(), &batch.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", batch.game_id)))?
        as i32;
    let master_db = crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    crate::modules::workspace::application::workspace_mutation::import_commit::commit_import_batch(
        &app,
        pool.inner(),
        input,
        &master_db,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_mod_inbox(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<ModInboxSnapshot, AppError> {
    let root = crate::modules::ingestion::application::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &game_id,
        None,
    )
    .await?;
    crate::modules::ingestion::application::import_batch::ready_to_move::validate_mod_inbox_root(
        pool.inner(),
        &game_id,
        &root,
    )
    .await?;
    crate::modules::ingestion::application::import_batch::mod_inbox::build_mod_inbox_snapshot(
        pool.inner(),
        &game_id,
        &root,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_mod_inbox_folder(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<ModInboxSnapshot, AppError> {
    let root = crate::modules::ingestion::application::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &game_id,
        None,
    )
    .await?;
    crate::modules::ingestion::application::import_batch::ready_to_move::validate_mod_inbox_root(
        pool.inner(),
        &game_id,
        &root,
    )
    .await?;
    std::fs::create_dir_all(&root)?;
    crate::modules::ingestion::application::import_batch::mod_inbox::build_mod_inbox_snapshot(
        pool.inner(),
        &game_id,
        &root,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn open_mod_inbox_folder(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<(), AppError> {
    let root = crate::modules::ingestion::application::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &game_id,
        None,
    )
    .await?;
    crate::modules::ingestion::application::import_batch::ready_to_move::validate_mod_inbox_root(
        pool.inner(),
        &game_id,
        &root,
    )
    .await?;
    let canonical_root = root.canonicalize().map_err(|error| {
        AppError::Validation(format!("Mod Inbox folder is unavailable: {error}"))
    })?;
    if !canonical_root.is_dir() {
        return Err(AppError::Validation(format!(
            "Mod Inbox path is not a directory: {}",
            canonical_root.display()
        )));
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&canonical_root)
            .spawn()
            .map_err(|error| AppError::Io(format!("Failed to open Mod Inbox: {error}")))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    Err(AppError::Io(
        "Open Mod Inbox is only supported on Windows".to_string(),
    ))
}

#[tauri::command]
#[specta::specta]
pub async fn create_mod_inbox_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    input: CreateModInboxBatchInput,
) -> Result<ImportBatch, AppError> {
    let root = crate::modules::ingestion::application::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &input.game_id,
        None,
    )
    .await?;
    crate::modules::ingestion::application::import_batch::ready_to_move::validate_mod_inbox_root(
        pool.inner(),
        &input.game_id,
        &root,
    )
    .await?;
    crate::modules::ingestion::application::import_batch::mod_inbox::create_mod_inbox_batch(
        pool.inner(),
        &input.game_id,
        &root,
        &input.entry_keys,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_processed_mod_inbox_sources(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    input: DeleteProcessedModInboxSourcesInput,
) -> Result<ModInboxSnapshot, AppError> {
    let root = crate::modules::ingestion::application::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &input.game_id,
        None,
    )
    .await?;
    crate::modules::ingestion::application::import_batch::ready_to_move::validate_mod_inbox_root(
        pool.inner(),
        &input.game_id,
        &root,
    )
    .await?;
    crate::modules::ingestion::application::import_batch::mod_inbox::delete_processed_sources(
        pool.inner(),
        &input.game_id,
        &root,
        &input.source_ids,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn start_mod_inbox_watcher(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    state: State<'_, crate::modules::ingestion::application::import_batch::mod_inbox_watcher::ModInboxWatcherState>,
    game_id: String,
) -> Result<(), AppError> {
    let root = crate::modules::ingestion::application::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &game_id,
        None,
    )
    .await?;
    crate::modules::ingestion::application::import_batch::ready_to_move::validate_mod_inbox_root(
        pool.inner(),
        &game_id,
        &root,
    )
    .await?;
    crate::modules::ingestion::application::import_batch::mod_inbox_watcher::start(app, state.inner(), &game_id, &root)
}

#[tauri::command]
#[specta::specta]
pub async fn stop_mod_inbox_watcher(
    state: State<'_, crate::modules::ingestion::application::import_batch::mod_inbox_watcher::ModInboxWatcherState>,
    game_id: String,
) -> Result<(), AppError> {
    crate::modules::ingestion::application::import_batch::mod_inbox_watcher::stop(state.inner(), &game_id);
    Ok(())
}

// ── Classification & Relocation ───────────────────────────────────────────────

use crate::modules::catalog::application::objects::classification_batch::{
    ApplyObjectClassificationBatchInput, ApplyObjectClassificationBatchResult,
    ObjectClassificationPreviewItem, PreviewObjectClassificationBatchInput,
};

#[tauri::command]
#[specta::specta]
pub async fn preview_object_classification_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    input: PreviewObjectClassificationBatchInput,
) -> Result<Vec<ObjectClassificationPreviewItem>, AppError> {
    let game_type = crate::modules::games::adapters::outbound::sqlite::game::get_game_type(pool.inner(), &input.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", input.game_id)))?
        as i32;
    let master_db = crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let schema = crate::modules::games::application::game::schema_loader::load_schema(&resource_dir, game_type);
    let filters = crate::modules::workspace::application::scanner::master_db::ini_filters(Some(&resource_dir), game_type);
    crate::modules::catalog::application::objects::classification_batch::preview_object_classification_batch(
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
    let game_type = crate::modules::games::adapters::outbound::sqlite::game::get_game_type(pool.inner(), &input.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", input.game_id)))?
        as i32;
    let master_db = crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let schema = crate::modules::games::application::game::schema_loader::load_schema(&resource_dir, game_type);
    let game_id = input.game_id.clone();
    let disable_after_apply = input.disable_after_apply;
    let object_ids = input
        .items
        .iter()
        .map(|item| item.object_id.clone())
        .collect::<Vec<_>>();
    let mut result =
        crate::modules::catalog::application::objects::classification_batch::apply_object_classification_batch(
            pool.inner(),
            input,
            &master_db,
            &schema.match_extensions,
        )
        .await?;
    if result.aliases_changed {
        crate::modules::workspace::application::scanner::master_db::MasterDbCache::invalidate(&app).await;
    }
    if disable_after_apply {
        match crate::modules::workspace::application::workspace_mutation::object_status::disable_object_roots(
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
    let Some(config) = app.try_state::<crate::modules::system::application::config::ConfigService>() else {
        log::warn!("Classification completed but ConfigService is unavailable");
        return;
    };
    let Some(state) =
        app.try_state::<crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileState>()
    else {
        log::warn!("Classification completed but DiskReconcileState is unavailable");
        return;
    };
    let settlement = crate::modules::system::application::app::runtime_effects::settle_committed_runtime_effects(
        state.inner(),
        crate::modules::system::application::app::runtime_effects::RuntimeSideEffects {
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
    input: crate::modules::ingestion::application::import_batch::relocation::PreviewRelocationBatchInput,
) -> Result<Vec<crate::modules::ingestion::application::import_batch::relocation::RelocationPreviewItem>, AppError> {
    crate::modules::ingestion::application::import_batch::relocation::preview_relocation_batch(pool.inner(), input).await
}
