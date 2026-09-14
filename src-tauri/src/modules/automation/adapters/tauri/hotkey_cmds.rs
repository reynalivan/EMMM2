//! Tauri commands for hotkey management — bindings, conflicts, and config updates.

use crate::modules::automation::application::hotkeys::manager::HotkeyManager;
use crate::modules::automation::application::hotkeys::{HotkeyConfig, KeyViewerConfig};
use crate::modules::settings::application::config::ConfigService;
use crate::shared::errors::AppError;
use tauri::State;

/// Validate and commit runtime-control settings without persisting a partial
/// shortcut while OS registration is still allowed to fail. F7 remains
/// generated for 3DMigoto, but is never registered as an OS shortcut.
#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri supplies command dependencies as separate State parameters.
pub async fn save_hotkey_configuration(
    app: tauri::AppHandle,
    expected_revision: u64,
    hotkeys: HotkeyConfig,
    keyviewer: KeyViewerConfig,
    config_state: State<'_, ConfigService>,
    hotkey_manager: State<'_, HotkeyManager>,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
) -> Result<crate::modules::settings::adapters::tauri::settings_cmds::SaveSettingsResult, AppError>
{
    let previous = config_state.get_settings();
    hotkey_manager.inner().update_bindings(&app, &hotkeys)?;

    let saved = match config_state.set_hotkey_configuration(expected_revision, hotkeys, keyviewer) {
        Ok(saved) => saved,
        Err(error) => {
            if let Err(restore_error) = hotkey_manager
                .inner()
                .update_bindings(&app, &previous.hotkeys)
            {
                log::error!(
                    "Hotkey settings were not persisted and the prior OS registrations could not be restored: {restore_error}"
                );
            }
            return Err(error);
        }
    };

    let mut sync_warning = None;
    if let Some(game_id) = saved.active_game_id.clone() {
        match crate::modules::system::application::app::post_apply::request_overlay_sync_with_retry_for_game(
            pool.inner(),
            config_state.inner(),
            &game_id,
            crate::modules::system::application::app::post_apply::OverlaySyncCause::SettingsChanged,
        )
        .await
        {
            Ok(result) => {
                if result.requires_retry() {
                    disk_reconcile_state.inner().stage_runtime_effects(
                        &game_id,
                        crate::modules::reconciliation::application::disk_reconcile::types::PendingRuntimeEffects {
                            collections_dirty: false,
                            overlay_refresh: true,
                        },
                    );
                }
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
                disk_reconcile_state.inner().stage_runtime_effects(
                    &game_id,
                    crate::modules::reconciliation::application::disk_reconcile::types::PendingRuntimeEffects {
                        collections_dirty: false,
                        overlay_refresh: true,
                    },
                );
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

/// The key 3DMigoto reloads its fixes on, read from the active game's
/// `d3dx.ini` (falling back to the loader default).
///
/// Toggling from the app moves folders on disk but cannot tell a running game
/// to re-read them: the reload keystroke has to be pressed while the game has
/// focus, so the UI names the key instead of replaying it into whatever window
/// happens to be in front. The in-game hotkey path replays it directly because
/// there the game IS focused (`services::hotkeys::reload`).
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
