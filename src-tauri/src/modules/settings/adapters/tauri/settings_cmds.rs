use crate::modules::games::domain::models::GameType;
use crate::modules::settings::application::config::{AppSettings, ConfigService};
use crate::platform::fs::guard::validate_path;
use crate::shared::errors::AppError;
use secrecy::ExposeSecret;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{Emitter, State};

const AI_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_AI_BASE_URL: &str = "https://api.openai.com/v1";

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

fn add_runtime_sync_warning(
    current: &mut Option<
        crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning,
    >,
    kind: crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind,
    message: String,
) {
    use crate::modules::reconciliation::application::disk_reconcile::types::{
        CommittedMutationSyncWarning, CommittedMutationSyncWarningKind,
    };

    let warning = CommittedMutationSyncWarning { kind, message };
    match current {
        Some(existing) => {
            existing.message = format!("{} {}", existing.message, warning.message);
            if matches!(
                warning.kind,
                CommittedMutationSyncWarningKind::ManualReloadRequired
            ) {
                existing.kind = warning.kind;
            }
        }
        None => *current = Some(warning),
    }
}

/// A Mods watcher improves freshness for later external edits, but publishing
/// the current overlay snapshot must not depend on it starting successfully.
async fn sync_after_watcher_start<F, Fut>(
    watcher_start: Result<(), AppError>,
    sync_warning: &mut Option<
        crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning,
    >,
    request_sync: F,
) -> Result<crate::modules::system::application::app::post_apply::RuntimeSyncResult, AppError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<
        Output = Result<
            crate::modules::system::application::app::post_apply::RuntimeSyncResult,
            AppError,
        >,
    >,
{
    if let Err(error) = watcher_start {
        log::warn!("Settings changed a runtime root but the watcher could not restart: {error}");
        add_runtime_sync_warning(
            sync_warning,
            crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind::WatcherUnavailable,
            format!(
                "Settings were committed, but external INI changes will not sync until the Mods watcher restarts: {error}"
            ),
        );
    }
    request_sync().await
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
pub fn get_keyviewer_runtime_diagnostics(
    game_id: String,
    state: State<'_, ConfigService>,
) -> Result<
    crate::modules::system::application::app::post_apply::KeyViewerRuntimeDiagnostics,
    AppError,
> {
    crate::modules::system::application::app::post_apply::keyviewer_runtime_diagnostics(
        &state.get_settings(),
        &game_id,
    )
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
    watcher: State<'_, crate::modules::workspace::application::scanner::watcher::WatcherState>,
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
    let active_game_id = saved.active_game_id.as_deref();
    let previous_active = previous
        .active_game()
        .filter(|game| Some(game.id.as_str()) == active_game_id);
    let saved_active = saved.active_game();
    let runtime_root_changed = matches!(
        (previous_active, saved_active),
        (Some(previous_game), Some(saved_game))
            if previous_game.instance_path != saved_game.instance_path
                || previous_game.mod_path != saved_game.mod_path
                || previous_game.game_type != saved_game.game_type
    );
    let overlay_cause = match (previous_active, saved_active) {
        (Some(previous_game), Some(saved_game))
            if previous_game.instance_path != saved_game.instance_path
                || previous_game.game_type != saved_game.game_type =>
        {
            Some(crate::modules::system::application::app::post_apply::OverlaySyncCause::ImporterRootChanged)
        }
        (Some(previous_game), Some(saved_game)) if previous_game.mod_path != saved_game.mod_path => {
            Some(crate::modules::system::application::app::post_apply::OverlaySyncCause::ModsRootChanged)
        }
        (Some(_), Some(_))
            if saved.hotkeys.toggle_overlay != previous.hotkeys.toggle_overlay
                || saved.hotkeys.safe_mode != previous.hotkeys.safe_mode
                || saved.hotkeys.next_preset != previous.hotkeys.next_preset
                || saved.hotkeys.prev_preset != previous.hotkeys.prev_preset
                || saved.keyviewer.enabled != previous.keyviewer.enabled =>
        {
            Some(crate::modules::system::application::app::post_apply::OverlaySyncCause::SettingsChanged)
        }
        _ => None,
    };
    let watcher_start = runtime_root_changed.then(|| {
        saved.active_game().map(|game| {
            crate::modules::workspace::application::scanner::watcher::lifecycle::start_watcher(
                app.clone(),
                watcher.inner(),
                pool.inner().clone(),
                game.mod_path.to_string_lossy().into_owned(),
                game.id.clone(),
            )
        })
    });
    if let (Some(game_id), Some(cause)) = (active_game_id, overlay_cause) {
        let request_sync = || {
            crate::modules::system::application::app::post_apply::request_overlay_sync_with_retry_for_game(
                pool.inner(),
                state.inner(),
                game_id,
                cause,
            )
        };
        let sync_result = match watcher_start.flatten() {
            Some(result) => {
                sync_after_watcher_start(
                    result.map_err(AppError::from),
                    &mut sync_warning,
                    request_sync,
                )
                .await
            }
            None => request_sync().await,
        };
        match sync_result {
            Ok(result) => {
                if result.requires_retry() {
                    disk_reconcile_state.inner().stage_runtime_effects(
                        game_id,
                        crate::modules::reconciliation::application::disk_reconcile::types::PendingRuntimeEffects {
                            collections_dirty: false,
                            overlay_refresh: true,
                        },
                    );
                }
                if let Some(message) = result.diagnostic_message() {
                    log::warn!("Settings were committed but KeyViewer sync is pending: {message}");
                    add_runtime_sync_warning(
                        &mut sync_warning,
                        if result.needs_manual_reload() {
                            crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind::ManualReloadRequired
                        } else {
                            crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind::RuntimeSyncPending
                        },
                        message,
                    );
                }
            }
            Err(error) => {
                disk_reconcile_state.inner().stage_runtime_effects(
                    game_id,
                    crate::modules::reconciliation::application::disk_reconcile::types::PendingRuntimeEffects {
                        collections_dirty: false,
                        overlay_refresh: true,
                    },
                );
                log::warn!("Settings were committed but KeyViewer sync is pending: {error}");
                add_runtime_sync_warning(
                    &mut sync_warning,
                    crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind::RuntimeSyncPending,
                    error.to_string(),
                );
            }
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
        if let Err(error) = crate::modules::system::application::app::post_apply::request_overlay_sync_for_game(
            pool.inner(),
            state.inner(),
            game_id,
            crate::modules::system::application::app::post_apply::OverlaySyncCause::GameActivated,
        )
        .await
        .and_then(
            crate::modules::system::application::app::post_apply::RuntimeSyncResult::ensure_success,
        )
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

/// Persist explicit diagnostics consent. Opting out clears every queued
/// aggregate and crash envelope before the setting change is reported as done.
#[specta::specta]
#[tauri::command]
pub async fn set_telemetry_enabled(
    enabled: bool,
    state: State<'_, ConfigService>,
    telemetry: State<'_, crate::modules::system::application::telemetry::TelemetryStore>,
) -> Result<AppSettings, AppError> {
    if !enabled {
        telemetry.purge_all().await.map_err(|_| {
            AppError::Db("Could not clear queued diagnostics while opting out".to_string())
        })?;
    }
    state.set_telemetry_enabled(enabled)
}

#[specta::specta]
#[tauri::command]
pub fn set_mod_viewer_executable(
    path: Option<String>,
    state: State<'_, ConfigService>,
) -> Result<AppSettings, AppError> {
    state.set_mod_viewer_executable(path.map(PathBuf::from))
}

#[specta::specta]
#[tauri::command]
pub async fn launch_mod_viewer(
    game_id: String,
    mod_folder: String,
    config: State<'_, ConfigService>,
) -> Result<crate::modules::library::api::mod_health::types::ModViewerLaunchReceipt, AppError> {
    let (executable, game_type) = mod_viewer_launch_inputs(&config, &game_id)?;
    ensure_mod_viewer_supports(game_type)?;
    ensure_configured_mod_viewer_executable(&executable)?;

    let requested_mod_folder = mod_folder;
    let mod_folder = validate_mod_viewer_folder(&config, &game_id, &requested_mod_folder)?;
    let receipt_game_id = game_id.clone();
    let receipt_folder = requested_mod_folder.clone();
    let receipt_root = mod_folder.clone();
    let receipt = tokio::task::spawn_blocking(move || {
        crate::modules::library::api::mod_health::service::create_mod_viewer_launch_receipt(
            receipt_game_id,
            receipt_folder,
            &receipt_root,
        )
    })
    .await??;

    let disabled_ini = should_pass_disabled_ini(&mod_folder)?;
    let mut command = build_mod_viewer_command(&executable, &mod_folder, disabled_ini);
    command.spawn().map_err(|error| {
        AppError::Io(format!(
            "Failed to start 3DMigoto Mod Viewer: {error}. Check Settings > Integrations."
        ))
    })?;

    Ok(receipt)
}

fn build_mod_viewer_command(
    executable: &Path,
    mod_folder: &Path,
    disabled_ini: bool,
) -> std::process::Command {
    let mut command = std::process::Command::new(executable);
    command.arg(mod_folder);
    if disabled_ini {
        command.arg("--disabled-ini");
    }
    command
}

fn validate_mod_viewer_folder(
    config: &ConfigService,
    game_id: &str,
    mod_folder: &str,
) -> Result<PathBuf, AppError> {
    let mod_folder = validate_path(config, game_id, mod_folder)?.into_path_buf();
    if !mod_folder.is_dir() {
        return Err(AppError::Validation(
            "3DMigoto Mod Viewer target must be a mod folder".to_string(),
        ));
    }
    Ok(mod_folder)
}

fn mod_viewer_launch_inputs(
    config: &ConfigService,
    game_id: &str,
) -> Result<(PathBuf, GameType), AppError> {
    config.with_settings(|settings| {
        let executable = settings
            .external_tools
            .mod_viewer_executable
            .clone()
            .ok_or_else(|| {
                AppError::Validation(
                    "3DMigoto Mod Viewer is not configured. Select its executable in Settings > Integrations."
                        .to_string(),
                )
            })?;
        let game_type = settings
            .games
            .iter()
            .find(|game| game.id == game_id)
            .map(|game| game.game_type)
            .ok_or_else(|| AppError::NotFound(format!("Game not found: {game_id}")))?;
        Ok((executable, game_type))
    })
}

fn ensure_mod_viewer_supports(game_type: GameType) -> Result<(), AppError> {
    match game_type {
        GameType::GIMI | GameType::ZZMI | GameType::WWMI | GameType::SRMI => Ok(()),
        GameType::EFMI => Err(AppError::Validation(
            "3DMigoto Mod Viewer is not available for EFMI".to_string(),
        )),
    }
}

fn ensure_configured_mod_viewer_executable(executable: &Path) -> Result<(), AppError> {
    crate::modules::settings::application::config::validate_mod_viewer_executable(executable)
        .map_err(|error| {
            AppError::Validation(format!(
                "Configured 3DMigoto Mod Viewer executable is no longer valid ({error}). Select it again in Settings > Integrations."
            ))
        })
}

fn should_pass_disabled_ini(mod_folder: &Path) -> Result<bool, AppError> {
    let mut has_disabled_ini = false;
    for entry in std::fs::read_dir(mod_folder)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("ini"))
        {
            continue;
        }

        let name = entry.file_name();
        if name
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("DISABLED")
        {
            has_disabled_ini = true;
        } else {
            return Ok(false);
        }
    }
    Ok(has_disabled_ini)
}

#[specta::specta]
#[tauri::command]
pub async fn run_maintenance(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<u64, AppError> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir()?;
    crate::modules::system::application::app::maintenance_service::run_maintenance_counts(
        pool.inner(),
        &app_data_dir,
    )
    .await
}

#[specta::specta]
#[tauri::command]
pub async fn clear_old_thumbnails() -> Result<u64, AppError> {
    use crate::platform::images::thumbnail_cache::{ThumbnailCache, THUMBNAIL_RETENTION_DAYS};
    let pruned = ThumbnailCache::clear_old_cache(THUMBNAIL_RETENTION_DAYS)?;
    Ok(pruned as u64)
}

#[specta::specta]
#[tauri::command]
pub fn set_ai_api_key(
    api_key: String,
    credentials: State<'_, crate::platform::security::credential_store::CredentialStore>,
    state: State<'_, ConfigService>,
) -> Result<AppSettings, AppError> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(AppError::Validation(
            "AI API key cannot be empty".to_string(),
        ));
    }
    if api_key.len() > 8_192 {
        return Err(AppError::Validation(
            "AI API key exceeds the maximum supported length".to_string(),
        ));
    }
    if api_key.contains('\0') {
        return Err(AppError::Validation(
            "AI API key contains an invalid null character".to_string(),
        ));
    }

    credentials.set_ai_api_key(&secrecy::SecretString::from(api_key.to_owned()))?;
    state.set_ai_key_status(true);
    Ok(state.get_settings())
}

#[specta::specta]
#[tauri::command]
pub fn delete_ai_api_key(
    credentials: State<'_, crate::platform::security::credential_store::CredentialStore>,
    state: State<'_, ConfigService>,
) -> Result<AppSettings, AppError> {
    credentials.delete_ai_api_key()?;
    state.set_ai_key_status(false);
    Ok(state.get_settings())
}

/// Verify that the configured AI endpoint is reachable and does not reject the stored key.
///
/// The deliberately incomplete payload exercises authentication and routing without invoking a
/// model. Response bodies and transport errors are not returned because they may contain secrets.
#[specta::specta]
#[tauri::command]
pub async fn test_ai_connection(
    credentials: State<'_, crate::platform::security::credential_store::CredentialStore>,
    state: State<'_, ConfigService>,
) -> Result<(), AppError> {
    let settings = state.get_settings();
    let configured_url = settings
        .ai
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_AI_BASE_URL);
    let mut url = reqwest::Url::parse(configured_url)
        .map_err(|_| AppError::Validation("AI base URL is invalid".to_string()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::Validation(
            "AI base URL must use HTTP or HTTPS".to_string(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::Validation(
            "AI base URL must not contain embedded credentials".to_string(),
        ));
    }
    let host = url.host_str().unwrap_or_default();
    let loopback_host = host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    if url.scheme() == "http" && !loopback_host {
        return Err(AppError::Validation(
            "AI base URL must use HTTPS unless it targets localhost".to_string(),
        ));
    }
    let base_path = url.path().trim_end_matches('/');
    let models_path = if let Some(prefix) = base_path.strip_suffix("/chat/completions") {
        format!("{prefix}/models")
    } else if let Some(prefix) = base_path.strip_suffix("/responses") {
        format!("{prefix}/models")
    } else {
        format!("{base_path}/models")
    };
    url.set_path(&models_path);

    let api_key = credentials.get_ai_api_key()?.ok_or_else(|| {
        AppError::Validation("Configure an AI API key before testing the connection".to_string())
    })?;
    let bearer = secrecy::SecretString::from(format!("Bearer {}", api_key.expose_secret()));
    let mut authorization = reqwest::header::HeaderValue::from_bytes(
        bearer.expose_secret().as_bytes(),
    )
    .map_err(|_| AppError::Validation("AI API key is not a valid HTTP header".to_string()))?;
    authorization.set_sensitive(true);

    let client = reqwest::Client::builder()
        .timeout(AI_CONNECTION_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| AppError::Internal("Could not initialize AI connection test".to_string()))?;
    let response = client
        .get(url)
        .header(reqwest::header::AUTHORIZATION, authorization)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                AppError::Io("AI connection test timed out".to_string())
            } else {
                AppError::Io("Could not reach the AI service".to_string())
            }
        })?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    if matches!(
        status,
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN
    ) {
        return Err(AppError::Validation(
            "AI service rejected the configured API key".to_string(),
        ));
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::Validation(
            "AI endpoint was not found".to_string(),
        ));
    }
    if status.is_server_error() || status == reqwest::StatusCode::REQUEST_TIMEOUT {
        return Err(AppError::Internal(format!(
            "AI service is unavailable (HTTP {status})"
        )));
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(AppError::Validation(
            "AI service rate limit prevented the connection test".to_string(),
        ));
    }
    Err(AppError::Validation(format!(
        "AI service rejected the connection test (HTTP {status})"
    )))
}

#[cfg(test)]
mod tests {
    use super::{
        build_mod_viewer_command, completed_settings_save, ensure_mod_viewer_supports,
        should_pass_disabled_ini, sync_after_watcher_start, validate_mod_viewer_folder,
    };
    use crate::modules::games::domain::models::{GameType, LaunchMode};
    use crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile;
    use crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind;
    use crate::modules::settings::application::config::{AppSettings, ConfigService, GameConfig};

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

    #[tokio::test]
    async fn watcher_start_failure_still_allows_a_published_overlay_sync() {
        let mut warning = None;
        let result = sync_after_watcher_start(
            Err(crate::shared::errors::AppError::Io(
                "injected watcher startup failure".to_string(),
            )),
            &mut warning,
            || async {
                Ok(
                    crate::modules::system::application::app::post_apply::RuntimeSyncResult {
                        cause: crate::modules::system::application::app::post_apply::OverlaySyncCause::ModsRootChanged,
                        publication: crate::modules::system::application::app::post_apply::RuntimeSyncPublication::Published,
                        reload: crate::modules::system::application::app::post_apply::RuntimeReloadOutcome::NotRequired,
                        failure: None,
                    },
                )
            },
        )
        .await
        .expect("watcher failure must not short-circuit overlay publication");

        assert_eq!(
            result.publication,
            crate::modules::system::application::app::post_apply::RuntimeSyncPublication::Published
        );
        assert_eq!(
            warning.expect("watcher failure should be visible").kind,
            CommittedMutationSyncWarningKind::WatcherUnavailable
        );
    }

    #[test]
    fn disabled_ini_flag_uses_only_direct_disabled_ini_files() {
        let folder = tempfile::tempdir().expect("temporary mod folder should create");
        std::fs::write(folder.path().join("DISABLED_mod.ini"), "[Constants]")
            .expect("disabled ini fixture should write");
        assert!(
            should_pass_disabled_ini(folder.path()).expect("direct disabled ini should inspect")
        );

        let nested = folder.path().join("nested");
        std::fs::create_dir(&nested).expect("nested folder should create");
        std::fs::write(nested.join("active.ini"), "[Constants]")
            .expect("nested active ini fixture should write");
        assert!(
            should_pass_disabled_ini(folder.path()).expect("nested ini should be ignored"),
            "nested files must not affect the direct-file launch argument"
        );
    }

    #[test]
    fn active_ini_wins_over_disabled_ini_and_folder_name() {
        let folder = tempfile::Builder::new()
            .prefix("DISABLED ")
            .tempdir()
            .expect("temporary disabled-folder-named mod should create");
        std::fs::write(folder.path().join("DISABLED_mod.ini"), "[Constants]")
            .expect("disabled ini fixture should write");
        std::fs::write(folder.path().join("active.INI"), "[Constants]")
            .expect("active ini fixture should write");

        assert!(
            !should_pass_disabled_ini(folder.path())
                .expect("active and disabled ini should inspect"),
            "a folder name starting with DISABLED must not force --disabled-ini"
        );
    }

    #[test]
    fn only_supported_mod_viewer_games_can_launch() {
        for game_type in [
            GameType::GIMI,
            GameType::ZZMI,
            GameType::WWMI,
            GameType::SRMI,
        ] {
            assert!(ensure_mod_viewer_supports(game_type).is_ok());
        }
        assert!(ensure_mod_viewer_supports(GameType::EFMI).is_err());
    }

    #[test]
    fn mod_viewer_command_passes_executable_and_folder_as_separate_arguments() {
        let executable = std::path::Path::new("C:/Tools/3DMigoto Mod Viewer.exe");
        let mod_folder = std::path::Path::new("C:/Mods/Character With Spaces");
        let command = build_mod_viewer_command(executable, mod_folder, true);

        assert_eq!(command.get_program(), executable.as_os_str());
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![
                mod_folder.as_os_str(),
                std::ffi::OsStr::new("--disabled-ini")
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mod_viewer_folder_validation_rejects_paths_outside_the_game_mods_root() {
        let pool = crate::test_utils::init_test_db().await.pool;
        let service = ConfigService::new_for_test(pool);
        let temp = tempfile::tempdir().expect("temporary game directory should create");
        let mods_root = temp.path().join("Mods");
        let mod_folder = mods_root.join("Valid Mod");
        let outside_folder = temp.path().join("Outside Mod");
        std::fs::create_dir_all(&mod_folder).expect("mod folder should create");
        std::fs::create_dir(&outside_folder).expect("outside folder should create");

        let mut settings = AppSettings::default();
        settings.games.push(GameConfig {
            id: "game-a".to_string(),
            name: "Game A".to_string(),
            game_type: GameType::GIMI,
            instance_path: temp.path().to_path_buf(),
            mod_path: mods_root,
            ready_to_move_path: None,
            launch_mode: LaunchMode::Standalone,
            game_exe: None,
            loader_exe: None,
            xxmi_launcher_exe: None,
            launch_args: None,
            warnings: Vec::new(),
        });
        service
            .save_settings(settings)
            .expect("game settings should save");

        assert!(
            validate_mod_viewer_folder(&service, "game-a", &mod_folder.to_string_lossy(),).is_ok()
        );
        assert!(
            validate_mod_viewer_folder(&service, "game-a", &outside_folder.to_string_lossy(),)
                .is_err()
        );
    }
}
