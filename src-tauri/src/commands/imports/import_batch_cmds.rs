use crate::domain::errors::AppError;
use crate::services::import_batch::types::{
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
    crate::services::import_batch::coordinator::create_import_batch(pool.inner(), input).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_import_batch(
    pool: State<'_, sqlx::SqlitePool>,
    batch_id: String,
) -> Result<ImportBatch, AppError> {
    crate::repo::import_batch::get_batch(pool.inner(), &batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))
}

#[tauri::command]
#[specta::specta]
pub async fn list_import_batches(
    pool: State<'_, sqlx::SqlitePool>,
    game_id: Option<String>,
) -> Result<Vec<ImportBatch>, AppError> {
    Ok(crate::repo::import_batch::list_batches(pool.inner(), game_id.as_deref()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn analyze_import_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    batch_id: String,
) -> Result<ImportBatch, AppError> {
    crate::services::import_batch::analyze::analyze_import_batch_for_app(
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
) -> Result<crate::services::import_batch::types::ImportItem, AppError> {
    crate::services::import_batch::coordinator::set_import_item_classification(pool.inner(), input)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_import_item_suggestions(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    item_id: String,
) -> Result<crate::services::import_batch::types::ImportItem, AppError> {
    let item = crate::repo::import_batch::get_item(pool.inner(), &item_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import item '{item_id}'")))?;
    let batch = crate::repo::import_batch::get_batch(pool.inner(), &item.batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{}'", item.batch_id)))?;
    let game_type = crate::repo::game::get_game_type(pool.inner(), &batch.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", batch.game_id)))?
        as i32;
    let master_db = crate::services::scanner::master_db::get_cached(&app, game_type)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let filters = crate::services::scanner::master_db::ini_filters(Some(&resource_dir), game_type);
    crate::services::import_batch::coordinator::refresh_import_item_suggestions(
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
) -> Result<crate::services::import_batch::types::ImportItem, AppError> {
    crate::services::import_batch::coordinator::set_import_item_decision(pool.inner(), input).await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_import_item_plan(
    pool: State<'_, sqlx::SqlitePool>,
    input: RenameImportItemInput,
) -> Result<crate::services::import_batch::types::ImportItem, AppError> {
    crate::services::import_batch::coordinator::rename_import_item_plan(pool.inner(), input).await
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_import_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    batch_id: String,
) -> Result<(), AppError> {
    if crate::repo::import_batch::cancel_batch(pool.inner(), &batch_id).await? {
        let staging_root = app
            .path()
            .app_data_dir()
            .map_err(AppError::from)?
            .join("import-staging");
        crate::services::import_batch::staging::cleanup_batch_staging(&staging_root, &batch_id)?;
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
    let batch = crate::repo::import_batch::get_batch(pool.inner(), &input.batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{}'", input.batch_id)))?;
    let game_type = crate::repo::game::get_game_type(pool.inner(), &batch.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", batch.game_id)))?
        as i32;
    let master_db = crate::services::scanner::master_db::get_cached(&app, game_type)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    crate::services::workspace_mutation::import_commit::commit_import_batch(
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
    let root = crate::services::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &game_id,
        None,
    )
    .await?;
    crate::services::import_batch::ready_to_move::validate_mod_inbox_root(
        pool.inner(),
        &game_id,
        &root,
    )
    .await?;
    crate::services::import_batch::mod_inbox::build_mod_inbox_snapshot(
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
    let root = crate::services::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &game_id,
        None,
    )
    .await?;
    crate::services::import_batch::ready_to_move::validate_mod_inbox_root(
        pool.inner(),
        &game_id,
        &root,
    )
    .await?;
    std::fs::create_dir_all(&root)?;
    crate::services::import_batch::mod_inbox::build_mod_inbox_snapshot(
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
    let root = crate::services::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &game_id,
        None,
    )
    .await?;
    crate::services::import_batch::ready_to_move::validate_mod_inbox_root(
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
    let root = crate::services::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &input.game_id,
        None,
    )
    .await?;
    crate::services::import_batch::ready_to_move::validate_mod_inbox_root(
        pool.inner(),
        &input.game_id,
        &root,
    )
    .await?;
    crate::services::import_batch::mod_inbox::create_mod_inbox_batch(
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
    let root = crate::services::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &input.game_id,
        None,
    )
    .await?;
    crate::services::import_batch::ready_to_move::validate_mod_inbox_root(
        pool.inner(),
        &input.game_id,
        &root,
    )
    .await?;
    crate::services::import_batch::mod_inbox::delete_processed_sources(
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
    state: State<'_, crate::services::import_batch::mod_inbox_watcher::ModInboxWatcherState>,
    game_id: String,
) -> Result<(), AppError> {
    let root = crate::services::import_batch::ready_to_move::resolve_mod_inbox_root(
        &app,
        pool.inner(),
        &game_id,
        None,
    )
    .await?;
    crate::services::import_batch::ready_to_move::validate_mod_inbox_root(
        pool.inner(),
        &game_id,
        &root,
    )
    .await?;
    crate::services::import_batch::mod_inbox_watcher::start(app, state.inner(), &game_id, &root)
}

#[tauri::command]
#[specta::specta]
pub async fn stop_mod_inbox_watcher(
    state: State<'_, crate::services::import_batch::mod_inbox_watcher::ModInboxWatcherState>,
    game_id: String,
) -> Result<(), AppError> {
    crate::services::import_batch::mod_inbox_watcher::stop(state.inner(), &game_id);
    Ok(())
}
