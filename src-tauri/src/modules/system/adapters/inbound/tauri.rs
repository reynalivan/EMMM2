
// --- From commands/app/app_cmds.rs ---
use crate::shared::errors::AppError;
use crate::modules::games::domain::models::ConfigStatus;

/// Check if the app has any games configured (determines which screen to show on startup).
#[specta::specta]
#[tauri::command]
pub async fn check_config_status(
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<ConfigStatus, AppError> {
    crate::modules::system::application::app::app_service::check_config_status(pool.inner()).await
}

/// Read the last N lines of the application log.
#[specta::specta]
#[tauri::command]
pub async fn get_logs(
    app: tauri::AppHandle,
    limit: Option<usize>,
    count: Option<usize>,
) -> Result<Vec<String>, AppError> {
    use tauri::Manager;
    let log_dir = app.path().app_log_dir()?;
    let log_path = log_dir.join("emmm.log");

    let lines = limit.or(count).unwrap_or(200);
    crate::modules::system::application::app::log_service::read_last_n_lines(&log_path, lines)
}

/// Open the logs directory in the OS file explorer.
#[specta::specta]
#[tauri::command]
pub async fn open_log_folder(app: tauri::AppHandle) -> Result<(), AppError> {
    use tauri::Manager;
    let log_dir = app.path().app_log_dir()?;

    crate::modules::system::application::app::log_service::open_log_folder_service(&log_dir)
}

/// Reset the application setup by clearing all data from the database.
/// Before clearing, a backup copy of `app.db` is saved to the trash folder.
/// No mod files or folders on disk are deleted — only database records are cleared.
#[specta::specta]
#[tauri::command]
pub async fn reset_database(
    app: tauri::AppHandle,
    config: tauri::State<'_, crate::modules::settings::application::config::ConfigService>,
) -> Result<(), AppError> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir()?;

    config.reset_database(&app_data_dir)
}

/// Check if a given absolute path exists on the disk.
/// Bypasses restrictive Tauri v2 plugin-fs scopes.
#[specta::specta]
#[tauri::command]
pub fn check_path_exists_cmd(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

#[cfg(test)]
#[path = "tests/app_cmds_tests.rs"]
mod tests;


// --- From commands/app/settings_cmds.rs ---
use crate::shared::errors::AppError;
use crate::modules::settings::application::config::{AppSettings, ConfigService};
use tauri::{Emitter, State};

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct SaveSettingsResult {
    pub settings: AppSettings,
    pub sync_warning: Option<crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning>,
}

fn completed_settings_save(
    settings: AppSettings,
    sync_warning: Option<crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning>,
) -> SaveSettingsResult {
    SaveSettingsResult {
        settings,
        sync_warning,
    }
}

#[specta::specta]
#[tauri::command]
pub async fn get_settings(
    app: tauri::AppHandle,
    state: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
) -> Result<AppSettings, AppError> {
    let _activation_guard = disk_reconcile_state.activation_guard().await;
    let settings = state.get_settings();
    if let Some(game) = settings
        .active_game()
        .filter(|game| !game.mod_path.as_os_str().is_empty())
    {
        // Frontend store initialization fans out into workspace, collection,
        // and runtime queries. Hold that fan-out until startup recovery has a
        // terminal result so none of those caches can race the disk scan.
        let outcome = crate::modules::reconciliation::application::disk_reconcile::emit::ensure_initial_disk_recovery(
            &app,
            pool.inner(),
            disk_reconcile_state.inner(),
            &game.id,
        )
        .await;
        match outcome {
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Completed(
                result,
            ) => {
                if result.status
                    != crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileStatus::Applied
                {
                    if let Err(error) = app.emit("disk_reconcile:result", result) {
                        log::warn!(
                            "Startup recovery completed but its terminal result could not be emitted: {error}"
                        );
                    }
                }
            }
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Failed(
                error,
            ) => {
                return Err(AppError::Io(format!(
                    "Disk recovery failed before settings hydration: {error}"
                )));
            }
        }
    }
    // Recovery can overlap source-directory repair, which updates the game
    // path without changing the active ID. Return the post-recovery snapshot.
    Ok(state.get_settings())
}

#[specta::specta]
#[tauri::command]
pub async fn save_settings(
    app: tauri::AppHandle,
    settings: AppSettings,
    state: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
) -> Result<SaveSettingsResult, AppError> {
    let _activation_guard = disk_reconcile_state.activation_guard().await;
    let previous = state.get_settings();
    if settings.active_game_id != previous.active_game_id {
        return Err(AppError::Validation(
            "Use the active-game command to change the active game safely".to_string(),
        ));
    }
    let saved = state.save_settings(settings)?;
    let mut sync_warning = None;
    if saved.safety.keywords != previous.safety.keywords {
        if let Some(game_id) = saved.active_game_id.as_deref() {
            let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
                crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile(
                    &app,
                    pool.inner(),
                    game_id,
                )
                .await,
            );
            if let Some(result) = settlement.reconcile {
                if !result.status.applied() {
                    if let Err(error) = app.emit("disk_reconcile:result", result) {
                        log::warn!(
                            "Safety keywords were saved but the blocked reconcile result could not be emitted: {error}"
                        );
                    }
                }
            }
            sync_warning = settlement.sync_warning;
        }
    }
    Ok(completed_settings_save(saved, sync_warning))
}

#[specta::specta]
#[tauri::command]
pub async fn set_active_game(
    app: tauri::AppHandle,
    game_id: Option<String>,
    state: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
) -> Result<(), AppError> {
    let _activation_guard = disk_reconcile_state.activation_guard().await;
    if let Some(game_id) = game_id.as_deref() {
        // Every activation is a new disk-authority boundary. The first
        // workspace read must scan this game's current folder before using a
        // DB projection that may predate changes made while the game was idle.
        disk_reconcile_state.reset_initial_recovery(game_id);
        let recovery_result =
            match crate::modules::reconciliation::application::disk_reconcile::emit::ensure_initial_disk_recovery(
                &app,
                pool.inner(),
                disk_reconcile_state.inner(),
                game_id,
            )
            .await
            {
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Completed(
                    result,
                ) => Ok(result),
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Failed(
                    error,
                ) => Err(AppError::Io(format!(
                    "Disk recovery failed while activating game '{game_id}': {error}"
                ))),
            };
        match recovery_result {
            Ok(result) => {
                // Publish the selection only after the target game's disk
                // recovery is terminal. A failed activation therefore needs
                // no stale-snapshot rollback at all.
                state.set_active_game(Some(game_id.to_string()))?;
                if result.status
                    != crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileStatus::Applied
                {
                    if let Err(error) = app.emit("disk_reconcile:result", result) {
                        log::warn!(
                            "Game activation reconciled but its resolution event could not be emitted: {error}"
                        );
                    }
                }
            }
            Err(error) => {
                return Err(error);
            }
        }
        if let Err(error) =
            crate::modules::system::application::app::post_apply::trigger_overlay_refresh(pool.inner(), &state).await
        {
            log::warn!("Active game changed but overlay refresh failed: {error}");
        }
    } else {
        state.set_active_game(None)?;
    }
    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn set_auto_close_launcher(
    enabled: bool,
    state: State<'_, ConfigService>,
) -> Result<(), AppError> {
    state.set_auto_close_launcher(enabled)
}

#[specta::specta]
#[tauri::command]
pub async fn run_maintenance(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<u64, AppError> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir()?;
    crate::modules::system::application::app::maintenance_service::run_maintenance_counts(pool.inner(), &app_data_dir)
        .await
}

#[specta::specta]
#[tauri::command]
pub async fn clear_old_thumbnails() -> Result<u64, AppError> {
    use crate::platform::images::thumbnail_cache::{ThumbnailCache, THUMBNAIL_RETENTION_DAYS};
    let pruned = ThumbnailCache::clear_old_cache(THUMBNAIL_RETENTION_DAYS)?;
    Ok(pruned as u64)
}

#[cfg(test)]
mod tests {
    use super::completed_settings_save;
    use crate::modules::settings::application::config::AppSettings;
    use crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile;
    use crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind;

    #[test]
    fn persisted_settings_remain_success_when_follow_up_reconcile_fails() {
        let mut settings = AppSettings::default();
        settings.safety.keywords = vec!["unsafe".to_string()];
        let settlement = settle_committed_reconcile(Err(crate::shared::errors::AppError::Io(
            "disk unavailable".to_string(),
        )));

        let result = completed_settings_save(settings, settlement.sync_warning);

        assert_eq!(result.settings.safety.keywords, ["unsafe"]);
        assert_eq!(
            result.sync_warning.expect("typed warning").kind,
            CommittedMutationSyncWarningKind::ReconcileFailed
        );
    }
}


// --- From commands/app/theme_cmds.rs ---
use crate::shared::errors::AppError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct ThemeConfig {
    pub colors: std::collections::HashMap<String, String>,
    pub glass: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct CustomTheme {
    pub id: String,
    pub label: String,
    pub config: ThemeConfig,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct ThemeMetadata {
    pub id: String,
    pub label: String,
}

fn get_themes_dir(app_handle: &AppHandle) -> Result<PathBuf, AppError> {
    let app_data_dir = app_handle.path().app_data_dir()?;
    let themes_dir = app_data_dir.join("themes");

    if !themes_dir.exists() {
        fs::create_dir_all(&themes_dir)?;
    }

    Ok(themes_dir)
}

#[tauri::command]
#[specta::specta]
pub async fn list_custom_themes(app_handle: AppHandle) -> Result<Vec<ThemeMetadata>, AppError> {
    let themes_dir = get_themes_dir(&app_handle)?;
    let mut themes = Vec::new();

    if let Ok(entries) = fs::read_dir(themes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(theme) = serde_json::from_str::<CustomTheme>(&content) {
                        themes.push(ThemeMetadata {
                            id: theme.id,
                            label: theme.label,
                        });
                    }
                }
            }
        }
    }

    Ok(themes)
}

#[tauri::command]
#[specta::specta]
pub async fn load_custom_theme(app_handle: AppHandle, id: String) -> Result<CustomTheme, AppError> {
    let themes_dir = get_themes_dir(&app_handle)?;
    let theme_path = themes_dir.join(format!("{}.json", id));

    if !theme_path.exists() {
        return Err(AppError::NotFound(format!("Theme '{}' not found", id)));
    }

    let content = fs::read_to_string(theme_path)?;
    let theme = serde_json::from_str::<CustomTheme>(&content)?;

    Ok(theme)
}

#[tauri::command]
#[specta::specta]
pub async fn save_custom_theme(app_handle: AppHandle, theme: CustomTheme) -> Result<(), AppError> {
    let themes_dir = get_themes_dir(&app_handle)?;
    let theme_path = themes_dir.join(format!("{}.json", theme.id));

    let content = serde_json::to_string_pretty(&theme)?;
    fs::write(theme_path, content)?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_custom_theme(app_handle: AppHandle, id: String) -> Result<(), AppError> {
    let themes_dir = get_themes_dir(&app_handle)?;
    let theme_path = themes_dir.join(format!("{}.json", id));

    if theme_path.exists() {
        crate::platform::fs::recycle_bin::move_path_to_recycle_bin(&theme_path)?;
    }

    Ok(())
}


// --- From commands/app/update_cmds.rs ---
use crate::shared::errors::AppError;
use crate::modules::updates::application::update::{asset_fetch, metadata_sync};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

/// Check for metadata updates from the remote manifest.
///
/// Returns whether an update was applied and the current version.
#[specta::specta]
#[tauri::command]
pub async fn check_metadata_update(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<metadata_sync::MetadataSyncResult, AppError> {
    let result = metadata_sync::check_and_sync_metadata(&pool).await;
    Ok(result)
}

/// Fetch a missing asset file from the remote CDN.
///
/// Returns the local path to the cached asset, or null if the fetch failed.
#[specta::specta]
#[tauri::command]
pub async fn fetch_missing_asset(
    app: AppHandle,
    asset_name: String,
) -> Result<Option<String>, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("Failed to get app data dir: {e}")))?;

    let cache_dir = app_data_dir.join("cache");
    let result = asset_fetch::fetch_asset_if_missing(&asset_name, &cache_dir).await;

    Ok(result.map(|p| p.to_string_lossy().to_string()))
}

#[cfg(test)]
#[path = "tests/update_cmds_tests.rs"]
mod tests;

