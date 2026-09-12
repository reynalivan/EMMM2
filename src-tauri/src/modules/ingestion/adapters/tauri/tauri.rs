use crate::modules::ingestion::application::import_batch::types::{
    AnalyzeImportBatchOptions, CommitImportBatchInput, CreateImportBatchInput,
    CreateModInboxBatchInput, DeleteProcessedModInboxSourcesInput, ImportBatch, ImportBatchReport,
    ImportSourcePreview, ModInboxSnapshot, RenameImportItemInput, SetImportItemClassificationInput,
    SetImportItemDecisionInput,
};
use crate::modules::library::application::mods::archive::{ExtractionEvent, StagingExtractOptions};
use crate::shared::errors::AppError;
use tauri::ipc::Channel;
use tauri::{Manager, State};

#[tauri::command]
#[specta::specta]
pub async fn create_import_batch(
    pool: State<'_, sqlx::SqlitePool>,
    input: CreateImportBatchInput,
) -> Result<ImportBatch, AppError> {
    crate::modules::ingestion::application::import_batch::coordinator::create_import_batch(
        pool.inner(),
        input,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_import_batch(
    pool: State<'_, sqlx::SqlitePool>,
    batch_id: String,
) -> Result<ImportBatch, AppError> {
    crate::modules::ingestion::adapters::sqlite::import_batch::get_batch(pool.inner(), &batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))
}

#[tauri::command]
#[specta::specta]
pub async fn list_import_batches(
    pool: State<'_, sqlx::SqlitePool>,
    game_id: Option<String>,
) -> Result<Vec<ImportBatch>, AppError> {
    Ok(
        crate::modules::ingestion::adapters::sqlite::import_batch::list_batches(
            pool.inner(),
            game_id.as_deref(),
        )
        .await?,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn analyze_import_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    extraction_state: State<'_, crate::modules::ingestion::application::import_batch::extraction_state::ImportExtractionState>,
    batch_id: String,
) -> Result<ImportBatch, AppError> {
    let lease = extraction_state.acquire(&batch_id)?;
    let result = crate::modules::ingestion::application::import_batch::analyze::analyze_import_batch_for_app_with_options(
        &app,
        pool.inner(),
        &batch_id,
        StagingExtractOptions {
            cancel_token: Some(lease.cancel_token()),
            ..Default::default()
        },
    )
    .await;
    drop(lease);
    result
}

#[tauri::command]
#[specta::specta]
pub async fn analyze_import_batch_with_options(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    extraction_state: State<'_, crate::modules::ingestion::application::import_batch::extraction_state::ImportExtractionState>,
    input: AnalyzeImportBatchOptions,
    on_progress: Channel<ExtractionEvent>,
) -> Result<ImportBatch, AppError> {
    let lease = extraction_state.acquire(&input.batch_id)?;
    let result = crate::modules::ingestion::application::import_batch::analyze::analyze_import_batch_for_app_with_options(
        &app,
        pool.inner(),
        &input.batch_id,
        StagingExtractOptions {
            password: input.password,
            cancel_token: Some(lease.cancel_token()),
            unpack_nested: input.unpack_nested.unwrap_or(true),
            on_progress: Some(on_progress),
        },
    )
    .await;
    drop(lease);
    result
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
    target_manifest_index: State<'_, crate::modules::ingestion::application::import_batch::target_manifest_index::TargetManifestIndexState>,
    item_id: String,
) -> Result<crate::modules::ingestion::application::import_batch::types::ImportItem, AppError> {
    let item =
        crate::modules::ingestion::adapters::sqlite::import_batch::get_item(pool.inner(), &item_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Import item '{item_id}'")))?;
    let batch = crate::modules::ingestion::adapters::sqlite::import_batch::get_batch(
        pool.inner(),
        &item.batch_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Import batch '{}'", item.batch_id)))?;
    let game_type =
        crate::modules::games::adapters::sqlite::game::get_game_type(pool.inner(), &batch.game_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Game '{}'", batch.game_id)))? as i32;
    let master_db =
        crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let filters = crate::modules::workspace::application::scanner::master_db::ini_filters(
        Some(&resource_dir),
        game_type,
    );
    crate::modules::ingestion::application::import_batch::coordinator::refresh_import_item_suggestions_with_target_index(
        pool.inner(),
        target_manifest_index.inner(),
        &item_id,
        &master_db,
        &filters,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn preview_import_library_readiness(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    batch_id: String,
) -> Result<
    crate::modules::ingestion::application::import_batch::coordinator::ImportLibraryReadiness,
    AppError,
> {
    let batch = crate::modules::ingestion::adapters::sqlite::import_batch::get_batch(
        pool.inner(),
        &batch_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    let game_type =
        crate::modules::games::adapters::sqlite::game::get_game_type(pool.inner(), &batch.game_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Game '{}'", batch.game_id)))? as i32;
    let master_db =
        crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let schema = crate::modules::games::application::game::schema_loader::load_schema(
        &resource_dir,
        game_type,
    );
    let filters = crate::modules::workspace::application::scanner::master_db::ini_filters(
        Some(&resource_dir),
        game_type,
    );
    crate::modules::ingestion::application::import_batch::coordinator::preview_import_library_readiness(
        pool.inner(),
        &batch_id,
        &master_db,
        &filters,
        &schema.match_extensions,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_import_batch_matches(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    target_manifest_index: State<'_, crate::modules::ingestion::application::import_batch::target_manifest_index::TargetManifestIndexState>,
    batch_id: String,
) -> Result<ImportBatch, AppError> {
    let batch = crate::modules::ingestion::adapters::sqlite::import_batch::get_batch(
        pool.inner(),
        &batch_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    let game_type =
        crate::modules::games::adapters::sqlite::game::get_game_type(pool.inner(), &batch.game_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Game '{}'", batch.game_id)))? as i32;
    let master_db =
        crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let filters = crate::modules::workspace::application::scanner::master_db::ini_filters(
        Some(&resource_dir),
        game_type,
    );
    crate::modules::ingestion::application::import_batch::coordinator::refresh_import_batch_matches_with_target_index(
        pool.inner(),
        target_manifest_index.inner(),
        &batch_id,
        &master_db,
        &filters,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn mark_import_batch_review_started(
    pool: State<'_, sqlx::SqlitePool>,
    batch_id: String,
) -> Result<(), AppError> {
    if crate::modules::ingestion::adapters::sqlite::import_batch::mark_batch_review_started(
        pool.inner(),
        &batch_id,
    )
    .await?
    {
        Ok(())
    } else {
        Err(AppError::NotFound(format!("Import batch '{batch_id}'")))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn set_import_item_decision(
    pool: State<'_, sqlx::SqlitePool>,
    target_manifest_index: State<'_, crate::modules::ingestion::application::import_batch::target_manifest_index::TargetManifestIndexState>,
    input: SetImportItemDecisionInput,
) -> Result<crate::modules::ingestion::application::import_batch::types::ImportItem, AppError> {
    crate::modules::ingestion::application::import_batch::coordinator::set_import_item_decision_with_target_index(
        pool.inner(),
        target_manifest_index.inner(),
        input,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_import_item_plan(
    pool: State<'_, sqlx::SqlitePool>,
    input: RenameImportItemInput,
) -> Result<crate::modules::ingestion::application::import_batch::types::ImportItem, AppError> {
    crate::modules::ingestion::application::import_batch::coordinator::rename_import_item_plan(
        pool.inner(),
        input,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_import_source_preview(
    pool: State<'_, sqlx::SqlitePool>,
    item_id: String,
) -> Result<ImportSourcePreview, AppError> {
    crate::modules::ingestion::application::import_batch::preview::get_import_source_preview(
        pool.inner(),
        &item_id,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn reveal_import_source(
    pool: State<'_, sqlx::SqlitePool>,
    item_id: String,
) -> Result<(), AppError> {
    crate::modules::ingestion::application::import_batch::preview::reveal_import_source(
        pool.inner(),
        &item_id,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn reveal_import_destination(
    pool: State<'_, sqlx::SqlitePool>,
    item_id: String,
) -> Result<(), AppError> {
    crate::modules::ingestion::application::import_batch::preview::reveal_import_destination(
        pool.inner(),
        &item_id,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_import_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    extraction_state: State<'_, crate::modules::ingestion::application::import_batch::extraction_state::ImportExtractionState>,
    target_manifest_index: State<'_, crate::modules::ingestion::application::import_batch::target_manifest_index::TargetManifestIndexState>,
    batch_id: String,
) -> Result<(), AppError> {
    let cancellation = extraction_state.cancel_and_wait(&batch_id).await;
    if crate::modules::ingestion::adapters::sqlite::import_batch::cancel_batch(
        pool.inner(),
        &batch_id,
    )
    .await?
    {
        let staging_root = app
            .path()
            .app_data_dir()
            .map_err(AppError::from)?
            .join("import-staging");
        crate::modules::ingestion::application::import_batch::staging::cleanup_batch_staging(
            &staging_root,
            &batch_id,
        )?;
        target_manifest_index.clear_batch(&batch_id);
        drop(cancellation);
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
    target_manifest_index: State<'_, crate::modules::ingestion::application::import_batch::target_manifest_index::TargetManifestIndexState>,
    input: CommitImportBatchInput,
) -> Result<ImportBatchReport, AppError> {
    let batch_id = input.batch_id.clone();
    let batch = crate::modules::ingestion::adapters::sqlite::import_batch::get_batch(
        pool.inner(),
        &batch_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Import batch '{}'", input.batch_id)))?;
    let game_type =
        crate::modules::games::adapters::sqlite::game::get_game_type(pool.inner(), &batch.game_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Game '{}'", batch.game_id)))? as i32;
    let master_db =
        crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let report = crate::modules::mutation::application::workspace_mutation::import_commit::commit_import_batch(
        &app,
        pool.inner(),
        input,
        &master_db,
    )
    .await?;
    let terminal = crate::modules::ingestion::adapters::sqlite::import_batch::get_batch(
        pool.inner(),
        &batch_id,
    )
    .await?
    .is_some_and(|current| {
        matches!(
            current.status,
            crate::modules::ingestion::application::import_batch::types::ImportBatchStatus::Done
                | crate::modules::ingestion::application::import_batch::types::ImportBatchStatus::Failed
                | crate::modules::ingestion::application::import_batch::types::ImportBatchStatus::Cancelled
        )
    });
    if terminal {
        target_manifest_index.clear_batch(&batch_id);
    }
    Ok(report)
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
    crate::modules::ingestion::application::import_batch::mod_inbox_watcher::start(
        app,
        state.inner(),
        &game_id,
        &root,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn stop_mod_inbox_watcher(
    state: State<'_, crate::modules::ingestion::application::import_batch::mod_inbox_watcher::ModInboxWatcherState>,
    game_id: String,
    root_path: String,
) -> Result<(), AppError> {
    crate::modules::ingestion::application::import_batch::mod_inbox_watcher::stop(
        state.inner(),
        &game_id,
        &root_path,
    );
    Ok(())
}

// ── Classification & Relocation ───────────────────────────────────────────────

use crate::modules::catalog::application::objects::classification_batch::{
    ApplyObjectClassificationBatchInput, ApplyObjectClassificationBatchResult,
    CanonicalClassificationCatalogEntry, ObjectClassificationPreviewItem,
    PreviewObjectClassificationBatchInput,
};

#[tauri::command]
#[specta::specta]
pub async fn preview_object_classification_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    input: PreviewObjectClassificationBatchInput,
) -> Result<Vec<ObjectClassificationPreviewItem>, AppError> {
    let game_type =
        crate::modules::games::adapters::sqlite::game::get_game_type(pool.inner(), &input.game_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Game '{}'", input.game_id)))? as i32;
    let master_db =
        crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let schema = crate::modules::games::application::game::schema_loader::load_schema(
        &resource_dir,
        game_type,
    );
    let filters = crate::modules::workspace::application::scanner::master_db::ini_filters(
        Some(&resource_dir),
        game_type,
    );
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
pub async fn list_canonical_classification_catalog(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<CanonicalClassificationCatalogEntry>, AppError> {
    let game_type =
        crate::modules::games::adapters::sqlite::game::get_game_type(pool.inner(), &game_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Game '{game_id}'")))? as i32;
    let master_db =
        crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    Ok(
        crate::modules::catalog::application::objects::classification_batch::list_canonical_classification_catalog(
            &master_db,
        ),
    )
}

#[tauri::command]
#[specta::specta]
pub async fn apply_object_classification_batch(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    input: ApplyObjectClassificationBatchInput,
) -> Result<ApplyObjectClassificationBatchResult, AppError> {
    let game_type =
        crate::modules::games::adapters::sqlite::game::get_game_type(pool.inner(), &input.game_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Game '{}'", input.game_id)))? as i32;
    let master_db =
        crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let schema = crate::modules::games::application::game::schema_loader::load_schema(
        &resource_dir,
        game_type,
    );
    let filters = crate::modules::workspace::application::scanner::master_db::ini_filters(
        Some(&resource_dir),
        game_type,
    );
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
            &filters,
            &schema.match_extensions,
        )
        .await?;
    if result.aliases_changed {
        crate::modules::workspace::application::scanner::master_db::MasterDbCache::invalidate(&app)
            .await;
    }
    if disable_after_apply {
        match crate::modules::mutation::application::workspace_mutation::object_status::disable_object_roots(
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
    let Some(config) =
        app.try_state::<crate::modules::settings::application::config::ConfigService>()
    else {
        log::warn!("Classification completed but ConfigService is unavailable");
        return;
    };
    let Some(state) =
        app.try_state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>()
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
) -> Result<
    Vec<crate::modules::ingestion::application::import_batch::relocation::RelocationPreviewItem>,
    AppError,
> {
    crate::modules::ingestion::application::import_batch::relocation::preview_relocation_batch(
        pool.inner(),
        input,
    )
    .await
}
