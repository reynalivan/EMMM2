//! Tauri commands for hotkey management — bindings, conflicts, and config updates.

use crate::modules::automation::application::hotkeys::manager::{
    HotkeyManager, ShortcutRegistrationSink, TauriShortcutRegistrationSink,
};
use crate::modules::automation::application::hotkeys::{HotkeyConfig, KeyViewerConfig};
use crate::modules::settings::application::config::ConfigService;
use crate::shared::errors::AppError;
use tauri::State;

/// Validate and commit runtime-control settings without persisting a partial
/// shortcut while OS registration is still allowed to fail. Every configured
/// action is registered while enabled. Workspace actions dispatch globally;
/// only KeyViewer input replay requires the active game in the foreground.
#[specta::specta]
#[tauri::command]
pub async fn save_hotkey_configuration(
    app: tauri::AppHandle,
    expected_revision: u64,
    hotkeys: HotkeyConfig,
    keyviewer: KeyViewerConfig,
    config_state: State<'_, ConfigService>,
    hotkey_manager: State<'_, HotkeyManager>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<crate::modules::settings::adapters::tauri::settings_cmds::SaveSettingsResult, AppError>
{
    let mut sink = TauriShortcutRegistrationSink::new(&app);
    let saved = commit_hotkey_configuration_with_sink(
        expected_revision,
        hotkeys,
        keyviewer,
        config_state.inner(),
        hotkey_manager.inner(),
        &mut sink,
    )?;

    let mut sync_warning = None;
    if let Some(game_id) = saved.active_game_id.clone() {
        match crate::modules::reconciliation::api::request_runtime_sync_and_wait(
            &app,
            pool.inner(),
            &game_id,
            crate::modules::reconciliation::api::RuntimeSyncCause::SettingsChanged,
        )
        .await
        {
            Ok(result) => {
                sync_warning = result.diagnostic_message().map(|message| {
                    crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning {
                        kind: if result.needs_manual_reload() {
                            crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind::ManualReloadRequired
                        } else {
                            crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind::RuntimeSyncPending
                        },
                        message,
                    }
                });
            }
            Err(error) => {
                log::warn!("Hotkey settings committed but overlay refresh is pending: {error}");
                sync_warning = Some(
                    crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning {
                        kind: crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind::RuntimeSyncPending,
                        message: error.to_string(),
                    },
                );
            }
        }
    }

    Ok(
        crate::modules::settings::adapters::tauri::settings_cmds::SaveSettingsResult {
            settings: saved,
            sync_warning,
        },
    )
}

pub(crate) fn commit_hotkey_configuration_with_sink(
    expected_revision: u64,
    hotkeys: HotkeyConfig,
    keyviewer: KeyViewerConfig,
    config_state: &ConfigService,
    hotkey_manager: &HotkeyManager,
    sink: &mut impl ShortcutRegistrationSink,
) -> Result<crate::modules::settings::application::config::AppSettings, AppError> {
    let lifecycle = hotkey_manager.registration_lifecycle();
    if config_state.get_settings().revision != expected_revision {
        return Err(AppError::Validation(
            "Settings changed since this screen was loaded. Refresh and retry your edit."
                .to_string(),
        ));
    }
    hotkey_manager.update_bindings_with_sink(&lifecycle, sink, &hotkeys)?;

    match config_state.set_hotkey_configuration(expected_revision, hotkeys, keyviewer) {
        Ok(saved) => Ok(saved),
        Err(error) => {
            let latest = config_state.get_settings();
            if let Err(restore_error) =
                hotkey_manager.update_bindings_with_sink(&lifecycle, sink, &latest.hotkeys)
            {
                log::error!(
                    "Hotkey settings were not persisted and the latest committed OS registrations could not be restored: {restore_error}"
                );
                return Err(AppError::Internal(format!(
                    "Hotkey settings were not persisted ({error}); restoring committed OS registrations also failed ({restore_error}). Refresh settings and retry registration."
                )));
            }
            Err(error)
        }
    }
}

/// The key 3DMigoto reloads its fixes on, read from the active game's
/// `d3dx.ini` (falling back to the loader default).
///
/// Toggling from the app moves folders on disk but cannot tell a running game
/// to re-read them: the reload keystroke has to be pressed while the game has
/// focus, so the UI names the key instead of replaying it into whatever window
/// happens to be in front. Global workspace hotkeys only replay reload input
/// when the shared game detector confirms the selected game is focused.
#[specta::specta]
#[tauri::command]
pub async fn get_reload_key(config_state: State<'_, ConfigService>) -> Result<String, AppError> {
    use crate::modules::automation::application::keyviewer::generator;

    config_state.with_settings(|settings| {
        let Some(game) = settings.active_game() else {
            return Err(AppError::Validation(
                "NeedsManualReload: no active game is configured".to_string(),
            ));
        };
        generator::discover_reload_key_for_game(game).map(|config| config.reload_config_key)
    })
}
