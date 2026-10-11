use super::watcher_runtime::{
    authority_catch_up_backoff, emit_activation_failure, emit_activation_failure_with_phase,
    pending_activation_target, prepare_current_activation_reuse, publish_activation_outcome,
    watcher_identity_cache_failure, RecoveredActivationPublication, WatcherActivation,
    WATCHER_IDENTITY_COVERAGE_ERROR,
};
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use tauri::Manager;

pub(crate) fn try_reuse_active_watcher_for_activation(
    app: &tauri::AppHandle,
    watcher: &WatcherState,
    game_id: &str,
    mods_root: &std::path::Path,
    runtime_config_path: &std::path::Path,
    activation_generation: u64,
) -> bool {
    let state = app.state::<crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState>();
    let coordinator = app.state::<crate::modules::mutation::api::MutationCoordinator>();
    let Some(reused) = prepare_current_activation_reuse(
        state.inner(),
        watcher,
        coordinator.inner(),
        game_id,
        mods_root,
        runtime_config_path,
        activation_generation,
    ) else {
        return false;
    };
    matches!(
        publish_activation_outcome(
            app,
            game_id,
            reused.session(),
            watcher.suppressor.as_ref(),
            mods_root,
            reused.activation(),
            reused.result()
        ),
        RecoveredActivationPublication::Published
    )
}

pub(crate) fn publish_pending_activation_when_recovered(
    app: tauri::AppHandle,
    game_id: String,
    activation: WatcherActivation,
    binding: crate::modules::reconciliation::api::disk_reconcile::orchestrator::InitialRecoveryBinding,
) {
    tokio::spawn(async move {
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::{
            DiskReconcileState, InitialRecoveryOutcome,
        };
        let state = app.state::<DiskReconcileState>();
        let config = app.state::<crate::modules::settings::api::config::ConfigService>();
        let outcome = loop {
            let settings = config.get_settings();
            let Some(game) =
                pending_activation_target(state.inner(), &settings, &game_id, activation, || {
                    emit_activation_failure(
                        &app,
                        &game_id,
                        activation,
                        "The selected game was removed or deselected during recovery".into(),
                    );
                })
            else {
                return;
            };
            if !state.initial_recovery_binding_is_current(
                &game_id,
                activation.recovery_generation,
                &binding,
                &game.mod_path,
                &game.instance_path.join("d3dx.ini"),
            ) {
                emit_activation_failure(
                    &app,
                    &game_id,
                    activation,
                    "Initial recovery root or runtime coverage changed; retry activation".into(),
                );
                return;
            }
            if let Ok(outcome) = tokio::time::timeout(
                std::time::Duration::from_millis(250),
                state.wait_initial_recovery_generation(
                    &game_id,
                    activation.recovery_generation,
                    &binding,
                ),
            )
            .await
            {
                break outcome;
            }
        };
        if !state.activation_is_current(Some(&game_id), activation.activation_generation) {
            return;
        }
        let result = match outcome {
            Some(InitialRecoveryOutcome::Completed(result)) => result,
            Some(InitialRecoveryOutcome::Failed(error)) => {
                emit_activation_failure(&app, &game_id, activation, error);
                return;
            }
            None => {
                emit_activation_failure(
                    &app,
                    &game_id,
                    activation,
                    "Initial recovery ownership changed; retry activation".into(),
                );
                return;
            }
        };
        let mut retry_round = 0;
        loop {
            let settings = config.get_settings();
            let Some(game) =
                pending_activation_target(state.inner(), &settings, &game_id, activation, || {
                    emit_activation_failure(
                        &app,
                        &game_id,
                        activation,
                        "The selected game was removed or deselected during recovery".into(),
                    );
                })
            else {
                return;
            };
            let runtime_config_path = game.instance_path.join("d3dx.ini");
            if !state.initial_recovery_binding_is_current(
                &game_id,
                activation.recovery_generation,
                &binding,
                &game.mod_path,
                &runtime_config_path,
            ) {
                emit_activation_failure(
                    &app,
                    &game_id,
                    activation,
                    "Initial recovery root or runtime coverage changed; retry activation".into(),
                );
                return;
            }
            let watcher = app.state::<WatcherState>();
            if result.status.applied()
                && (crate::shared::sync::lock(&watcher.watcher).is_none()
                    || watcher
                        .current_session_for_coverage(&game.mod_path, Some(&runtime_config_path))
                        .is_none())
            {
                emit_activation_failure(
                    &app,
                    &game_id,
                    activation,
                    "Watcher coverage ended during recovery; retry activation".into(),
                );
                return;
            }
            if watcher
                .current_session_for_coverage(&game.mod_path, Some(&runtime_config_path))
                .is_some_and(|session| watcher_identity_cache_failure(&session).is_some())
            {
                emit_activation_failure(
                    &app,
                    &game_id,
                    activation,
                    WATCHER_IDENTITY_COVERAGE_ERROR.into(),
                );
                return;
            }
            if result.status.applied() {
                let coordinator = app.state::<crate::modules::mutation::api::MutationCoordinator>();
                if let Some(reused) = prepare_current_activation_reuse(
                    state.inner(),
                    watcher.inner(),
                    coordinator.inner(),
                    &game_id,
                    &game.mod_path,
                    &runtime_config_path,
                    activation.activation_generation,
                ) {
                    if reused.activation().recovery_generation != activation.recovery_generation {
                        return;
                    }
                    if matches!(
                        publish_activation_outcome(
                            &app,
                            &game_id,
                            reused.session(),
                            watcher.suppressor.as_ref(),
                            &game.mod_path,
                            activation,
                            reused.result()
                        ),
                        RecoveredActivationPublication::Published
                    ) {
                        return;
                    }
                }
            } else {
                let phase = if result.status == crate::modules::reconciliation::api::disk_reconcile::types::DiskReconcileStatus::SourceUnavailable {
                    crate::modules::reconciliation::api::disk_reconcile::types::GameActivationPhase::SourceUnavailable
                } else {
                    crate::modules::reconciliation::api::disk_reconcile::types::GameActivationPhase::Failed
                };
                emit_activation_failure_with_phase(
                    &app,
                    &game_id,
                    activation,
                    phase,
                    result
                        .error_message
                        .clone()
                        .unwrap_or_else(|| "Initial disk recovery requires attention".into()),
                );
                return;
            }
            tokio::time::sleep(authority_catch_up_backoff(retry_round)).await;
            retry_round = retry_round.saturating_add(1);
        }
    });
}
