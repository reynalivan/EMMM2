//! Tauri-free watcher acceptance and catch-up decisions.
//!
//! Acceptance and authority helpers below are stateful lease/identity checks.
//! Scope, reason, publication eligibility and backoff decisions use only supplied facts.

use crate::modules::workspace::application::scanner::watcher::{
    WatcherSession, WatcherState, WatcherSuppressor,
};
use crate::shared::sync::lock;

pub(crate) const WATCHER_IDENTITY_COVERAGE_ERROR: &str =
    "Watcher directory identity coverage is incomplete; retry activation to rebuild the watcher";

pub(crate) fn watcher_identity_cache_failure(session: &WatcherSession) -> Option<&'static str> {
    (!session
        .identity_cache_validity()
        .load(std::sync::atomic::Ordering::Acquire))
    .then_some(WATCHER_IDENTITY_COVERAGE_ERROR)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatcherInstallScope {
    InactiveOnly,
    StartupSelected,
}

pub(crate) fn install_scope_matches_selection(
    scope: WatcherInstallScope,
    selected_game_id: Option<&str>,
    game_id: &str,
) -> bool {
    match scope {
        WatcherInstallScope::InactiveOnly => selected_game_id != Some(game_id),
        WatcherInstallScope::StartupSelected => selected_game_id == Some(game_id),
    }
}

pub(crate) fn initial_reconcile_reason(
    activation: bool,
) -> crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason {
    if activation {
        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::GameSwitched
    } else {
        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::ManualRepair
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WatcherActivation {
    pub activation_generation: u64,
    pub recovery_generation: u64,
}

pub(crate) fn retire_superseded_activation(
    state: &crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState,
    game_id: &str,
    activation: &mut Option<WatcherActivation>,
) -> bool {
    if activation.is_some_and(|duty| {
        !state.activation_is_current(Some(game_id), duty.activation_generation)
            || !state.initial_recovery_generation_is_current(game_id, duty.recovery_generation)
    }) {
        *activation = None;
        true
    } else {
        false
    }
}

pub(crate) fn no_pending_disk_commit(
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    game_id: &str,
) -> bool {
    match coordinator.has_pending_disk_commit_for_game(game_id) {
        Ok(pending) => !pending,
        Err(error) => {
            log::warn!("Could not check pending disk commit for '{game_id}': {error}");
            false
        }
    }
}

pub(crate) fn activation_result_is_accepted(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    suppressor: &WatcherSuppressor,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    game_id: &str,
    mods_root: &std::path::Path,
    session: &WatcherSession,
    result: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
) -> bool {
    no_pending_disk_commit(coordinator, game_id)
        && suppressor.pending_repair(session).is_none()
        && watcher_result_is_current_authority(
            state,
            game_id,
            mods_root,
            session.generation(),
            result,
        )
}

pub(crate) fn activation_publication_allowed(
    result: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    validate_ready: impl FnOnce() -> bool,
) -> bool {
    !result.status.applied() || validate_ready()
}

pub(crate) fn initial_activation_waits_for_barrier(
    activation: bool,
    authority_pending: bool,
    result: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
) -> bool {
    activation && authority_pending && result.status.applied()
}

pub(crate) fn clean_cached_activation_result(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    suppressor: &WatcherSuppressor,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    game_id: &str,
    mods_root: &std::path::Path,
    session: &WatcherSession,
) -> Option<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult>
{
    let _game_guard = state.try_game_guard(game_id)?;
    if !no_pending_disk_commit(coordinator, game_id) || suppressor.pending_repair(session).is_some()
    {
        return None;
    }
    let result = state.clean_authoritative_result(game_id, mods_root, session.generation())?;
    activation_result_is_accepted(
        state,
        suppressor,
        coordinator,
        game_id,
        mods_root,
        session,
        &result,
    )
    .then_some(result)
}

pub(crate) fn try_clean_activation_recovery_result(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    suppressor: &WatcherSuppressor,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    game_id: &str,
    mods_root: &std::path::Path,
    session: &WatcherSession,
) -> Option<(
    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    crate::platform::fs::operation_lock::OpGuard,
)> {
    let operation_guard = coordinator.inner_lock().try_acquire_for_reconcile()?;
    let result = clean_cached_activation_result(
        state,
        suppressor,
        coordinator,
        game_id,
        mods_root,
        session,
    )?;
    Some((result, operation_guard))
}

pub(crate) struct CurrentActivationReuse {
    activation: WatcherActivation,
    session: WatcherSession,
    result: crate::modules::reconciliation::api::disk_reconcile::types::DiskReconcileResult,
    _operation_guard: crate::platform::fs::operation_lock::OpGuard,
}

impl CurrentActivationReuse {
    pub(crate) fn activation(&self) -> WatcherActivation {
        self.activation
    }
    pub(crate) fn session(&self) -> &WatcherSession {
        &self.session
    }
    pub(crate) fn result(
        &self,
    ) -> &crate::modules::reconciliation::api::disk_reconcile::types::DiskReconcileResult {
        &self.result
    }
}

pub(crate) fn prepare_current_activation_reuse(
    state: &crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState,
    watcher: &WatcherState,
    coordinator: &crate::modules::mutation::api::MutationCoordinator,
    game_id: &str,
    mods_root: &std::path::Path,
    runtime_config_path: &std::path::Path,
    activation_generation: u64,
) -> Option<CurrentActivationReuse> {
    use crate::modules::reconciliation::api::disk_reconcile::orchestrator::InitialRecoveryReadiness;

    if !state.activation_is_current(Some(game_id), activation_generation) {
        return None;
    }
    let InitialRecoveryReadiness::Ready { generation } = state.initial_recovery_readiness(game_id)
    else {
        return None;
    };
    let installed = lock(&watcher.watcher);
    installed.as_ref()?;
    let session = watcher.current_session_for_coverage(mods_root, Some(runtime_config_path))?;
    if watcher_identity_cache_failure(&session).is_some() {
        return None;
    }
    if !session.matches_current_binding(mods_root, Some(runtime_config_path)) {
        return None;
    }
    let (result, operation_guard) = try_clean_activation_recovery_result(
        state,
        watcher.suppressor.as_ref(),
        coordinator,
        game_id,
        mods_root,
        &session,
    )?;
    Some(CurrentActivationReuse {
        activation: WatcherActivation {
            activation_generation,
            recovery_generation: generation,
        },
        session,
        result,
        _operation_guard: operation_guard,
    })
}

pub(crate) fn pending_activation_target<'a>(
    state: &crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState,
    settings: &'a crate::modules::settings::api::config::AppSettings,
    game_id: &str,
    activation: WatcherActivation,
    terminal_failure: impl FnOnce(),
) -> Option<&'a crate::modules::settings::api::config::GameConfig> {
    if !state.activation_is_current(Some(game_id), activation.activation_generation) {
        return None;
    }
    let game = settings.active_game().filter(|game| game.id == game_id);
    if game.is_none() {
        terminal_failure();
    }
    game
}

pub(crate) fn prepare_activation_recovery(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    suppressor: &WatcherSuppressor,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    game_id: &str,
    mods_root: &std::path::Path,
    session: &WatcherSession,
    activation_generation: u64,
) -> (
    WatcherActivation,
    Option<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult>,
) {
    let cached =
        clean_cached_activation_result(state, suppressor, coordinator, game_id, mods_root, session);
    if let (Some(result), crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Ready { generation }) =
        (cached, state.initial_recovery_readiness(game_id))
    {
        return (WatcherActivation { activation_generation, recovery_generation: generation }, Some(result));
    }
    if !matches!(
        state.initial_recovery_readiness(game_id),
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Unstarted { .. }
    ) {
        state.reset_initial_recovery(game_id);
    }
    let recovery_generation = state.mark_initial_recovery_pending(game_id);
    (
        WatcherActivation {
            activation_generation,
            recovery_generation,
        },
        None,
    )
}

pub(crate) fn prepare_inactive_authority(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    game_id: &str,
    mods_root: &std::path::Path,
    covered_session: Option<u64>,
    session: &WatcherSession,
) -> bool {
    let watcher_session = session.generation();
    let transferred = covered_session.is_some_and(|covered_session| {
        state.handoff_authority_session_with_coverage(game_id, mods_root, covered_session, session)
    });
    if !transferred
        && state
            .authority_event_generation(game_id, watcher_session)
            .is_none()
    {
        state.begin_authority_session_with_coverage(game_id, mods_root, session);
    }
    transferred
}

pub(crate) fn initial_watcher_recovery_plan(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    suppressor: &WatcherSuppressor,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    game_id: &str,
    mods_root: &std::path::Path,
    session: &WatcherSession,
) -> (
    Option<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult>,
    Vec<String>,
    bool,
    u64,
) {
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp;

    let watcher_session = session.generation();
    match state.authority_catch_up(game_id, mods_root, watcher_session) {
        AuthorityCatchUp::Clean {
            observed_generation,
            ..
        } if state.authority_event_generation(game_id, watcher_session)
            == Some(observed_generation) =>
        {
            let cached = clean_cached_activation_result(
                state,
                suppressor,
                coordinator,
                game_id,
                mods_root,
                session,
            );
            let force_full = cached.is_none();
            (cached, Vec::new(), force_full, observed_generation)
        }
        AuthorityCatchUp::Scoped {
            changed_paths,
            observed_generation,
        } => (None, changed_paths, false, observed_generation),
        AuthorityCatchUp::Full {
            observed_generation,
        }
        | AuthorityCatchUp::Clean {
            observed_generation,
            ..
        } => (None, Vec::new(), true, observed_generation),
    }
}

#[derive(Debug)]
pub(crate) struct WatcherCatchUpPlan {
    pub(crate) changed_paths: Vec<String>,
    pub(crate) force_full: bool,
    pub(crate) observed_generation: u64,
}

pub(crate) struct WatcherAuthorityContext<'a> {
    pub(crate) state:
        &'a crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState,
    pub(crate) suppressor: &'a WatcherSuppressor,
    pub(crate) game_id: &'a str,
    pub(crate) mods_root: &'a std::path::Path,
    pub(crate) session: &'a WatcherSession,
}

impl WatcherAuthorityContext<'_> {
    pub(crate) fn retire_publication(
        &self,
        activation: &mut Option<WatcherActivation>,
        activation_recovery_pending: &mut bool,
        pending_catch_up: &mut Option<WatcherCatchUpPlan>,
        retry_after: &mut Option<tokio::time::Instant>,
    ) -> bool {
        if !retire_superseded_activation(self.state, self.game_id, activation) {
            return false;
        }
        *activation_recovery_pending = false;
        *pending_catch_up = pending_activation_catch_up_plan(
            self.state,
            self.suppressor,
            self.game_id,
            self.mods_root,
            self.session,
        )
        .or_else(|| pending_catch_up.take());
        *retry_after = None;
        true
    }
}

/// Reconcile work still required after a watcher result could not become the
/// authority. The plan reads the current authority generation, rather than
/// replaying stale batch paths, so a slow rename cannot strand disk changes.
pub(crate) fn pending_watcher_catch_up_plan(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    game_id: &str,
    mods_root: &std::path::Path,
    watcher_session: u64,
) -> Option<WatcherCatchUpPlan> {
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp;

    match state.authority_catch_up(game_id, mods_root, watcher_session) {
        AuthorityCatchUp::Clean {
            reconcile_revision,
            observed_generation,
        } => state
            .authoritative_result(game_id)
            .filter(|result| {
                result.status.applied() && result.reconcile_revision == reconcile_revision
            })
            .is_none()
            .then_some(WatcherCatchUpPlan {
                changed_paths: Vec::new(),
                force_full: true,
                observed_generation,
            }),
        AuthorityCatchUp::Scoped {
            changed_paths,
            observed_generation,
        } => Some(WatcherCatchUpPlan {
            changed_paths,
            force_full: false,
            observed_generation,
        }),
        AuthorityCatchUp::Full {
            observed_generation,
        } => Some(WatcherCatchUpPlan {
            changed_paths: Vec::new(),
            force_full: true,
            observed_generation,
        }),
    }
}

pub(crate) fn pending_activation_catch_up_plan(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    suppressor: &WatcherSuppressor,
    game_id: &str,
    mods_root: &std::path::Path,
    session: &WatcherSession,
) -> Option<WatcherCatchUpPlan> {
    if suppressor.pending_repair(session).is_some() {
        return Some(WatcherCatchUpPlan {
            changed_paths: Vec::new(),
            force_full: true,
            observed_generation: state
                .authority_event_generation(game_id, session.generation())
                .unwrap_or(0),
        });
    }
    pending_watcher_catch_up_plan(state, game_id, mods_root, session.generation())
}

fn watcher_result_is_current_authority(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    game_id: &str,
    mods_root: &std::path::Path,
    watcher_session: u64,
    result: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
) -> bool {
    result.status.applied()
        && matches!(
            state.authority_catch_up(game_id, mods_root, watcher_session),
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp::Clean {
                reconcile_revision,
                ..
            } if reconcile_revision == result.reconcile_revision
        )
}

pub(crate) fn authority_catch_up_backoff(retry_round: u32) -> std::time::Duration {
    let seconds = (1_u64 << retry_round.min(5)).min(30);
    std::time::Duration::from_secs(seconds)
}

pub(crate) struct PrewarmAcceptance<'a> {
    pub(crate) game_id: &'a str,
    pub(crate) mods_root: &'a std::path::Path,
    pub(crate) runtime_config_path: Option<&'a std::path::Path>,
    pub(crate) watcher_session: u64,
    pub(crate) observed_generation: u64,
    pub(crate) result:
        &'a crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    pub(crate) changed_paths: &'a [String],
}

pub(crate) async fn accept_prewarm_result(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    watcher: &WatcherState,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    selected_game_id: impl FnOnce() -> Option<String>,
    acceptance: PrewarmAcceptance<'_>,
) -> bool {
    let PrewarmAcceptance {
        game_id,
        mods_root,
        runtime_config_path,
        watcher_session,
        observed_generation,
        result,
        changed_paths,
    } = acceptance;
    let _activation_guard = state.activation_guard().await;
    let Some(_game_guard) = state.try_game_guard(game_id) else {
        state.reject_untrusted_reconcile(game_id, result.reconcile_revision);
        return false;
    };
    let active_session = (selected_game_id().as_deref() == Some(game_id))
        .then(|| watcher.current_session_for_coverage(mods_root, runtime_config_path))
        .flatten()
        .filter(|session| session.generation() == watcher_session);
    let inactive_session = watcher
        .inactive_session_for_coverage(game_id, mods_root, runtime_config_path)
        .filter(|session| session.generation() == watcher_session);
    let trusted = active_session.or(inactive_session).is_some_and(|session| {
        no_pending_disk_commit(coordinator, game_id)
            && watcher.suppressor.pending_repair(&session).is_none()
            && state.mark_authority_reconciled(
                game_id,
                mods_root,
                watcher_session,
                observed_generation,
                result,
                changed_paths,
            )
    });
    if !trusted {
        state.reject_untrusted_reconcile(game_id, result.reconcile_revision);
    }
    trusted
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn removing_configured_game_during_join_publishes_terminal_failure_for_current_intent() {
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState;
        use crate::modules::settings::api::config::ConfigService;
        let db = crate::test_utils::init_test_db().await;
        let temp = tempfile::tempdir().expect("root");
        crate::test_utils::insert_test_game(
            &db.pool,
            &crate::test_utils::TestGameFixture {
                id: "game",
                name: "Game",
                game_type: crate::modules::games::api::testing::domain::models::GameType::Gimi,
                path: temp.path().to_string_lossy().as_ref(),
                mods_path: Some(temp.path().to_string_lossy().as_ref()),
            },
        )
        .await
        .expect("game");
        let config = ConfigService::new_for_test_async(db.pool).await;
        config
            .set_active_game(Some("game".into()))
            .expect("selection");
        let state = DiskReconcileState::new();
        let recovery_generation = state.mark_initial_recovery_pending_for_root(
            "game",
            temp.path(),
            &temp.path().join("d3dx.ini"),
        );
        let activation = WatcherActivation {
            activation_generation: state.begin_activation(Some("game".into())),
            recovery_generation,
        };
        let mut terminal_failures = 0;
        assert!(pending_activation_target(
            &state,
            &config.get_settings(),
            "game",
            activation,
            || terminal_failures += 1
        )
        .is_some());
        let mut removed = config.get_settings();
        removed.games.clear();
        config
            .save_settings(removed)
            .expect("remove active configured game");
        assert!(config.get_settings().active_game_id.is_none());
        assert!(state.activation_is_current(Some("game"), activation.activation_generation));
        assert!(pending_activation_target(
            &state,
            &config.get_settings(),
            "game",
            activation,
            || terminal_failures += 1
        )
        .is_none());
        assert_eq!(
            terminal_failures, 1,
            "the current joined intent receives one terminal failure instead of remaining Syncing"
        );
        state.begin_activation(None);
        assert!(pending_activation_target(
            &state,
            &config.get_settings(),
            "game",
            activation,
            || terminal_failures += 1
        )
        .is_none());
        assert_eq!(
            terminal_failures, 1,
            "a genuinely superseded intent stays silent"
        );
    }

    #[test]
    fn activation_reuses_a_fresh_authoritative_reconcile() {
        use crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason;

        assert_eq!(
            initial_reconcile_reason(true),
            DiskReconcileReason::GameSwitched
        );
        assert_eq!(
            initial_reconcile_reason(false),
            DiskReconcileReason::ManualRepair
        );
    }

    #[test]
    fn startup_can_install_selected_watcher_but_prewarm_cannot() {
        assert!(install_scope_matches_selection(
            WatcherInstallScope::StartupSelected,
            Some("game-1"),
            "game-1",
        ));
        assert!(!install_scope_matches_selection(
            WatcherInstallScope::InactiveOnly,
            Some("game-1"),
            "game-1",
        ));
        assert!(!install_scope_matches_selection(
            WatcherInstallScope::StartupSelected,
            Some("game-2"),
            "game-1",
        ));
        assert!(install_scope_matches_selection(
            WatcherInstallScope::InactiveOnly,
            Some("game-2"),
            "game-1",
        ));
    }

    #[test]
    fn inactive_handoff_keeps_an_event_seen_by_the_new_watcher() {
        use crate::modules::reconciliation::application::disk_reconcile::orchestrator::{
            AuthorityCatchUp, DiskReconcileState,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("mods root");
        let state = DiskReconcileState::new();
        state.begin_authority_session("game", &root, 1);
        state.observe_authority_event("game", 2, &root, &[root.join("Changed")], false);
        let observed = state.authority_event_generation("game", 2);

        assert!(!prepare_inactive_authority(
            &state,
            "game",
            &root,
            Some(1),
            &WatcherSession::new_with_runtime_config(2, &root, None)
        ));
        assert_eq!(state.authority_event_generation("game", 2), observed);
        assert!(matches!(
            state.authority_catch_up("game", &root, 2),
            AuthorityCatchUp::Full { .. }
        ));
    }

    #[test]
    fn rejected_watcher_projection_plans_current_authority_catch_up() {
        use crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState;

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("mods root");
        let state = DiskReconcileState::new();
        state.begin_authority_session("game", &root, 7);
        state.observe_authority_event("game", 7, &root, &[root.join("Renamed")], false);

        let plan = pending_watcher_catch_up_plan(&state, "game", &root, 7)
            .expect("dirty authority must be retried without a new notify event");

        assert!(plan.force_full);
        assert!(plan.changed_paths.is_empty());
        assert_eq!(
            Some(plan.observed_generation),
            state.authority_event_generation("game", 7)
        );
    }

    #[test]
    fn initial_dirty_authority_seeds_catch_up_without_notify_event() {
        use crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState;

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("mods root");
        let state = DiskReconcileState::new();
        state.begin_authority_session("game", &root, 7);

        let plan = pending_watcher_catch_up_plan(&state, "game", &root, 7)
            .expect("initial dirty authority must schedule a disk catch-up");

        assert!(plan.force_full);
        assert!(plan.changed_paths.is_empty());
    }

    #[test]
    fn clean_authority_retries_after_source_becomes_unavailable() {
        use crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState;
        use crate::modules::reconciliation::application::disk_reconcile::types::{
            DiskReconcileReason, DiskReconcileResult, DiskReconcileScanScope, DiskReconcileStatus,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("mods root");
        let state = DiskReconcileState::new();
        state.begin_authority_session("game", &root, 7);
        let mut baseline = DiskReconcileResult {
            game_id: "game".to_string(),
            reconcile_revision: 0,
            reason: DiskReconcileReason::StartupBoot,
            status: DiskReconcileStatus::Applied,
            scan_scope: DiskReconcileScanScope::Full,
            folder_conflicts: Vec::new(),
            rename_confirmations: Vec::new(),
            error_message: None,
            changed_roots: Vec::new(),
            objects_changed: false,
            folders_changed: false,
            collections_changed: false,
            runtime_file_changed: false,
            thumbnail_roots: Vec::new(),
            cleared_selection_paths: Vec::new(),
            path_updates: Vec::new(),
            collection_reference_impact: Default::default(),
            change_summary: Default::default(),
            pending_runtime_effects: Default::default(),
            warnings: Vec::new(),
        };
        state.record_result("game", &mut baseline);
        assert!(state.mark_authority_reconciled("game", &root, 7, 0, &baseline, &[],));

        let mut unavailable = baseline.clone();
        unavailable.status = DiskReconcileStatus::SourceUnavailable;
        state.record_result("game", &mut unavailable);

        let plan = pending_watcher_catch_up_plan(&state, "game", &root, 7)
            .expect("source unavailable must retry despite a clean watcher token");
        assert!(plan.force_full);
        assert!(plan.changed_paths.is_empty());
    }

    #[test]
    fn authority_catch_up_backoff_caps_scan_pressure() {
        assert_eq!(authority_catch_up_backoff(0), Duration::from_secs(1));
        assert_eq!(authority_catch_up_backoff(1), Duration::from_secs(2));
        assert_eq!(authority_catch_up_backoff(2), Duration::from_secs(4));
        assert_eq!(authority_catch_up_backoff(5), Duration::from_secs(30));
        assert_eq!(
            authority_catch_up_backoff(u32::MAX),
            Duration::from_secs(30)
        );
    }
}
