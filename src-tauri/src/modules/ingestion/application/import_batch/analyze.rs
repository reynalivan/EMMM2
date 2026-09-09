use super::staging::stage_import_batch_sources_with_options;
use super::types::{
    ImportBatch, ImportBatchStatus, ImportItemStatus, SetImportItemClassificationInput,
    StableCategory,
};
use crate::modules::catalog::application::match_engine::classification::classify_source;
use crate::modules::catalog::application::match_engine::inspection::{
    inspect_source, InspectionRequest,
};
use crate::modules::ingestion::adapters::sqlite::import_batch;
use crate::modules::library::application::mods::archive::StagingExtractOptions;
use crate::modules::matching::application::deep_matcher::analysis::content::PreparedTokenFilters;
use crate::modules::matching::application::deep_matcher::MasterDb;
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tauri::Manager;

pub async fn analyze_import_batch_for_app(
    app: &tauri::AppHandle,
    db: &SqlitePool,
    batch_id: &str,
) -> Result<ImportBatch, AppError> {
    analyze_import_batch_for_app_with_options(app, db, batch_id, StagingExtractOptions::default())
        .await
}

pub async fn analyze_import_batch_for_app_with_options(
    app: &tauri::AppHandle,
    db: &SqlitePool,
    batch_id: &str,
    options: StagingExtractOptions,
) -> Result<ImportBatch, AppError> {
    let batch = import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    let game_type = crate::modules::games::adapters::sqlite::game::get_game_type(db, &batch.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", batch.game_id)))?
        as i32;
    let master_db =
        crate::modules::workspace::application::scanner::master_db::get_cached(app, game_type)
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
    let staging_root = app
        .path()
        .app_data_dir()
        .map_err(AppError::from)?
        .join("import-staging");

    analyze_import_batch_with_options(
        db,
        batch_id,
        &staging_root,
        &master_db,
        &filters,
        &schema.match_extensions,
        options,
    )
    .await
}

pub async fn analyze_import_batch(
    db: &SqlitePool,
    batch_id: &str,
    staging_root: &Path,
    master_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
    match_extensions: &[String],
) -> Result<ImportBatch, AppError> {
    analyze_import_batch_with_options(
        db,
        batch_id,
        staging_root,
        master_db,
        ini_filters,
        match_extensions,
        StagingExtractOptions::default(),
    )
    .await
}

pub async fn analyze_import_batch_with_options(
    db: &SqlitePool,
    batch_id: &str,
    staging_root: &Path,
    master_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
    match_extensions: &[String],
    options: StagingExtractOptions,
) -> Result<ImportBatch, AppError> {
    let mut batch = import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    if matches!(
        batch.status,
        ImportBatchStatus::Draft | ImportBatchStatus::Failed | ImportBatchStatus::Partial
    ) {
        batch =
            stage_import_batch_sources_with_options(db, batch_id, staging_root, &options).await?;
    }
    if batch.status != ImportBatchStatus::AwaitingReview {
        return Err(AppError::Validation(format!(
            "Import batch '{batch_id}' cannot be analyzed from status {:?}",
            batch.status
        )));
    }

    for item in batch
        .items
        .iter()
        .filter(|item| item.status == ImportItemStatus::Staged)
    {
        if super::is_cancelled(&options.cancel_token) {
            return Err(AppError::Cancelled);
        }
        let analysis_path = item
            .staging_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&item.source_path));
        let inspection = inspect_source(&InspectionRequest {
            source_path: analysis_path.clone(),
            planned_name: Some(item.planned_name.clone()),
            match_extensions: match_extensions.to_vec(),
        })?;
        let categories =
            classify_source(&analysis_path, &item.planned_name, master_db, ini_filters);
        if super::is_cancelled(&options.cancel_token) {
            return Err(AppError::Cancelled);
        }
        if !import_batch::store_inspection(db, &item.id, &inspection, &categories).await? {
            return Err(AppError::Validation(format!(
                "Import item '{}' changed while analysis was running",
                item.id
            )));
        }
        let selected_category = categories.first();
        super::coordinator::set_import_item_classification(
            db,
            SetImportItemClassificationInput {
                item_id: item.id.clone(),
                category: selected_category
                    .map(|suggestion| suggestion.category)
                    .unwrap_or(StableCategory::Other),
                sub_category: selected_category
                    .and_then(|suggestion| suggestion.sub_category.clone()),
                metadata: selected_category
                    .map(|suggestion| suggestion.metadata.clone())
                    .unwrap_or_else(|| serde_json::json!({})),
            },
        )
        .await?;
        super::coordinator::refresh_import_item_suggestions(db, &item.id, master_db, ini_filters)
            .await?;
    }

    if super::is_cancelled(&options.cancel_token) {
        return Err(AppError::Cancelled);
    }

    import_batch::get_batch(db, batch_id).await?.ok_or_else(|| {
        AppError::Internal("Analyzed import batch could not be reloaded".to_string())
    })
}
