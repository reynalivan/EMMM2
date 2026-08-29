//! Browser import queue listing and bounded cleanup of legacy staging.

use crate::domain::errors::BrowserError;
use sqlx::SqlitePool;
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::repo::browser;

pub use crate::domain::browser::ImportJobDto;

pub async fn list_jobs(db: &SqlitePool) -> Result<Vec<ImportJobDto>, BrowserError> {
    Ok(browser::list_active_jobs(db).await?)
}

pub async fn cleanup_old_terminal_staging(
    db: &SqlitePool,
    app: &AppHandle,
    limit: i64,
) -> Result<usize, BrowserError> {
    let candidates = browser::list_terminal_staging_cleanup_candidates(db, limit).await?;
    let staging_root = browser_staging_root(app)?;
    let mut cleaned = 0;
    for (job_id, staging_path) in candidates {
        match remove_job_staging_dir(&staging_root, &job_id, Path::new(&staging_path)) {
            Ok(()) => {
                browser::clear_staging_path(db, &job_id).await?;
                cleaned += 1;
            }
            Err(error) => {
                log::warn!("startup: skipped unsafe import staging cleanup for {job_id}: {error}");
            }
        }
    }
    Ok(cleaned)
}

fn browser_staging_root(app: &AppHandle) -> Result<PathBuf, BrowserError> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("staging"))
        .map_err(|error| {
            BrowserError::Import(format!("app data directory is unavailable: {error}"))
        })
}

fn remove_job_staging_dir(
    staging_root: &Path,
    job_id: &str,
    stored_archive_path: &Path,
) -> Result<(), BrowserError> {
    if !matches!(
        Path::new(job_id)
            .components()
            .collect::<Vec<_>>()
            .as_slice(),
        [Component::Normal(_)]
    ) {
        return Err(BrowserError::Import(format!(
            "invalid import job id for staging cleanup: '{job_id}'"
        )));
    }
    let expected_dir = staging_root.join(job_id);
    let stored_dir = stored_archive_path.parent().ok_or_else(|| {
        BrowserError::Import(format!(
            "staging path has no parent: {}",
            stored_archive_path.display()
        ))
    })?;
    let expected_key =
        crate::common::path_key::folder_path_key(expected_dir.to_string_lossy().as_ref(), None);
    let stored_key =
        crate::common::path_key::folder_path_key(stored_dir.to_string_lossy().as_ref(), None);
    if expected_key != stored_key {
        return Err(BrowserError::Import(format!(
            "refusing to remove staging path outside its staging directory: {}",
            stored_dir.display()
        )));
    }
    if expected_dir.exists() {
        std::fs::remove_dir_all(expected_dir)?;
    }
    Ok(())
}
