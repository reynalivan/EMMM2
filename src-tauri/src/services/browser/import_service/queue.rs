//! Browser downloads enter the shared import-batch workflow.
//!
//! This module intentionally never places files. Analysis ends in the shared
//! Match Wizard even for high-confidence matches.

use crate::domain::errors::BrowserError;
use crate::repo::{browser_repo, import_batch_repo};
use crate::services::import_batch::types::{
    CreateImportBatchInput, ImportFlow, ImportSourceInput, ImportSourceKind, TargetMode,
};
use sqlx::SqlitePool;
use std::path::Path;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;

static ANALYSIS_SEMAPHORE: Semaphore = Semaphore::const_new(2);

pub async fn queue_import_job(
    db: &SqlitePool,
    app: &AppHandle,
    download_id: &str,
    session_id: Option<&str>,
    archive_path: &str,
) -> Result<String, BrowserError> {
    let game_id = match session_id {
        Some(session_id) => browser_repo::get_session_game_id(db, session_id).await?,
        None => None,
    }
    .ok_or_else(|| {
        BrowserError::Import(
            "a game must be selected before a browser download can enter Match Wizard".to_string(),
        )
    })?;

    let item_ids = create_browser_batch(
        db,
        app,
        &game_id,
        vec![(download_id.to_string(), archive_path.to_string())],
    )
    .await?;
    item_ids.into_iter().next().ok_or_else(|| {
        BrowserError::Import("browser import batch was created without an item".to_string())
    })
}

pub async fn bulk_queue_imports(
    db: &SqlitePool,
    app: &AppHandle,
    download_ids: &[String],
    game_id: &str,
) -> Result<Vec<String>, BrowserError> {
    let mut sources = Vec::with_capacity(download_ids.len());
    for download_id in download_ids {
        let Some(download) = browser_repo::get_finished_for_import(db, download_id).await? else {
            continue;
        };
        let Some(file_path) = download.file_path else {
            continue;
        };
        sources.push((download_id.clone(), file_path));
    }
    if sources.is_empty() {
        return Ok(Vec::new());
    }
    create_browser_batch(db, app, game_id, sources).await
}

async fn create_browser_batch(
    db: &SqlitePool,
    app: &AppHandle,
    game_id: &str,
    downloads: Vec<(String, String)>,
) -> Result<Vec<String>, BrowserError> {
    let mut canonical_sources = Vec::with_capacity(downloads.len());
    for (download_id, source) in downloads {
        let canonical = Path::new(&source).canonicalize().map_err(|error| {
            BrowserError::Import(format!(
                "downloaded archive '{source}' is unavailable: {error}"
            ))
        })?;
        canonical_sources.push((download_id, canonical.to_string_lossy().into_owned()));
    }

    let batch = crate::services::import_batch::coordinator::create_import_batch(
        db,
        CreateImportBatchInput {
            game_id: game_id.to_string(),
            flow: ImportFlow::Browser,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: canonical_sources
                .iter()
                .map(|(_, path)| ImportSourceInput {
                    path: path.clone(),
                    source_kind: Some(ImportSourceKind::BrowserDownload),
                })
                .collect(),
        },
    )
    .await
    .map_err(|error| BrowserError::Import(error.to_string()))?;

    let mut item_ids = Vec::with_capacity(batch.items.len());
    for item in &batch.items {
        if let Some((download_id, _)) = canonical_sources
            .iter()
            .find(|(_, source_path)| source_path == &item.source_path)
        {
            import_batch_repo::attach_download_id(db, &item.id, download_id).await?;
        }
        item_ids.push(item.id.clone());
    }

    spawn_batch_analysis(db, app, batch.id.clone());
    Ok(item_ids)
}

fn spawn_batch_analysis(db: &SqlitePool, app: &AppHandle, batch_id: String) {
    let db = db.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let outcome = match ANALYSIS_SEMAPHORE.acquire().await {
            Ok(_permit) => crate::services::import_batch::analyze::analyze_import_batch_for_app(
                &app, &db, &batch_id,
            )
            .await
            .map(|_| ()),
            Err(error) => Err(error.into()),
        };

        let (status, error) = match outcome {
            Ok(()) => ("awaiting_review", None),
            Err(error) => {
                if let Err(status_error) = import_batch_repo::set_batch_status(
                    &db,
                    &batch_id,
                    crate::services::import_batch::types::ImportBatchStatus::Failed,
                )
                .await
                {
                    log::error!(
                        "Failed to persist browser batch failure for {batch_id}: {status_error}"
                    );
                }
                ("failed", Some(error.to_string()))
            }
        };
        let _ = app.emit(
            "import:batch-update",
            serde_json::json!({
                "batch_id": batch_id,
                "status": status,
                "error": error,
            }),
        );
    });
}
