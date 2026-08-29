use super::staging::stage_import_batch_sources;
use super::types::{ImportBatch, ImportBatchStatus, ImportItemStatus};
use crate::domain::errors::AppError;
use crate::repo::import_batch_repo;
use crate::services::match_engine::classification::classify_source;
use crate::services::match_engine::inspection::{inspect_source, InspectionRequest};
use crate::services::scanner::deep_matcher::analysis::content::PreparedTokenFilters;
use crate::services::scanner::deep_matcher::MasterDb;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tauri::Manager;

pub async fn analyze_import_batch_for_app(
    app: &tauri::AppHandle,
    db: &SqlitePool,
    batch_id: &str,
) -> Result<ImportBatch, AppError> {
    let batch = import_batch_repo::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    let game_type = crate::repo::game_repo::get_game_type(db, &batch.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", batch.game_id)))?
        as i32;
    let master_db = crate::services::scanner::master_db::get_cached(app, game_type)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("MasterDB for game type {game_type}")))?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let schema = crate::services::game::schema_loader::load_schema(&resource_dir, game_type);
    let filters = crate::services::scanner::master_db::ini_filters(Some(&resource_dir), game_type);
    let staging_root = app
        .path()
        .app_data_dir()
        .map_err(AppError::from)?
        .join("import-staging");

    analyze_import_batch(
        db,
        batch_id,
        &staging_root,
        &master_db,
        &filters,
        &schema.match_extensions,
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
    let mut batch = import_batch_repo::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    if matches!(
        batch.status,
        ImportBatchStatus::Draft | ImportBatchStatus::Failed | ImportBatchStatus::Partial
    ) {
        batch = stage_import_batch_sources(db, batch_id, staging_root).await?;
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
        if !import_batch_repo::store_inspection(db, &item.id, &inspection, &categories).await? {
            return Err(AppError::Validation(format!(
                "Import item '{}' changed while analysis was running",
                item.id
            )));
        }
    }

    import_batch_repo::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| {
            AppError::Internal("Analyzed import batch could not be reloaded".to_string())
        })
}
