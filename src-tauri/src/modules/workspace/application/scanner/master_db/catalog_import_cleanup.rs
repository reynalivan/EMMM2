#[cfg(not(test))]
use std::time::Duration;
use std::{
    path::Path,
    time::{Instant, SystemTime},
};
#[cfg(not(test))]
use tauri::Manager;

use super::{CatalogImportState, STAGING_TTL};
#[cfg(not(test))]
use crate::modules::workspace::application::scanner::master_db::CatalogUpdateState;
use crate::shared::errors::ScannerError;

const STAGING_PREFIX: &str = ".catalog-import-staging-";
#[cfg(not(test))]
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

fn is_staging_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(STAGING_PREFIX))
        .is_some_and(|suffix| uuid::Uuid::parse_str(suffix).is_ok())
}

pub(super) fn remove_staging(app_data_dir: &Path, path: &Path) -> Result<(), ScannerError> {
    if !is_staging_name(path) || path.parent() != Some(app_data_dir) {
        return Err(ScannerError::Security(
            "Invalid catalog staging cleanup path".to_string(),
        ));
    }
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let resolved = path.canonicalize()?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || resolved.parent() != Some(app_data_dir.canonicalize()?.as_path())
        || resolved.file_name() != path.file_name()
    {
        return Err(ScannerError::Security(
            "Catalog staging must be a direct, non-linked child of app data".to_string(),
        ));
    }
    std::fs::remove_dir_all(&resolved)?;
    Ok(())
}

pub async fn discard(
    state: &CatalogImportState,
    app_data_dir: &Path,
    token: &str,
) -> Result<(), ScannerError> {
    let mut pending = state.pending.lock().await;
    if let Some(review) = pending.get(token) {
        remove_staging(app_data_dir, &review.staging_path)?;
        pending.remove(token);
    }
    Ok(())
}

pub(super) async fn prune(
    state: &CatalogImportState,
    app_data_dir: &Path,
) -> Result<(), ScannerError> {
    prune_at(state, app_data_dir, Instant::now(), SystemTime::now()).await
}

async fn prune_at(
    state: &CatalogImportState,
    app_data_dir: &Path,
    now: Instant,
    wall_time: SystemTime,
) -> Result<(), ScannerError> {
    let mut pending = state.pending.lock().await;
    let expired: Vec<_> = pending
        .iter()
        .filter(|(_, item)| item.expires_at <= now)
        .map(|(token, item)| (token.clone(), item.staging_path.clone()))
        .collect();
    for (token, path) in expired {
        remove_staging(app_data_dir, &path)?;
        pending.remove(&token);
    }
    let entries = match std::fs::read_dir(app_data_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let path = entry?.path();
        if !is_staging_name(&path) || pending.values().any(|item| item.staging_path == path) {
            continue;
        }
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            continue;
        }
        let age = wall_time
            .duration_since(metadata.modified()?)
            .unwrap_or_default();
        if age >= STAGING_TTL {
            if let Err(error) = remove_staging(app_data_dir, &path) {
                log::warn!("Could not remove orphaned catalog staging: {error}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn orphan_cleanup_preserves_active_pack_backup_and_unknown_directories() {
        let root = tempfile::tempdir().unwrap();
        let orphan = root
            .path()
            .join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4()));
        let retained = [
            "asset-pack",
            ".catalog-pack-backup-fixture",
            ".catalog-import-staging-not-a-uuid",
            "other",
        ];
        for name in retained {
            std::fs::create_dir(root.path().join(name)).unwrap();
        }
        std::fs::create_dir(&orphan).unwrap();
        prune_at(
            &CatalogImportState::default(),
            root.path(),
            Instant::now(),
            SystemTime::now() + STAGING_TTL * 2,
        )
        .await
        .unwrap();
        assert!(!orphan.exists());
        for name in retained {
            assert!(root.path().join(name).exists());
        }
        assert!(remove_staging(root.path(), &root.path().join("asset-pack")).is_err());
    }
}

/// One worker per application; serializes expiry cleanup with preview/install.
#[cfg(not(test))]
pub fn start_cleanup(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let app_data_dir = match app.path().app_data_dir() {
            Ok(path) => path,
            Err(error) => {
                log::error!("Catalog cleanup cannot resolve app data: {error}");
                return;
            }
        };
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        loop {
            interval.tick().await;
            let update_state = app.state::<CatalogUpdateState>();
            let _operation = update_state.0.lock().await;
            if let Err(error) =
                prune(app.state::<CatalogImportState>().inner(), &app_data_dir).await
            {
                log::warn!("Catalog preview cleanup failed: {error}");
            }
        }
    });
}
