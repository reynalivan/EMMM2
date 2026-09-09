//! Per-game reconcile locks, activation cache, and runtime-effect state.

use crate::shared::errors::AppError;
use crate::shared::sync::lock;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify, OwnedMutexGuard};

use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcileResult, PendingRuntimeEffects,
};

#[derive(Debug, Default, Clone)]
struct GameSyncState {
    last_result: Option<DiskReconcileResult>,
    completed_at: Option<std::time::Instant>,
    pending_runtime_effects: PendingRuntimeEffects,
    runtime_effects_generation: u64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StagedRuntimeEffects {
    pub(crate) pending: PendingRuntimeEffects,
    generation: u64,
}

impl PendingRuntimeEffects {
    fn merge(&mut self, other: Self) {
        self.collections_dirty |= other.collections_dirty;
        self.overlay_refresh |= other.overlay_refresh;
    }
}

#[derive(Default)]
pub struct DiskReconcileState {
    activation_lock: Mutex<()>,
    locks: std::sync::Mutex<HashMap<String, Arc<Mutex<()>>>>,
    games: std::sync::Mutex<HashMap<String, GameSyncState>>,
    initial_recovery: std::sync::Mutex<HashMap<String, Arc<InitialRecoveryGate>>>,
}

/// Owned proof that a disk mutation holds both serialization locks in the
/// only supported order: per-game first, then the global operation lock.
pub struct DiskMutationLease {
    _game_guard: OwnedMutexGuard<()>,
    operation_guard: DiskMutationOperationGuard,
}

enum DiskMutationOperationGuard {
    LockOnly {
        _guard: crate::platform::fs::operation_lock::OpGuard,
    },
    Durable(crate::modules::mutation::coordinator::MutationGuard),
}

impl DiskMutationLease {
    pub(crate) fn from_durable_guard(
        game_guard: OwnedMutexGuard<()>,
        operation_guard: crate::modules::mutation::coordinator::MutationGuard,
    ) -> Self {
        Self {
            _game_guard: game_guard,
            operation_guard: DiskMutationOperationGuard::Durable(operation_guard),
        }
    }

    pub(crate) fn operation_guard(&self) -> &crate::platform::fs::operation_lock::OpGuard {
        match &self.operation_guard {
            DiskMutationOperationGuard::LockOnly { _guard } => _guard,
            DiskMutationOperationGuard::Durable(guard) => guard.op_guard(),
        }
    }

    pub(crate) fn mark_step_applied(&self, sequence: u32) -> Result<(), AppError> {
        match &self.operation_guard {
            DiskMutationOperationGuard::Durable(guard) => guard.mark_step_applied(sequence),
            DiskMutationOperationGuard::LockOnly { .. } => Err(AppError::Internal(
                "Mutation lease has no durable operation plan".to_string(),
            )),
        }
    }

    pub(crate) fn mark_step_skipped(&self, sequence: u32) -> Result<(), AppError> {
        match &self.operation_guard {
            DiskMutationOperationGuard::Durable(guard) => guard.mark_step_skipped(sequence),
            DiskMutationOperationGuard::LockOnly { .. } => Err(AppError::Internal(
                "Mutation lease has no durable operation plan".to_string(),
            )),
        }
    }

    pub(crate) fn mark_db_committed(&self) -> Result<(), AppError> {
        match &self.operation_guard {
            DiskMutationOperationGuard::Durable(guard) => guard.mark_db_committed(),
            DiskMutationOperationGuard::LockOnly { .. } => Err(AppError::Internal(
                "Mutation lease has no durable operation plan".to_string(),
            )),
        }
    }

    pub(crate) fn begin_rollback(&self) -> Result<(), AppError> {
        match &self.operation_guard {
            DiskMutationOperationGuard::Durable(guard) => guard.begin_rollback(),
            DiskMutationOperationGuard::LockOnly { .. } => Err(AppError::Internal(
                "Mutation lease has no durable operation plan".to_string(),
            )),
        }
    }

    pub(crate) fn mark_step_rolled_back(&self, sequence: u32) -> Result<(), AppError> {
        match &self.operation_guard {
            DiskMutationOperationGuard::Durable(guard) => guard.mark_step_rolled_back(sequence),
            DiskMutationOperationGuard::LockOnly { .. } => Err(AppError::Internal(
                "Mutation lease has no durable operation plan".to_string(),
            )),
        }
    }

    pub(crate) fn finish_rollback(self) -> Result<(), AppError> {
        match self.operation_guard {
            DiskMutationOperationGuard::Durable(guard) => guard.finish_rollback(),
            DiskMutationOperationGuard::LockOnly { .. } => Err(AppError::Internal(
                "Mutation lease has no durable operation plan".to_string(),
            )),
        }
    }

    pub(crate) fn commit(self) -> Result<(), AppError> {
        match self.operation_guard {
            DiskMutationOperationGuard::Durable(guard) => guard.commit(),
            DiskMutationOperationGuard::LockOnly { .. } => Err(AppError::Internal(
                "Mutation lease has no durable operation plan".to_string(),
            )),
        }
    }

    pub(crate) fn fail(self, error: impl Into<String>) -> Result<(), AppError> {
        match self.operation_guard {
            DiskMutationOperationGuard::Durable(guard) => guard.fail(error),
            DiskMutationOperationGuard::LockOnly { .. } => Err(AppError::Internal(
                "Mutation lease has no durable operation plan".to_string(),
            )),
        }
    }
}

struct InitialRecoveryGate {
    state: std::sync::Mutex<InitialRecoveryState>,
    notify: Notify,
}

#[derive(Debug, Clone)]
enum InitialRecoveryStatus {
    Unstarted,
    Pending,
    Finished(InitialRecoveryOutcome),
}

#[derive(Debug, Clone)]
struct InitialRecoveryState {
    generation: u64,
    status: InitialRecoveryStatus,
}

impl Default for InitialRecoveryGate {
    fn default() -> Self {
        Self {
            state: std::sync::Mutex::new(InitialRecoveryState {
                generation: 0,
                status: InitialRecoveryStatus::Unstarted,
            }),
            notify: Notify::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InitialRecoveryOutcome {
    Completed(Box<DiskReconcileResult>),
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum InitialRecoveryClaim {
    Run { generation: u64 },
    Finished(InitialRecoveryOutcome),
}

#[derive(Debug, Clone)]
pub enum InitialRecoveryStart {
    Run { generation: u64 },
    Syncing { generation: u64 },
    Finished(InitialRecoveryOutcome),
}

/// A non-blocking view of a game's initial disk recovery. Readers use this to
/// render a last-valid workspace while the recovery owner continues to make
/// the disk-authoritative projection. It deliberately carries no result data:
/// callers must wait for the terminal reconcile result before mutating disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitialRecoveryReadiness {
    Unstarted { generation: u64 },
    Syncing { generation: u64 },
    Ready { generation: u64 },
    Failed { generation: u64 },
}

impl DiskReconcileState {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn acquire_nested_mutation_lease(
        &self,
        game_id: &str,
        coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    ) -> Result<DiskMutationLease, AppError> {
        let game_guard = self.lock_for_game(game_id).lock_owned().await;
        let operation_guard = coordinator.acquire_nested_operation_lock().await?;
        Ok(DiskMutationLease {
            _game_guard: game_guard,
            operation_guard: DiskMutationOperationGuard::LockOnly {
                _guard: operation_guard,
            },
        })
    }

    pub async fn acquire_mutation_lease(
        &self,
        game_id: &str,
        operation_lock: &crate::platform::fs::operation_lock::OperationLock,
    ) -> Result<DiskMutationLease, AppError> {
        let game_guard = self.lock_for_game(game_id).lock_owned().await;
        let operation_guard =
            crate::modules::mutation::coordinator::MutationCoordinator::acquire_nested_operation_on(
                operation_lock,
            )
            .await?;
        Ok(DiskMutationLease {
            _game_guard: game_guard,
            operation_guard: DiskMutationOperationGuard::LockOnly {
                _guard: operation_guard,
            },
        })
    }

    pub(super) fn lock_for_game(&self, game_id: &str) -> Arc<Mutex<()>> {
        let mut locks = lock(&self.locks);
        locks
            .entry(game_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub(crate) fn stage_runtime_effects(
        &self,
        game_id: &str,
        requested: PendingRuntimeEffects,
    ) -> PendingRuntimeEffects {
        self.stage_runtime_effects_for_settlement(game_id, requested)
            .pending
    }

    pub(crate) fn stage_runtime_effects_for_settlement(
        &self,
        game_id: &str,
        requested: PendingRuntimeEffects,
    ) -> StagedRuntimeEffects {
        let mut games = lock(&self.games);
        let state = games.entry(game_id.to_string()).or_default();
        state.pending_runtime_effects.merge(requested);
        state.runtime_effects_generation += 1;
        StagedRuntimeEffects {
            pending: state.pending_runtime_effects,
            generation: state.runtime_effects_generation,
        }
    }

    pub(crate) fn acknowledge_runtime_effects(
        &self,
        game_id: &str,
        staged: StagedRuntimeEffects,
    ) -> PendingRuntimeEffects {
        let mut games = lock(&self.games);
        let Some(state) = games.get_mut(game_id) else {
            return PendingRuntimeEffects::default();
        };
        if state.runtime_effects_generation == staged.generation {
            state.pending_runtime_effects = PendingRuntimeEffects::default();
        }
        state.pending_runtime_effects
    }

    /// The per-game reconcile mutex, for a flow that must run
    /// `reconcile_disk_projection` inline without interleaving with a public
    /// reconcile for the same game.
    pub fn game_lock(&self, game_id: &str) -> Arc<Mutex<()>> {
        self.lock_for_game(game_id)
    }

    /// Serializes active-game selection through recovery and any rollback.
    /// Without this lease, a late failed activation could restore an older
    /// selection over a newer successful activation.
    pub async fn activation_guard(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.activation_lock.lock().await
    }

    fn initial_recovery_gate(&self, game_id: &str) -> Arc<InitialRecoveryGate> {
        let mut gates = lock(&self.initial_recovery);
        gates
            .entry(game_id.to_string())
            .or_insert_with(|| Arc::new(InitialRecoveryGate::default()))
            .clone()
    }

    pub fn initial_recovery_readiness(&self, game_id: &str) -> InitialRecoveryReadiness {
        let gate = self.initial_recovery_gate(game_id);
        let state = lock(&gate.state);
        match &state.status {
            InitialRecoveryStatus::Unstarted => InitialRecoveryReadiness::Unstarted {
                generation: state.generation,
            },
            InitialRecoveryStatus::Pending => InitialRecoveryReadiness::Syncing {
                generation: state.generation,
            },
            InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Completed(_)) => {
                InitialRecoveryReadiness::Ready {
                    generation: state.generation,
                }
            }
            InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Failed(_)) => {
                InitialRecoveryReadiness::Failed {
                    generation: state.generation,
                }
            }
        }
    }

    /// Starts recovery exactly once without making a read caller wait for it.
    /// The caller owning `Run` must finish the same generation.
    pub fn start_initial_recovery(&self, game_id: &str) -> InitialRecoveryStart {
        let gate = self.initial_recovery_gate(game_id);
        let mut state = lock(&gate.state);
        match &state.status {
            InitialRecoveryStatus::Unstarted => {
                state.generation += 1;
                state.status = InitialRecoveryStatus::Pending;
                InitialRecoveryStart::Run {
                    generation: state.generation,
                }
            }
            InitialRecoveryStatus::Pending => InitialRecoveryStart::Syncing {
                generation: state.generation,
            },
            InitialRecoveryStatus::Finished(outcome) => {
                InitialRecoveryStart::Finished(outcome.clone())
            }
        }
    }

    pub fn mark_initial_recovery_pending(&self, game_id: &str) -> u64 {
        let gate = self.initial_recovery_gate(game_id);
        let generation = {
            let mut state = lock(&gate.state);
            state.generation += 1;
            state.status = InitialRecoveryStatus::Pending;
            state.generation
        };
        gate.notify.notify_waiters();
        generation
    }

    pub fn reset_initial_recovery(&self, game_id: &str) {
        let gate = self.initial_recovery_gate(game_id);
        let gate = {
            let mut state = lock(&gate.state);
            state.generation += 1;
            state.status = InitialRecoveryStatus::Unstarted;
            Arc::clone(&gate)
        };
        gate.notify.notify_waiters();
        if let Some(state) = lock(&self.games).get_mut(game_id) {
            state.completed_at = None;
        }
    }

    pub fn finish_initial_recovery(
        &self,
        game_id: &str,
        generation: u64,
        outcome: InitialRecoveryOutcome,
    ) {
        let gate = lock(&self.initial_recovery).get(game_id).cloned();
        if let Some(gate) = gate {
            let finished = {
                let mut state = lock(&gate.state);
                if state.generation != generation
                    || !matches!(state.status, InitialRecoveryStatus::Pending)
                {
                    false
                } else {
                    state.status = InitialRecoveryStatus::Finished(outcome);
                    true
                }
            };
            if finished {
                gate.notify.notify_waiters();
            }
        }
    }

    pub async fn claim_initial_recovery(&self, game_id: &str) -> InitialRecoveryClaim {
        loop {
            let gate = self.initial_recovery_gate(game_id);
            let notified = gate.notify.notified();
            match self.start_initial_recovery(game_id) {
                InitialRecoveryStart::Run { generation } => {
                    return InitialRecoveryClaim::Run { generation };
                }
                InitialRecoveryStart::Finished(outcome) => {
                    return InitialRecoveryClaim::Finished(outcome);
                }
                InitialRecoveryStart::Syncing { .. } => {}
            }
            notified.await;
        }
    }

    pub(super) fn record_result(&self, game_id: &str, result: &DiskReconcileResult) {
        {
            let mut games = lock(&self.games);
            let state = games.entry(game_id.to_string()).or_default();
            state.last_result = Some(result.clone());
            state.completed_at = Some(std::time::Instant::now());
        }

        // Once activation recovery has reached a terminal state, every later
        // successful reconcile becomes its newest disk-authoritative result.
        // Pending startup/workspace claims retain generation ownership.
        if let Some(gate) = lock(&self.initial_recovery).get(game_id).cloned() {
            let updated = {
                let mut recovery = lock(&gate.state);
                if matches!(recovery.status, InitialRecoveryStatus::Finished(_)) {
                    recovery.status = InitialRecoveryStatus::Finished(
                        InitialRecoveryOutcome::Completed(Box::new(result.clone())),
                    );
                    true
                } else {
                    false
                }
            };
            if updated {
                gate.notify.notify_waiters();
            }
        }
    }

    pub(super) fn recent_applied_result(
        &self,
        game_id: &str,
        max_age: std::time::Duration,
    ) -> Option<DiskReconcileResult> {
        let games = lock(&self.games);
        let state = games.get(game_id)?;
        if state.completed_at?.elapsed() > max_age {
            return None;
        }
        state
            .last_result
            .as_ref()
            .filter(|result| {
                result.status
                    == crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileStatus::Applied
            })
            .cloned()
    }
}

#[cfg(test)]
mod initial_recovery_tests {
    use super::{
        DiskReconcileState, InitialRecoveryClaim, InitialRecoveryOutcome, InitialRecoveryReadiness,
    };
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn workspace_gate_waits_until_initial_recovery_is_terminal() {
        let state = Arc::new(DiskReconcileState::new());
        let generation = state.mark_initial_recovery_pending("game");
        let waiting_state = Arc::clone(&state);
        let waiter =
            tokio::spawn(async move { waiting_state.claim_initial_recovery("game").await });
        let mut waiter = waiter;
        assert!(tokio::time::timeout(Duration::from_millis(20), &mut waiter)
            .await
            .is_err());

        state.finish_initial_recovery(
            "game",
            generation,
            InitialRecoveryOutcome::Failed("disk unavailable".to_string()),
        );
        let claim = tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("gate should open")
            .expect("waiter should complete");
        assert!(matches!(
            claim,
            InitialRecoveryClaim::Finished(InitialRecoveryOutcome::Failed(message))
                if message == "disk unavailable"
        ));
    }

    #[test]
    fn recovery_readiness_reports_pending_without_waiting_for_completion() {
        let state = DiskReconcileState::new();
        let generation = state.mark_initial_recovery_pending("game");

        assert_eq!(
            state.initial_recovery_readiness("game"),
            InitialRecoveryReadiness::Syncing { generation }
        );
    }

    #[tokio::test]
    async fn unmarked_game_is_claimed_for_disk_recovery() {
        let state = DiskReconcileState::new();
        let claim = state.claim_initial_recovery("game").await;
        assert!(matches!(claim, InitialRecoveryClaim::Run { generation: 1 }));
    }

    #[tokio::test]
    async fn reset_ignores_completion_from_an_older_activation() {
        let state = DiskReconcileState::new();
        let old_generation = state.mark_initial_recovery_pending("game");
        state.reset_initial_recovery("game");
        state.finish_initial_recovery(
            "game",
            old_generation,
            InitialRecoveryOutcome::Failed("stale failure".to_string()),
        );

        let claim = state.claim_initial_recovery("game").await;
        assert!(matches!(claim, InitialRecoveryClaim::Run { generation: 3 }));
    }

    #[tokio::test]
    async fn activation_lease_serializes_recovery_and_rollback_ownership() {
        let state = Arc::new(DiskReconcileState::new());
        let first = state.activation_guard().await;
        let waiting_state = Arc::clone(&state);
        let mut second = tokio::spawn(async move {
            let _guard = waiting_state.activation_guard().await;
        });
        assert!(tokio::time::timeout(Duration::from_millis(20), &mut second)
            .await
            .is_err());

        drop(first);
        tokio::time::timeout(Duration::from_secs(1), second)
            .await
            .expect("second activation should proceed after the first terminal result")
            .expect("activation waiter should complete");
    }
}

#[cfg(test)]
mod runtime_effect_retry_tests {
    use super::DiskReconcileState;
    use crate::modules::reconciliation::application::disk_reconcile::types::PendingRuntimeEffects;

    #[test]
    fn runtime_effect_intent_persists_and_merges_until_acknowledged() {
        let state = DiskReconcileState::new();

        let first = state.stage_runtime_effects(
            "game",
            PendingRuntimeEffects {
                collections_dirty: true,
                overlay_refresh: false,
            },
        );
        assert!(first.collections_dirty);
        assert!(!first.overlay_refresh);

        let retry = state.stage_runtime_effects_for_settlement(
            "game",
            PendingRuntimeEffects {
                collections_dirty: false,
                overlay_refresh: true,
            },
        );
        assert!(retry.pending.collections_dirty);
        assert!(retry.pending.overlay_refresh);

        state.acknowledge_runtime_effects("game", retry);
        assert_eq!(
            state.stage_runtime_effects("game", PendingRuntimeEffects::default()),
            PendingRuntimeEffects::default()
        );
    }
}
