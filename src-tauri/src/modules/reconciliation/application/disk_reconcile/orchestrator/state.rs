//! Per-game reconcile locks, activation cache, and runtime-effect state.

use crate::shared::errors::AppError;
use crate::shared::path_key::physical_namespace_path;
use crate::shared::sync::lock;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};

use tokio::sync::{Mutex, Notify, OwnedMutexGuard};

use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcileResult, DiskReconcileScanScope, PendingRuntimeEffects,
};

#[derive(Debug, Default, Clone)]
struct GameSyncState {
    last_result: Option<DiskReconcileResult>,
    reconcile_revision: u64,
    pending_runtime_effects: PendingRuntimeEffects,
    runtime_effects_generation: u64,
    runtime_sync_generation: u64,
    queued_runtime_sync: Option<QueuedRuntimeSync>,
    runtime_sync_worker_running: bool,
}

#[derive(Debug, Clone)]
struct QueuedRuntimeSync {
    generation: u64,
    cause: String,
    publication_revision: Option<u64>,
    activation_authority: Option<ActivationAuthority>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeSyncJob {
    pub(crate) generation: u64,
    pub(crate) cause: String,
    pub(crate) publication_revision: Option<u64>,
    pub(crate) activation_authority: Option<ActivationAuthority>,
}

#[derive(Debug, Clone)]
pub(crate) struct ActivationAuthority {
    epoch: Arc<std::sync::Mutex<ActivationEpochState>>,
    generation: u64,
    game_id: String,
}

impl ActivationAuthority {
    pub(crate) fn with_current<T>(&self, commit: impl FnOnce() -> T) -> Option<T> {
        let epoch = lock(&self.epoch);
        (epoch.generation == self.generation && epoch.game_id.as_deref() == Some(&self.game_id))
            .then(commit)
    }

    pub(crate) fn current_for(game_id: &str) -> Option<Self> {
        let epoch = lock(&ACTIVATION_EPOCH);
        (epoch.game_id.as_deref() == Some(game_id)).then(|| Self {
            epoch: Arc::clone(&ACTIVATION_EPOCH),
            generation: epoch.generation,
            game_id: game_id.to_string(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeSyncEnqueue {
    pub(crate) generation: u64,
    pub(crate) start_worker: bool,
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
    activation_intent: AtomicU64,
    activation: std::sync::Mutex<ActivationCoordinatorState>,
    locks: std::sync::Mutex<HashMap<String, Arc<Mutex<()>>>>,
    games: std::sync::Mutex<HashMap<String, GameSyncState>>,
    initial_recovery: std::sync::Mutex<HashMap<String, Arc<InitialRecoveryGate>>>,
    authority: std::sync::Mutex<HashMap<String, GameAuthorityState>>,
    projection_wakes: std::sync::Mutex<HashMap<String, Arc<Notify>>>,
}

#[derive(Debug, Default)]
struct ActivationCoordinatorState {
    generation: u64,
    game_id: Option<String>,
}

#[derive(Debug, Default)]
struct ActivationEpochState {
    generation: u64,
    game_id: Option<String>,
}

static ACTIVATION_EPOCH: LazyLock<Arc<std::sync::Mutex<ActivationEpochState>>> =
    LazyLock::new(|| Arc::new(std::sync::Mutex::new(ActivationEpochState::default())));

#[cfg(test)]
static ACTIVATION_EPOCH_TEST_LOCK: LazyLock<Arc<tokio::sync::Mutex<()>>> =
    LazyLock::new(|| Arc::new(tokio::sync::Mutex::new(())));

#[cfg(test)]
pub(crate) fn activation_epoch_test_guard() -> tokio::sync::OwnedMutexGuard<()> {
    Arc::clone(&ACTIVATION_EPOCH_TEST_LOCK).blocking_lock_owned()
}

#[cfg(test)]
pub(crate) async fn activation_epoch_test_guard_async() -> tokio::sync::OwnedMutexGuard<()> {
    Arc::clone(&ACTIVATION_EPOCH_TEST_LOCK).lock_owned().await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AuthorityCatchUp {
    Clean {
        reconcile_revision: u64,
        observed_generation: u64,
    },
    Scoped {
        changed_paths: Vec<String>,
        observed_generation: u64,
    },
    Full {
        observed_generation: u64,
    },
}

#[derive(Debug, Default)]
struct GameAuthorityState {
    root_key: String,
    root_identity: Option<String>,
    watcher_session: u64,
    event_generation: u64,
    reconcile_revision: u64,
    dirty_roots: BTreeSet<PathBuf>,
    dropped_events: bool,
    trusted: bool,
}

pub(super) const MAX_SCOPED_DIRTY_ROOTS: usize = 512;

#[derive(Debug, Clone)]
pub(crate) struct TrustedAuthorityEvidence {
    game_id: String,
    mods_root: PathBuf,
    root_key: String,
    root_identity: String,
    watcher_session: u64,
    observed_generation: u64,
}

fn authority_root_key(root: &Path) -> String {
    crate::shared::path_key::canonical_path_key_for_path(root)
}

fn affected_top_root(mods_root: &Path, changed_path: &Path) -> Option<PathBuf> {
    let mods_root = physical_namespace_path(mods_root).ok()?;
    let changed_path = physical_namespace_path(changed_path).ok()?;
    let relative = changed_path.strip_prefix(&mods_root).ok()?;
    let first = relative.components().next()?;
    let name = first.as_os_str().to_string_lossy();
    if name.is_empty() || name.starts_with('.') {
        return None;
    }
    Some(mods_root.join(first.as_os_str()))
}

/// Owned proof that a disk mutation holds both serialization locks in the
/// only supported order: per-game first, then the global operation lock.
pub struct DiskMutationLease {
    _game_guard: OwnedMutexGuard<()>,
    operation_guard: DiskMutationOperationGuard,
}

pub(crate) struct ToggleStorageProof {
    session: crate::modules::workspace::api::scanner::watcher::WatcherSession,
    namespace: Arc<crate::platform::fs::file_utils::FilesystemNamespaceProof>,
}

impl ToggleStorageProof {
    pub(crate) fn session(
        &self,
    ) -> &crate::modules::workspace::api::scanner::watcher::WatcherSession {
        &self.session
    }

    pub(crate) fn namespace(
        &self,
    ) -> Arc<crate::platform::fs::file_utils::FilesystemNamespaceProof> {
        Arc::clone(&self.namespace)
    }
}

enum DiskMutationOperationGuard {
    LockOnly {
        _guard: crate::platform::fs::operation_lock::OpGuard,
    },
    Durable(crate::modules::mutation::coordinator::MutationGuard),
}

impl DiskMutationLease {
    pub(crate) fn from_reconcile_guard(
        game_guard: OwnedMutexGuard<()>,
        operation_guard: crate::platform::fs::operation_lock::OpGuard,
    ) -> Self {
        Self {
            _game_guard: game_guard,
            operation_guard: DiskMutationOperationGuard::LockOnly {
                _guard: operation_guard,
            },
        }
    }

    pub(crate) fn from_durable_guard(
        game_guard: OwnedMutexGuard<()>,
        operation_guard: crate::modules::mutation::coordinator::MutationGuard,
    ) -> Self {
        Self {
            _game_guard: game_guard,
            operation_guard: DiskMutationOperationGuard::Durable(operation_guard),
        }
    }

    pub(crate) fn from_ready_durable_guard(
        state: &DiskReconcileState,
        game_id: &str,
        game_guard: OwnedMutexGuard<()>,
        operation_guard: crate::modules::mutation::coordinator::MutationGuard,
    ) -> Result<Self, AppError> {
        let lease = Self::from_durable_guard(game_guard, operation_guard);
        if let Err(error) = state.ensure_core_ready_for_mutation(game_id) {
            lease.abort_unapplied()?;
            return Err(error);
        }
        Ok(lease)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn from_toggle_scope_guard(
        state: &DiskReconcileState,
        game_id: &str,
        mods_root: &Path,
        paths: &[String],
        game_guard: OwnedMutexGuard<()>,
        operation_guard: crate::modules::mutation::coordinator::MutationGuard,
        storage_proof: Option<(
            &ToggleStorageProof,
            &crate::modules::workspace::api::scanner::watcher::WatcherState,
        )>,
        repair: impl AsyncFnOnce(
            &OwnedMutexGuard<()>,
            &crate::modules::mutation::coordinator::MutationGuard,
        ) -> Result<(), AppError>,
    ) -> Result<Self, AppError> {
        if let Some((proof, watcher)) = storage_proof {
            let validation =
                state.validate_toggle_storage_proof(game_id, mods_root, paths, proof, watcher);
            if let Err(error) = validation {
                operation_guard.abort_unapplied()?;
                return Err(error);
            }
            return Ok(Self::from_durable_guard(game_guard, operation_guard));
        }
        // Watcher observations can arrive while the durable plan is written.
        // Repair once under the same guards without rebinding that plan.
        if !state.toggle_scope_is_ready(game_id, mods_root, paths) {
            if let Err(error) = repair(&game_guard, &operation_guard).await {
                operation_guard.abort_unapplied()?;
                return Err(error);
            }
        }
        let lease = Self::from_durable_guard(game_guard, operation_guard);
        if !state.toggle_scope_is_ready(game_id, mods_root, paths) {
            lease.abort_unapplied()?;
            return Err(AppError::Io(
                "Target disk authority changed during switch preparation; retry after repair"
                    .to_string(),
            ));
        }
        Ok(lease)
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

    pub(crate) fn settle_steps(
        &self,
        settlements: &[(u32, crate::modules::mutation::api::StepSettlement)],
    ) -> Result<(), AppError> {
        match &self.operation_guard {
            DiskMutationOperationGuard::Durable(guard) => guard.settle_steps(settlements),
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

    pub(crate) fn mark_disk_committed(&self) -> Result<u64, AppError> {
        match &self.operation_guard {
            DiskMutationOperationGuard::Durable(guard) => guard.mark_disk_committed(),
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

    pub(crate) fn abort_unapplied(self) -> Result<(), AppError> {
        match self.operation_guard {
            DiskMutationOperationGuard::Durable(guard) => guard.abort_unapplied(),
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
    core_root_identity: Option<String>,
}

impl Default for InitialRecoveryGate {
    fn default() -> Self {
        Self {
            state: std::sync::Mutex::new(InitialRecoveryState {
                generation: 0,
                status: InitialRecoveryStatus::Unstarted,
                core_root_identity: None,
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

    pub(crate) fn projection_wakeup(&self, game_id: &str) -> Arc<Notify> {
        lock(&self.projection_wakes)
            .entry(game_id.to_string())
            .or_insert_with(|| Arc::new(Notify::new()))
            .clone()
    }

    pub async fn acquire_nested_mutation_lease(
        &self,
        game_id: &str,
        coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    ) -> Result<DiskMutationLease, AppError> {
        let game_guard = self.lock_for_game(game_id).lock_owned().await;
        self.ensure_core_ready_for_mutation(game_id)?;
        let operation_guard = coordinator.acquire_nested_operation_lock().await?;
        self.ensure_core_ready_for_mutation(game_id)?;
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

    /// User-initiated writes must recheck readiness after acquiring the game
    /// lock. Recovery of a previously journaled write uses the ungated lease.
    pub async fn acquire_ready_mutation_lease(
        &self,
        game_id: &str,
        operation_lock: &crate::platform::fs::operation_lock::OperationLock,
    ) -> Result<DiskMutationLease, AppError> {
        let game_guard = self.lock_for_game(game_id).lock_owned().await;
        self.ensure_core_ready_for_mutation(game_id)?;
        let operation_guard =
            crate::modules::mutation::coordinator::MutationCoordinator::acquire_nested_operation_on(
                operation_lock,
            )
            .await?;
        self.ensure_core_ready_for_mutation(game_id)?;
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

    pub(crate) fn try_game_guard(&self, game_id: &str) -> Option<OwnedMutexGuard<()>> {
        self.lock_for_game(game_id).try_lock_owned().ok()
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

    pub(crate) fn enqueue_runtime_sync(
        &self,
        game_id: &str,
        cause: impl Into<String>,
        publication_revision: Option<u64>,
        activation_authority: Option<ActivationAuthority>,
    ) -> RuntimeSyncEnqueue {
        let mut games = lock(&self.games);
        let state = games.entry(game_id.to_string()).or_default();
        state.runtime_sync_generation = state.runtime_sync_generation.saturating_add(1);
        let generation = state.runtime_sync_generation;
        state.queued_runtime_sync = Some(QueuedRuntimeSync {
            generation,
            cause: cause.into(),
            publication_revision,
            activation_authority,
        });
        let start_worker = !state.runtime_sync_worker_running;
        state.runtime_sync_worker_running = true;
        RuntimeSyncEnqueue {
            generation,
            start_worker,
        }
    }

    pub(crate) fn claim_latest_runtime_sync(&self, game_id: &str) -> Option<RuntimeSyncJob> {
        let mut games = lock(&self.games);
        let state = games.get_mut(game_id)?;
        state
            .queued_runtime_sync
            .take()
            .map(|queued| RuntimeSyncJob {
                generation: queued.generation,
                cause: queued.cause,
                publication_revision: queued.publication_revision,
                activation_authority: queued.activation_authority,
            })
    }

    pub(crate) fn runtime_sync_is_current(&self, game_id: &str, generation: u64) -> bool {
        lock(&self.games)
            .get(game_id)
            .is_some_and(|state| state.runtime_sync_generation == generation)
    }

    /// Returns true only when this generation is still authoritative and the
    /// worker may stop. A newer queued request keeps the same worker alive.
    pub(crate) fn finish_runtime_sync(&self, game_id: &str, generation: u64) -> bool {
        let mut games = lock(&self.games);
        let Some(state) = games.get_mut(game_id) else {
            return true;
        };
        if state.runtime_sync_generation == generation && state.queued_runtime_sync.is_none() {
            state.runtime_sync_worker_running = false;
            true
        } else {
            false
        }
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

    pub(crate) fn try_activation_guard(&self) -> Option<tokio::sync::MutexGuard<'_, ()>> {
        self.activation_lock.try_lock().ok()
    }

    pub fn reserve_activation_intent(&self) -> u64 {
        self.activation_intent.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn activation_intent_is_current(&self, intent: u64) -> bool {
        self.activation_intent.load(Ordering::Acquire) == intent
    }

    pub fn begin_activation(&self, game_id: Option<String>) -> u64 {
        let generation = {
            let mut epoch = lock(&ACTIVATION_EPOCH);
            epoch.generation = epoch.generation.saturating_add(1);
            epoch.game_id = game_id.clone();
            epoch.generation
        };
        let mut activation = lock(&self.activation);
        activation.generation = generation;
        activation.game_id = game_id;
        generation
    }

    pub fn activation_is_current(&self, game_id: Option<&str>, generation: u64) -> bool {
        let activation = lock(&self.activation);
        activation.generation == generation && activation.game_id.as_deref() == game_id
    }

    pub(crate) fn activation_authority_for(&self, game_id: &str) -> Option<ActivationAuthority> {
        let generation = {
            let activation = lock(&self.activation);
            (activation.game_id.as_deref() == Some(game_id)).then_some(activation.generation)?
        };
        self.activation_authority_for_generation(game_id, generation)
    }

    pub(crate) fn activation_authority_for_generation(
        &self,
        game_id: &str,
        generation: u64,
    ) -> Option<ActivationAuthority> {
        {
            let activation = lock(&self.activation);
            if activation.game_id.as_deref() != Some(game_id) || activation.generation != generation
            {
                return None;
            }
        }
        let authority = ActivationAuthority::current_for(game_id)?;
        (authority.generation == generation).then_some(authority)
    }

    /// Trusted internal mutations may use regional validation only while the
    /// watcher-backed authority for this exact root is complete and clean.
    /// Any gap, unknown identity, or pending external change falls back to the
    /// conservative collector before disk is mutated.
    pub(crate) fn trusted_regional_mutation_allowed(
        &self,
        game_id: &str,
        mods_root: &Path,
    ) -> bool {
        let root_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(mods_root);
        let authority = lock(&self.authority);
        authority.get(game_id).is_some_and(|state| {
            state.root_key == authority_root_key(mods_root)
                && root_identity.is_some()
                && state.root_identity == root_identity
                && state.trusted
                && !state.dropped_events
                && state.dirty_roots.is_empty()
        })
    }

    pub(crate) fn toggle_scope_is_ready(
        &self,
        game_id: &str,
        mods_root: &Path,
        paths: &[String],
    ) -> bool {
        self.toggle_scope_is_ready_for_session(game_id, mods_root, paths, None)
    }

    pub(crate) fn capture_toggle_storage_proof(
        &self,
        game_id: &str,
        mods_root: &Path,
        paths: &[PathBuf],
        watcher: &crate::modules::workspace::api::scanner::watcher::WatcherState,
    ) -> Result<Option<ToggleStorageProof>, AppError> {
        let Some(session) = watcher.current_session_for_root(mods_root) else {
            return Ok(None);
        };
        if paths.is_empty()
            || !self.toggle_storage_coverage_is_ready(game_id, mods_root, session.generation())
            || watcher.suppressor.has_unrepaired_drops()
            || watcher.suppressor.pending_repair(&session).is_some()
        {
            return Ok(None);
        }
        Ok(Some(ToggleStorageProof {
            session,
            namespace: Arc::new(
                crate::platform::fs::file_utils::FilesystemNamespaceProof::capture(
                    mods_root, paths,
                )?,
            ),
        }))
    }

    pub(crate) fn validate_toggle_storage_proof(
        &self,
        game_id: &str,
        mods_root: &Path,
        paths: &[String],
        proof: &ToggleStorageProof,
        watcher: &crate::modules::workspace::api::scanner::watcher::WatcherState,
    ) -> Result<(), AppError> {
        if paths.is_empty()
            || watcher.current_session_for_root(mods_root).as_ref() != Some(&proof.session)
            || !self.toggle_storage_coverage_is_ready(
                game_id,
                mods_root,
                proof.session.generation(),
            )
            || watcher.suppressor.has_unrepaired_drops()
            || watcher.suppressor.pending_repair(&proof.session).is_some()
        {
            #[cfg(debug_assertions)]
            {
                let identity = crate::platform::fs::file_utils::filesystem_identity(mods_root);
                let (trusted, dropped, root_matches) = lock(&self.authority)
                    .get(game_id)
                    .map(|state| {
                        (
                            state.trusted,
                            state.dropped_events,
                            state.root_identity == identity,
                        )
                    })
                    .unwrap_or((false, false, false));
                log::info!("toggle storage proof rejected game_id={} session={} current_session_matches={} indexed_root={} suppressor_repair={} authority_trusted={} authority_dropped={} root_matches={}",
                    game_id, proof.session.generation(), watcher.current_session_for_root(mods_root).as_ref() == Some(&proof.session),
                    self.ensure_indexed_root(game_id, mods_root).is_ok(), watcher.suppressor.has_unrepaired_drops(),
                    trusted, dropped, root_matches);
            }
            return Err(AppError::Io(
                "Toggle watcher coverage changed; repair indexing before retrying".into(),
            ));
        }
        proof
            .namespace
            .validate_paths(&paths.iter().map(PathBuf::from).collect::<Vec<_>>())
    }

    fn toggle_storage_coverage_is_ready(
        &self,
        game_id: &str,
        mods_root: &Path,
        watcher_session: u64,
    ) -> bool {
        if self.ensure_indexed_root(game_id, mods_root).is_err() {
            return false;
        }
        let current_identity = crate::platform::fs::file_utils::filesystem_identity(mods_root);
        lock(&self.authority).get(game_id).is_some_and(|state| {
            state.root_key == authority_root_key(mods_root)
                && state.watcher_session == watcher_session
                && state.root_identity.is_some()
                && state.root_identity == current_identity
                && state.trusted
                && !state.dropped_events
        })
    }

    pub(crate) fn trusted_toggle_session_is_ready(
        &self,
        game_id: &str,
        mods_root: &Path,
        paths: &[String],
        watcher_session: u64,
    ) -> bool {
        self.toggle_scope_is_ready_for_session(game_id, mods_root, paths, Some(watcher_session))
    }

    fn toggle_scope_is_ready_for_session(
        &self,
        game_id: &str,
        mods_root: &Path,
        paths: &[String],
        watcher_session: Option<u64>,
    ) -> bool {
        if self.ensure_indexed_root(game_id, mods_root).is_err() || paths.is_empty() {
            return false;
        }
        let Ok(paths) = paths
            .iter()
            .map(|path| physical_namespace_path(Path::new(path)))
            .collect::<std::io::Result<Vec<_>>>()
        else {
            return false;
        };
        let identity = crate::platform::fs::file_utils::filesystem_identity(mods_root);
        lock(&self.authority).get(game_id).is_some_and(|state| {
            state.root_key == authority_root_key(mods_root)
                && watcher_session.is_none_or(|session| state.watcher_session == session)
                && identity.is_some()
                && state.root_identity == identity
                && state.trusted
                && !state.dropped_events
                && !state.dirty_roots.iter().any(|dirty| {
                    paths
                        .iter()
                        .any(|path| path.starts_with(dirty) || dirty.starts_with(path))
                })
        })
    }

    pub(crate) fn ensure_indexed_root(
        &self,
        game_id: &str,
        mods_root: &Path,
    ) -> Result<(), AppError> {
        self.ensure_core_recovery_allows_preflight(game_id)?;
        let current = crate::platform::fs::file_utils::filesystem_identity(mods_root);
        let gate = self.initial_recovery_gate(game_id);
        let proof = lock(&gate.state).core_root_identity.clone();
        if current.is_some() && proof == current {
            Ok(())
        } else {
            Err(AppError::Io("Mods root identity changed or core proof is unavailable; index this game before modifying mods".to_string()))
        }
    }

    /// Starts watcher coverage without continuity proof. A full catch-up is
    /// required before this session may become authoritative.
    pub(crate) fn begin_authority_session(
        &self,
        game_id: &str,
        mods_root: &Path,
        watcher_session: u64,
    ) {
        let root_key = authority_root_key(mods_root);
        let root_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(mods_root);
        let mut authority = lock(&self.authority);
        let state = authority.entry(game_id.to_string()).or_default();
        if watcher_session < state.watcher_session {
            return;
        }
        *state = GameAuthorityState {
            root_key,
            root_identity,
            watcher_session,
            dropped_events: true,
            ..GameAuthorityState::default()
        };
        self.projection_wakeup(game_id).notify_one();
    }

    /// Moves the existing observation to an overlapping watcher without
    /// acknowledging any events or changing its reconcile revision.
    pub(crate) fn handoff_authority_session(
        &self,
        game_id: &str,
        mods_root: &Path,
        covered_session: u64,
        watcher_session: u64,
    ) -> bool {
        let root_key = authority_root_key(mods_root);
        let root_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(mods_root);
        let mut authority = lock(&self.authority);
        let Some(state) = authority.get_mut(game_id) else {
            return false;
        };
        if watcher_session <= covered_session
            || state.watcher_session != covered_session
            || state.root_key != root_key
            || root_identity.is_none()
            || state.root_identity != root_identity
        {
            return false;
        }
        state.watcher_session = watcher_session;
        true
    }

    pub(crate) fn invalidate_authority(&self, game_id: &str, mods_root: &Path) {
        let root_key = authority_root_key(mods_root);
        let root_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(mods_root);
        let mut authority = lock(&self.authority);
        let state = authority.entry(game_id.to_string()).or_default();
        if state.root_key != root_key || state.root_identity != root_identity {
            *state = GameAuthorityState {
                root_key,
                root_identity,
                dropped_events: true,
                ..GameAuthorityState::default()
            };
            return;
        }
        state.event_generation = state.event_generation.saturating_add(1);
        state.dropped_events = true;
        state.trusted = false;
    }

    pub(crate) fn trusted_internal_mutation_evidence(
        &self,
        game_id: &str,
        mods_root: &Path,
        watcher_session: u64,
    ) -> Option<TrustedAuthorityEvidence> {
        let root_key = authority_root_key(mods_root);
        let root_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(mods_root)?;
        let authority = lock(&self.authority);
        let state = authority.get(game_id)?;
        (state.root_key == root_key
            && state.root_identity.as_deref() == Some(root_identity.as_str())
            && state.watcher_session == watcher_session
            && state.trusted
            && !state.dropped_events
            && state.dirty_roots.is_empty())
        .then(|| TrustedAuthorityEvidence {
            game_id: game_id.to_string(),
            mods_root: mods_root.to_path_buf(),
            root_key,
            root_identity,
            watcher_session,
            observed_generation: state.event_generation,
        })
    }

    pub(crate) fn mark_trusted_internal_mutation_reconciled(
        &self,
        evidence: &TrustedAuthorityEvidence,
        result: &DiskReconcileResult,
        changed_paths: &[String],
    ) -> bool {
        if authority_root_key(&evidence.mods_root) != evidence.root_key
            || crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&evidence.mods_root)
                .as_deref()
                != Some(evidence.root_identity.as_str())
        {
            return false;
        }
        self.mark_authority_reconciled(
            &evidence.game_id,
            &evidence.mods_root,
            evidence.watcher_session,
            evidence.observed_generation,
            result,
            changed_paths,
        )
    }

    /// Records watcher evidence before suppression or batching. This makes a
    /// clean token conservative: app-owned writes may make it dirty, but no
    /// filesystem write can silently preserve a stale clean token.
    pub(crate) fn observe_authority_event(
        &self,
        game_id: &str,
        watcher_session: u64,
        mods_root: &Path,
        paths: &[PathBuf],
        events_lost: bool,
    ) {
        let root_key = authority_root_key(mods_root);
        let root_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(mods_root);
        let mut authority = lock(&self.authority);
        let state = authority.entry(game_id.to_string()).or_default();
        if watcher_session < state.watcher_session {
            if state.root_key == root_key {
                state.event_generation = state.event_generation.saturating_add(1);
                state.dropped_events = true;
            }
            return;
        }
        if watcher_session > state.watcher_session || state.root_key != root_key {
            *state = GameAuthorityState {
                root_key,
                root_identity,
                watcher_session,
                dropped_events: true,
                ..GameAuthorityState::default()
            };
        }
        state.event_generation = state.event_generation.saturating_add(1);
        state.dropped_events |= events_lost;
        let physical_root = physical_namespace_path(mods_root);
        for path in paths {
            if let Some(root) = affected_top_root(mods_root, path) {
                state.dirty_roots.insert(root);
                if state.dirty_roots.len() > MAX_SCOPED_DIRTY_ROOTS {
                    state.dirty_roots.clear();
                    state.dropped_events = true;
                    break;
                }
            } else if physical_root
                .as_ref()
                .ok()
                .zip(physical_namespace_path(path).ok())
                .is_none_or(|(root, path)| path == *root || !path.starts_with(root))
            {
                // Importer/settings changes sit outside Mods and can alter the
                // effective runtime roots. A root-only event is equally
                // ambiguous because it may summarize unknown descendant work.
                state.dropped_events = true;
            }
        }
    }

    pub(crate) fn end_authority_session(
        &self,
        game_id: &str,
        watcher_session: u64,
        mods_root: &Path,
    ) {
        let mut authority = lock(&self.authority);
        if let Some(state) = authority.get_mut(game_id) {
            if state.watcher_session == watcher_session
                && state.root_key == authority_root_key(mods_root)
            {
                state.event_generation = state.event_generation.saturating_add(1);
                state.dropped_events = true;
            }
        }
    }

    pub(crate) fn authority_catch_up(
        &self,
        game_id: &str,
        mods_root: &Path,
        watcher_session: u64,
    ) -> AuthorityCatchUp {
        let root_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(mods_root);
        let authority = lock(&self.authority);
        let Some(state) = authority.get(game_id) else {
            return AuthorityCatchUp::Full {
                observed_generation: 0,
            };
        };
        let observed_generation = state.event_generation;
        if state.watcher_session != watcher_session
            || state.root_key != authority_root_key(mods_root)
            || root_identity.is_none()
            || state.root_identity != root_identity
            || !state.trusted
            || state.dropped_events
        {
            return AuthorityCatchUp::Full {
                observed_generation,
            };
        }
        if state.dirty_roots.is_empty() {
            return AuthorityCatchUp::Clean {
                reconcile_revision: state.reconcile_revision,
                observed_generation,
            };
        }
        AuthorityCatchUp::Scoped {
            changed_paths: state
                .dirty_roots
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            observed_generation,
        }
    }

    pub(crate) fn authority_event_generation(
        &self,
        game_id: &str,
        watcher_session: u64,
    ) -> Option<u64> {
        lock(&self.authority)
            .get(game_id)
            .filter(|state| state.watcher_session == watcher_session)
            .map(|state| state.event_generation)
    }

    /// Accepts a reconcile as authority only if no watcher observation arrived
    /// after its input snapshot. Newer evidence remains dirty for the next
    /// batch instead of being accidentally acknowledged.
    pub(crate) fn mark_authority_reconciled(
        &self,
        game_id: &str,
        mods_root: &Path,
        watcher_session: u64,
        observed_generation: u64,
        result: &DiskReconcileResult,
        changed_paths: &[String],
    ) -> bool {
        let root_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(mods_root);
        let games = lock(&self.games);
        if games
            .get(game_id)
            .and_then(|game| game.last_result.as_ref())
            .is_none_or(|latest| latest.reconcile_revision != result.reconcile_revision)
        {
            return false;
        }
        let mut authority = lock(&self.authority);
        let Some(state) = authority.get_mut(game_id) else {
            return false;
        };
        if state.watcher_session != watcher_session
            || state.event_generation != observed_generation
            || state.root_key != authority_root_key(mods_root)
            || root_identity.is_none()
            || state.root_identity != root_identity
            || !result.status.applied()
            || result.reconcile_revision < state.reconcile_revision
        {
            return false;
        }

        match result.scan_scope {
            crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileScanScope::Full => {
                state.dirty_roots.clear();
                state.dropped_events = false;
            }
            crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileScanScope::Scoped => {
                for changed_path in changed_paths {
                    if let Some(root) = affected_top_root(mods_root, Path::new(changed_path)) {
                        state.dirty_roots.remove(&root);
                    }
                }
            }
            crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileScanScope::None => {}
        }
        state.reconcile_revision = result.reconcile_revision;
        state.trusted = true;
        self.projection_wakeup(game_id).notify_one();
        if let Some(gate) = lock(&self.initial_recovery).get(game_id).cloned() {
            let mut core = lock(&gate.state);
            if core.core_root_identity.is_none()
                && matches!(&core.status,
                InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Completed(result)) if result.status.applied())
            {
                core.core_root_identity = state.root_identity.clone();
            }
        }
        true
    }

    pub(crate) fn authoritative_result(&self, game_id: &str) -> Option<DiskReconcileResult> {
        lock(&self.games)
            .get(game_id)
            .and_then(|state| state.last_result.clone())
    }

    pub(crate) fn clean_authoritative_result(
        &self,
        game_id: &str,
        mods_root: &Path,
        watcher_session: u64,
    ) -> Option<DiskReconcileResult> {
        let (reconcile_revision, observed_generation) =
            match self.authority_catch_up(game_id, mods_root, watcher_session) {
                AuthorityCatchUp::Clean {
                    reconcile_revision,
                    observed_generation,
                } => (reconcile_revision, observed_generation),
                AuthorityCatchUp::Scoped { .. } | AuthorityCatchUp::Full { .. } => return None,
            };
        let mut result = self.authoritative_result(game_id)?;
        if !result.status.applied() || result.reconcile_revision != reconcile_revision {
            return None;
        }
        if !matches!(
            self.authority_catch_up(game_id, mods_root, watcher_session),
            AuthorityCatchUp::Clean {
                reconcile_revision: current_revision,
                observed_generation: current_generation,
            } if current_revision == reconcile_revision && current_generation == observed_generation
        ) {
            return None;
        }
        result.scan_scope = DiskReconcileScanScope::None;
        Some(result)
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
        let (generation, core_applied) = {
            let state = lock(&gate.state);
            match &state.status {
                InitialRecoveryStatus::Unstarted => {
                    return InitialRecoveryReadiness::Unstarted {
                        generation: state.generation,
                    };
                }
                InitialRecoveryStatus::Pending => {
                    return InitialRecoveryReadiness::Syncing {
                        generation: state.generation,
                    };
                }
                InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Failed(_)) => {
                    return InitialRecoveryReadiness::Failed {
                        generation: state.generation,
                    };
                }
                InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Completed(result)) => {
                    (state.generation, result.status.applied())
                }
            }
        };
        if core_applied {
            InitialRecoveryReadiness::Ready { generation }
        } else {
            InitialRecoveryReadiness::Failed { generation }
        }
    }

    pub fn ensure_core_ready_for_mutation(&self, game_id: &str) -> Result<(), AppError> {
        match self.initial_recovery_readiness(game_id) {
            InitialRecoveryReadiness::Ready { .. } => {
                let revision = lock(&self.games)
                    .get(game_id)
                    .and_then(|game| game.last_result.as_ref())
                    .map(|result| result.reconcile_revision);
                let clean = lock(&self.authority).get(game_id).is_some_and(|state| {
                    state.trusted
                        && !state.dropped_events
                        && state.dirty_roots.is_empty()
                        && Some(state.reconcile_revision) == revision
                });
                if clean {
                    Ok(())
                } else {
                    Err(AppError::Io("Disk watcher authority needs repair before this operation; core indexing is already complete".to_string()))
                }
            }
            InitialRecoveryReadiness::Unstarted { .. }
            | InitialRecoveryReadiness::Syncing { .. }
            | InitialRecoveryReadiness::Failed { .. } => Err(AppError::Io(
                "Game indexing is not ready. Wait for indexing or retry it before modifying mods."
                    .to_string(),
            )),
        }
    }

    /// A preflight may repair dirty watcher authority, but it must not run
    /// before the first core disk observation has completed successfully.
    pub fn ensure_core_recovery_allows_preflight(&self, game_id: &str) -> Result<(), AppError> {
        let gate = self.initial_recovery_gate(game_id);
        let completed_revision = match &lock(&gate.state).status {
            InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Completed(result))
                if result.status.applied() =>
            {
                Some(result.reconcile_revision)
            }
            _ => None,
        };
        let current_result_exists = lock(&self.games)
            .get(game_id)
            .and_then(|game| game.last_result.as_ref())
            .is_some_and(|result| {
                completed_revision.is_some_and(|revision| result.reconcile_revision >= revision)
            });
        if completed_revision.is_some() && current_result_exists {
            Ok(())
        } else {
            Err(AppError::Io(
                "Game indexing is not ready. Wait for indexing or retry it before modifying mods."
                    .to_string(),
            ))
        }
    }

    pub(crate) fn reject_untrusted_reconcile(&self, game_id: &str, reconcile_revision: u64) {
        let gate = self.initial_recovery_gate(game_id);
        let mut recovery = lock(&gate.state);
        if matches!(
            &recovery.status,
            InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Completed(result))
                if result.reconcile_revision == reconcile_revision
        ) {
            recovery.status = InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Failed(
                "Disk changed before indexing could be validated".to_string(),
            ));
            gate.notify.notify_waiters();
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
            state.core_root_identity = None;
            Arc::clone(&gate)
        };
        gate.notify.notify_waiters();
    }

    pub fn finish_initial_recovery(
        &self,
        game_id: &str,
        generation: u64,
        outcome: InitialRecoveryOutcome,
    ) -> bool {
        let core_root_identity = lock(&self.authority)
            .get(game_id)
            .filter(|authority| authority.trusted && !authority.dropped_events)
            .and_then(|authority| authority.root_identity.clone());
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
                    state.core_root_identity = core_root_identity;
                    true
                }
            };
            if finished {
                gate.notify.notify_waiters();
            }
            finished
        } else {
            false
        }
    }

    pub(crate) fn initial_recovery_generation_is_pending(
        &self,
        game_id: &str,
        generation: u64,
    ) -> bool {
        let gate = self.initial_recovery_gate(game_id);
        let recovery = lock(&gate.state);
        recovery.generation == generation
            && matches!(recovery.status, InitialRecoveryStatus::Pending)
    }

    pub(crate) fn complete_verified_onboarding_recovery(
        &self,
        game_id: &str,
        generation: u64,
        result: &DiskReconcileResult,
    ) -> bool {
        if result.game_id != game_id || !result.status.applied() {
            return false;
        }
        let games = lock(&self.games);
        if games
            .get(game_id)
            .and_then(|game| game.last_result.as_ref())
            .is_none_or(|latest| {
                latest.reconcile_revision != result.reconcile_revision || !latest.status.applied()
            })
        {
            return false;
        }
        let authority = lock(&self.authority);
        if authority.get(game_id).is_none_or(|state| {
            !state.trusted
                || state.dropped_events
                || !state.dirty_roots.is_empty()
                || state.reconcile_revision != result.reconcile_revision
        }) {
            return false;
        }
        let gate = self.initial_recovery_gate(game_id);
        let mut recovery = lock(&gate.state);
        if recovery.generation != generation {
            return false;
        }
        if matches!(
            &recovery.status,
            InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Completed(current))
                if current.reconcile_revision == result.reconcile_revision
        ) {
            return true;
        }
        if !matches!(recovery.status, InitialRecoveryStatus::Pending) {
            return false;
        }
        recovery.status = InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Completed(
            Box::new(result.clone()),
        ));
        recovery.core_root_identity = authority
            .get(game_id)
            .and_then(|state| state.root_identity.clone());
        drop(recovery);
        gate.notify.notify_waiters();
        true
    }

    pub async fn claim_initial_recovery(&self, game_id: &str) -> InitialRecoveryClaim {
        loop {
            let gate = self.initial_recovery_gate(game_id);
            let notified = gate.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
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

    pub(crate) fn record_result(&self, game_id: &str, result: &mut DiskReconcileResult) {
        {
            let mut games = lock(&self.games);
            let state = games.entry(game_id.to_string()).or_default();
            state.reconcile_revision = state.reconcile_revision.saturating_add(1);
            result.reconcile_revision = state.reconcile_revision;
            state.last_result = Some(result.clone());
        }

        // The first applied core index remains the preflight entitlement even
        // if a later watcher pass cannot read the source. Latest disk truth and
        // watcher authority are checked separately for mutation readiness.
        if let Some(gate) = lock(&self.initial_recovery).get(game_id).cloned() {
            let updated = {
                let mut recovery = lock(&gate.state);
                let can_upgrade_core = match &recovery.status {
                    InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Failed(_)) => true,
                    InitialRecoveryStatus::Finished(InitialRecoveryOutcome::Completed(
                        previous,
                    )) => !previous.status.applied(),
                    _ => false,
                };
                if result.status.applied() && can_upgrade_core {
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
}

#[cfg(test)]
mod initial_recovery_tests {
    use super::{
        AuthorityCatchUp, DiskReconcileState, GameAuthorityState, InitialRecoveryClaim,
        InitialRecoveryOutcome, InitialRecoveryReadiness,
    };
    use crate::modules::reconciliation::application::disk_reconcile::types::{
        DiskReconcileReason, DiskReconcileResult, DiskReconcileScanScope, DiskReconcileStatus,
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

    fn recovery_result(status: DiskReconcileStatus) -> DiskReconcileResult {
        DiskReconcileResult {
            game_id: "game".to_string(),
            reconcile_revision: 0,
            reason: DiskReconcileReason::StartupBoot,
            status,
            scan_scope: DiskReconcileScanScope::Full,
            folder_conflicts: vec![],
            rename_confirmations: vec![],
            error_message: None,
            changed_roots: vec![],
            objects_changed: false,
            folders_changed: false,
            collections_changed: false,
            runtime_file_changed: false,
            thumbnail_roots: vec![],
            cleared_selection_paths: vec![],
            path_updates: vec![],
            collection_reference_impact: Default::default(),
            change_summary: Default::default(),
            pending_runtime_effects: Default::default(),
            warnings: vec![],
        }
    }

    fn trust_result(state: &DiskReconcileState, result: &DiskReconcileResult) {
        crate::shared::sync::lock(&state.authority).insert(
            "game".to_string(),
            GameAuthorityState {
                reconcile_revision: result.reconcile_revision,
                trusted: true,
                ..GameAuthorityState::default()
            },
        );
    }

    #[test]
    fn verified_onboarding_result_opens_mutation_gate_without_another_scan() {
        let state = DiskReconcileState::new();
        let generation = state.mark_initial_recovery_pending("game");
        let mut result = recovery_result(DiskReconcileStatus::Applied);
        result.reason = DiskReconcileReason::OnboardingCompleted;
        state.record_result("game", &mut result);
        trust_result(&state, &result);

        assert!(state.complete_verified_onboarding_recovery("game", generation, &result));
        assert!(state.ensure_core_ready_for_mutation("game").is_ok());
        assert!(state.ensure_core_recovery_allows_preflight("game").is_ok());
    }

    #[test]
    fn verified_onboarding_result_supersedes_an_older_pending_recovery() {
        let state = DiskReconcileState::new();
        let stale_generation = state.mark_initial_recovery_pending("game");
        let onboarding_generation = state.mark_initial_recovery_pending("game");
        assert!(!state.initial_recovery_generation_is_pending("game", stale_generation));
        assert!(state.initial_recovery_generation_is_pending("game", onboarding_generation));
        let mut result = recovery_result(DiskReconcileStatus::Applied);
        result.reason = DiskReconcileReason::OnboardingCompleted;
        state.record_result("game", &mut result);
        trust_result(&state, &result);

        assert!(state.complete_verified_onboarding_recovery(
            "game",
            onboarding_generation,
            &result
        ));
        assert!(!state.initial_recovery_generation_is_pending("game", onboarding_generation));
        assert!(!state.finish_initial_recovery(
            "game",
            stale_generation,
            InitialRecoveryOutcome::Failed("stale recovery".to_string()),
        ));
        assert!(state.ensure_core_ready_for_mutation("game").is_ok());
    }

    #[test]
    fn running_recovery_cannot_finish_after_onboarding_takes_ownership() {
        let state = DiskReconcileState::new();
        let stale_generation = state.mark_initial_recovery_pending("game");
        let onboarding_generation = state.mark_initial_recovery_pending("game");
        let mut stale_result = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut stale_result);
        assert!(!state.finish_initial_recovery(
            "game",
            stale_generation,
            InitialRecoveryOutcome::Completed(Box::new(stale_result)),
        ));
        assert!(state.initial_recovery_generation_is_pending("game", onboarding_generation));

        let mut onboarding_result = recovery_result(DiskReconcileStatus::Applied);
        onboarding_result.reason = DiskReconcileReason::OnboardingCompleted;
        state.record_result("game", &mut onboarding_result);
        trust_result(&state, &onboarding_result);
        assert!(state.complete_verified_onboarding_recovery(
            "game",
            onboarding_generation,
            &onboarding_result,
        ));
        assert!(state.ensure_core_ready_for_mutation("game").is_ok());
    }

    #[test]
    fn later_unavailable_watcher_result_keeps_core_repair_preflight_open() {
        let state = DiskReconcileState::new();
        let generation = state.mark_initial_recovery_pending("game");
        let mut initial = recovery_result(DiskReconcileStatus::Applied);
        initial.reason = DiskReconcileReason::OnboardingCompleted;
        state.record_result("game", &mut initial);
        trust_result(&state, &initial);
        assert!(state.complete_verified_onboarding_recovery("game", generation, &initial));

        let mut unavailable = recovery_result(DiskReconcileStatus::SourceUnavailable);
        state.record_result("game", &mut unavailable);
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
        state
            .ensure_core_recovery_allows_preflight("game")
            .expect("a later watcher failure must not erase successful core indexing");

        let mut repaired = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut repaired);
        trust_result(&state, &repaired);
        assert!(state.ensure_core_ready_for_mutation("game").is_ok());
    }

    #[test]
    fn later_untrusted_watcher_result_blocks_rename_but_allows_repair() {
        let state = DiskReconcileState::new();
        let generation = state.mark_initial_recovery_pending("game");
        let mut initial = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut initial);
        trust_result(&state, &initial);
        state.finish_initial_recovery(
            "game",
            generation,
            InitialRecoveryOutcome::Completed(Box::new(initial)),
        );

        let mut untrusted = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut untrusted);
        state.reject_untrusted_reconcile("game", untrusted.reconcile_revision);
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
        assert!(state.ensure_core_recovery_allows_preflight("game").is_ok());

        trust_result(&state, &untrusted);
        assert!(state.ensure_core_ready_for_mutation("game").is_ok());
    }

    #[test]
    fn onboarding_cannot_open_mutation_gate_for_stale_or_dirty_authority() {
        let state = DiskReconcileState::new();
        let generation = state.mark_initial_recovery_pending("game");
        let mut result = recovery_result(DiskReconcileStatus::Applied);
        result.reason = DiskReconcileReason::OnboardingCompleted;
        state.record_result("game", &mut result);
        assert!(!state.complete_verified_onboarding_recovery("game", generation, &result));

        trust_result(&state, &result);
        crate::shared::sync::lock(&state.authority)
            .get_mut("game")
            .expect("trusted game")
            .dirty_roots
            .insert(std::path::PathBuf::from("changed"));
        assert!(!state.complete_verified_onboarding_recovery("game", generation, &result));

        trust_result(&state, &result);
        let mut newer = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut newer);
        assert!(!state.complete_verified_onboarding_recovery("game", generation, &result));
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
    }

    #[test]
    fn only_applied_terminal_results_make_core_ready() {
        for status in [
            DiskReconcileStatus::Applied,
            DiskReconcileStatus::AppliedWithFolderConflicts,
            DiskReconcileStatus::SourceUnavailable,
            DiskReconcileStatus::NeedsRenameConfirmation,
        ] {
            let state = DiskReconcileState::new();
            let generation = state.mark_initial_recovery_pending("game");
            let mut result = recovery_result(status.clone());
            state.record_result("game", &mut result);
            if status.applied() {
                trust_result(&state, &result);
            }
            state.finish_initial_recovery(
                "game",
                generation,
                InitialRecoveryOutcome::Completed(Box::new(result)),
            );
            assert_eq!(
                state.initial_recovery_readiness("game")
                    == InitialRecoveryReadiness::Ready { generation },
                status.applied()
            );
            assert_eq!(
                state.ensure_core_ready_for_mutation("game").is_ok(),
                status.applied()
            );
        }
    }

    #[test]
    fn applied_projection_waits_for_authority_proof_before_allowing_mutations() {
        let state = DiskReconcileState::new();
        let generation = state.mark_initial_recovery_pending("game");
        let mut result = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut result);
        state.finish_initial_recovery(
            "game",
            generation,
            InitialRecoveryOutcome::Completed(Box::new(result.clone())),
        );
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
        trust_result(&state, &result);
        assert!(state.ensure_core_ready_for_mutation("game").is_ok());

        let mut next = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut next);
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
        trust_result(&state, &next);
        assert!(state.ensure_core_ready_for_mutation("game").is_ok());
        crate::shared::sync::lock(&state.authority)
            .get_mut("game")
            .expect("trusted game")
            .dirty_roots
            .insert(std::path::PathBuf::from("external-change"));
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
        trust_result(&state, &next);
        crate::shared::sync::lock(&state.authority)
            .get_mut("game")
            .expect("trusted game")
            .dropped_events = true;
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
    }

    #[test]
    fn unknown_pending_and_failed_core_recovery_reject_mutations() {
        let state = DiskReconcileState::new();
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
        let generation = state.mark_initial_recovery_pending("game");
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
        state.finish_initial_recovery(
            "game",
            generation,
            InitialRecoveryOutcome::Failed("disk unavailable".to_string()),
        );
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
    }

    #[test]
    fn indexed_toggle_scope_ignores_unrelated_dirty_roots_but_rejects_gaps_and_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Mods");
        std::fs::create_dir_all(root.join("Alice/Skin")).unwrap();
        let state = DiskReconcileState::new();
        let generation = state.mark_initial_recovery_pending("game");
        state.begin_authority_session("game", &root, 1);
        let mut result = recovery_result(DiskReconcileStatus::Applied);
        result.scan_scope = DiskReconcileScanScope::Full;
        state.record_result("game", &mut result);
        assert!(state.mark_authority_reconciled("game", &root, 1, 0, &result, &[]));
        state.finish_initial_recovery(
            "game",
            generation,
            InitialRecoveryOutcome::Completed(Box::new(result)),
        );
        let paths = vec![root.join("Alice/Skin").to_string_lossy().into_owned()];
        assert!(state.toggle_scope_is_ready("game", &root, &paths));
        state.observe_authority_event("game", 1, &root, &[root.join("Bob/Other")], false);
        assert!(state.toggle_scope_is_ready("game", &root, &paths));
        assert!(state.trusted_toggle_session_is_ready("game", &root, &paths, 1));
        assert!(!state.trusted_toggle_session_is_ready("game", &root, &paths, 2));
        assert!(state
            .trusted_internal_mutation_evidence("game", &root, 1)
            .is_none());
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
        assert_eq!(
            state.initial_recovery_readiness("game"),
            InitialRecoveryReadiness::Ready { generation }
        );
        state.observe_authority_event("game", 1, &root, &[root.join("Alice/Other")], false);
        assert!(!state.toggle_scope_is_ready("game", &root, &paths));
        assert!(!state.trusted_toggle_session_is_ready("game", &root, &paths, 1));
        state.invalidate_authority("game", &root);
        assert!(!state.toggle_scope_is_ready("game", &root, &paths));
        assert!(!state.trusted_toggle_session_is_ready("game", &root, &paths, 1));
        std::fs::rename(&root, temp.path().join("OriginalMods")).unwrap();
        std::fs::create_dir(&root).unwrap();
        assert!(state.ensure_indexed_root("game", &root).is_err());
        assert!(!state.trusted_toggle_session_is_ready("game", &root, &paths, 1));
    }

    #[tokio::test]
    async fn mutation_lease_rechecks_readiness_after_waiting_for_game_lock() {
        let state = Arc::new(DiskReconcileState::new());
        let generation = state.mark_initial_recovery_pending("game");
        let mut result = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut result);
        trust_result(&state, &result);
        state.finish_initial_recovery(
            "game",
            generation,
            InitialRecoveryOutcome::Completed(Box::new(result)),
        );
        let game_guard = state.game_lock("game").lock_owned().await;
        let operation_lock = Arc::new(crate::platform::fs::operation_lock::OperationLock::new());
        let waiting_state = Arc::clone(&state);
        let waiting_lock = Arc::clone(&operation_lock);
        let mut waiter = tokio::spawn(async move {
            waiting_state
                .acquire_ready_mutation_lease("game", &waiting_lock)
                .await
        });
        assert!(tokio::time::timeout(Duration::from_millis(20), &mut waiter)
            .await
            .is_err());
        state.mark_initial_recovery_pending("game");
        drop(game_guard);
        let result = tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("waiting command should finish")
            .expect("waiting task should complete");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mutation_lease_rechecks_readiness_after_waiting_for_operation_lock() {
        let state = Arc::new(DiskReconcileState::new());
        let generation = state.mark_initial_recovery_pending("game");
        let mut result = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut result);
        trust_result(&state, &result);
        state.finish_initial_recovery(
            "game",
            generation,
            InitialRecoveryOutcome::Completed(Box::new(result)),
        );
        let operation_lock = Arc::new(crate::platform::fs::operation_lock::OperationLock::new());
        let operation_guard = operation_lock.acquire().await.expect("operation lock");
        let waiting_state = Arc::clone(&state);
        let waiting_lock = Arc::clone(&operation_lock);
        let mut waiter = tokio::spawn(async move {
            waiting_state
                .acquire_ready_mutation_lease("game", &waiting_lock)
                .await
        });
        assert!(tokio::time::timeout(Duration::from_millis(20), &mut waiter)
            .await
            .is_err());
        state.mark_initial_recovery_pending("game");
        drop(operation_guard);
        let result = tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("waiting command should finish")
            .expect("waiting task should complete");
        assert!(result.is_err());
    }

    #[test]
    fn later_unavailable_reconcile_preserves_core_but_blocks_unverified_mutation() {
        let state = DiskReconcileState::new();
        let generation = state.mark_initial_recovery_pending("game");
        let mut applied = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut applied);
        trust_result(&state, &applied);
        state.finish_initial_recovery(
            "game",
            generation,
            InitialRecoveryOutcome::Completed(Box::new(applied)),
        );
        let mut unavailable = recovery_result(DiskReconcileStatus::SourceUnavailable);
        state.record_result("game", &mut unavailable);
        assert_eq!(
            state.initial_recovery_readiness("game"),
            InitialRecoveryReadiness::Ready { generation }
        );
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
    }

    #[test]
    fn completed_core_and_latest_projection_have_independent_eligibility() {
        let state = DiskReconcileState::new();
        let generation = state.mark_initial_recovery_pending("game");
        let mut applied = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut applied);
        let mut unavailable = recovery_result(DiskReconcileStatus::SourceUnavailable);
        state.record_result("game", &mut unavailable);

        state.finish_initial_recovery(
            "game",
            generation,
            InitialRecoveryOutcome::Completed(Box::new(applied)),
        );
        assert_eq!(
            state.initial_recovery_readiness("game"),
            InitialRecoveryReadiness::Ready { generation }
        );
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
    }

    #[test]
    fn failed_watcher_validation_revokes_only_its_own_result() {
        let state = DiskReconcileState::new();
        let generation = state.mark_initial_recovery_pending("game");
        let mut applied = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut applied);
        trust_result(&state, &applied);
        state.finish_initial_recovery(
            "game",
            generation,
            InitialRecoveryOutcome::Completed(Box::new(applied.clone())),
        );
        state.reject_untrusted_reconcile("game", applied.reconcile_revision);
        assert_eq!(
            state.initial_recovery_readiness("game"),
            InitialRecoveryReadiness::Failed { generation }
        );

        let mut newer = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut newer);
        trust_result(&state, &newer);
        state.reject_untrusted_reconcile("game", applied.reconcile_revision);
        assert_eq!(
            state.initial_recovery_readiness("game"),
            InitialRecoveryReadiness::Ready { generation }
        );
    }

    #[test]
    fn overlapping_watcher_handoff_keeps_clean_and_dirty_evidence() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("mods root");
        let state = DiskReconcileState::new();
        let old_session = 1;
        let new_session = 2;
        state.begin_authority_session("game", &root, old_session);
        let mut result = recovery_result(DiskReconcileStatus::Applied);
        state.record_result("game", &mut result);
        assert!(state.mark_authority_reconciled("game", &root, old_session, 0, &result, &[],));

        assert!(state.handoff_authority_session("game", &root, old_session, new_session));
        assert!(matches!(
            state.authority_catch_up("game", &root, new_session),
            AuthorityCatchUp::Clean { reconcile_revision, .. }
                if reconcile_revision == result.reconcile_revision
        ));
        state.end_authority_session("game", old_session, &root);
        assert!(matches!(
            state.authority_catch_up("game", &root, new_session),
            AuthorityCatchUp::Clean { .. }
        ));
        let latest_session = new_session + 1;
        assert!(state.handoff_authority_session("game", &root, new_session, latest_session));
        state.end_authority_session("game", new_session, &root);
        assert!(matches!(
            state.authority_catch_up("game", &root, latest_session),
            AuthorityCatchUp::Clean { .. }
        ));
        let changed = root.join("Changed");
        state.observe_authority_event("game", latest_session, &root, &[changed], false);
        assert!(matches!(
            state.authority_catch_up("game", &root, latest_session),
            AuthorityCatchUp::Scoped { .. }
        ));
        state.observe_authority_event("game", old_session, &root, &[root.join("Late")], false);
        assert!(matches!(
            state.authority_catch_up("game", &root, latest_session),
            AuthorityCatchUp::Full { .. }
        ));
        assert!(!state.handoff_authority_session("game", &root, old_session, latest_session + 1));
        assert!(matches!(
            state.authority_catch_up("game", &root, latest_session),
            AuthorityCatchUp::Full { .. }
        ));
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

    #[test]
    fn newest_activation_intent_supersedes_an_earlier_selection() {
        let state = DiskReconcileState::new();
        let first = state.reserve_activation_intent();
        assert!(state.activation_intent_is_current(first));
        let second = state.reserve_activation_intent();
        assert!(!state.activation_intent_is_current(first));
        assert!(state.activation_intent_is_current(second));
    }

    #[tokio::test]
    async fn concurrent_activations_only_leave_the_latest_generation_authorized() {
        let _test_guard = super::activation_epoch_test_guard_async().await;
        let state = Arc::new(DiskReconcileState::new());
        let (first_started_tx, first_started) = tokio::sync::oneshot::channel();
        let (release_first_tx, release_first) = tokio::sync::oneshot::channel();
        let first_state = Arc::clone(&state);
        let first = tokio::spawn(async move {
            let guard = first_state.activation_guard().await;
            let generation = first_state.begin_activation(Some("game-a".to_string()));
            let authority = first_state
                .activation_authority_for("game-a")
                .expect("first activation should receive authority initially");
            first_started_tx
                .send(generation)
                .expect("first generation receiver should remain available");
            release_first
                .await
                .expect("first activation should be released");
            drop(guard);
            authority
        });
        let first_generation = first_started.await.expect("first activation should start");

        let (second_started_tx, second_started) = tokio::sync::oneshot::channel();
        let (release_second_tx, release_second) = tokio::sync::oneshot::channel();
        let second_state = Arc::clone(&state);
        let second = tokio::spawn(async move {
            let guard = second_state.activation_guard().await;
            let generation = second_state.begin_activation(Some("game-b".to_string()));
            let authority = second_state
                .activation_authority_for("game-b")
                .expect("second activation should receive authority initially");
            second_started_tx
                .send(generation)
                .expect("second generation receiver should remain available");
            release_second
                .await
                .expect("second activation should be released");
            drop(guard);
            authority
        });

        release_first_tx
            .send(())
            .expect("first activation should still be waiting");
        let second_generation = second_started
            .await
            .expect("second activation should start");

        let third_state = Arc::clone(&state);
        let third = tokio::spawn(async move {
            let _guard = third_state.activation_guard().await;
            let generation = third_state.begin_activation(Some("game-c".to_string()));
            let authority = third_state
                .activation_authority_for("game-c")
                .expect("third activation should receive authority initially");
            (generation, authority)
        });

        release_second_tx
            .send(())
            .expect("second activation should still be waiting");
        let (third_generation, third_authority) =
            third.await.expect("third activation task should complete");
        let first_authority = first.await.expect("first activation task should complete");
        let second_authority = second
            .await
            .expect("second activation task should complete");

        assert!(first_generation < second_generation);
        assert!(second_generation < third_generation);
        assert!(!state.activation_is_current(Some("game-a"), first_generation));
        assert!(!state.activation_is_current(Some("game-b"), second_generation));
        assert!(state.activation_is_current(Some("game-c"), third_generation));
        assert!(first_authority.with_current(|| ()).is_none());
        assert!(second_authority.with_current(|| ()).is_none());
        assert_eq!(
            third_authority.with_current(|| third_generation),
            Some(third_generation)
        );
    }

    #[test]
    fn activation_generation_rejects_stale_game_results() {
        let _test_guard = super::activation_epoch_test_guard();
        let state = DiskReconcileState::new();
        let first = state.begin_activation(Some("game-a".to_string()));
        let first_authority = state
            .activation_authority_for("game-a")
            .expect("current game should receive publication authority");
        let second = state.begin_activation(Some("game-b".to_string()));

        assert!(!state.activation_is_current(Some("game-a"), first));
        assert!(state.activation_is_current(Some("game-b"), second));
        assert!(first_authority.with_current(|| ()).is_none());
    }

    #[test]
    fn restoring_activation_after_a_failed_persist_revokes_the_failed_target() {
        let _test_guard = super::activation_epoch_test_guard();
        let state = DiskReconcileState::new();
        state.begin_activation(Some("game-a".to_string()));
        let failed_generation = state.begin_activation(Some("game-b".to_string()));
        state.begin_activation(Some("game-a".to_string()));

        assert!(state
            .activation_authority_for_generation("game-b", failed_generation)
            .is_none());
        assert!(state.activation_authority_for("game-a").is_some());
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

    #[test]
    fn runtime_sync_queue_is_single_flight_and_latest_wins() {
        let state = DiskReconcileState::new();
        let first = state.enqueue_runtime_sync("game", "first", Some(1), None);
        let second = state.enqueue_runtime_sync("game", "second", Some(2), None);

        assert!(first.start_worker);
        assert!(!second.start_worker);
        assert!(second.generation > first.generation);
        let claimed = state
            .claim_latest_runtime_sync("game")
            .expect("latest request should be queued");
        assert_eq!(claimed.generation, second.generation);
        assert_eq!(claimed.cause, "second");
        assert_eq!(claimed.publication_revision, Some(2));
        assert!(state.finish_runtime_sync("game", claimed.generation));
    }

    #[test]
    fn older_runtime_sync_cannot_finish_a_newer_generation() {
        let state = DiskReconcileState::new();
        let first = state.enqueue_runtime_sync("game", "first", Some(1), None);
        let first_job = state
            .claim_latest_runtime_sync("game")
            .expect("first request should be claimed");
        let second = state.enqueue_runtime_sync("game", "second", Some(2), None);

        assert!(!state.runtime_sync_is_current("game", first_job.generation));
        assert!(state.runtime_sync_is_current("game", second.generation));
        assert!(!state.finish_runtime_sync("game", first_job.generation));
        let second_job = state
            .claim_latest_runtime_sync("game")
            .expect("newer request should remain queued");
        assert_eq!(second_job.generation, second.generation);
        assert!(state.finish_runtime_sync("game", second_job.generation));
        assert!(first.generation < second.generation);
    }
}
