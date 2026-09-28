//! Short-lived disk snapshots used by the onboarding indexer.
//!
//! The session watcher starts before the initial scan. A clean session can
//! therefore project the captured discovery directly; any uncertain watcher
//! state falls back to the normal full reconcile path.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{oneshot, Notify};
use uuid::Uuid;

use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::{
    collect_onboarding_disk_discovery_with_progress_and_gate,
    collect_scoped_onboarding_disk_discovery, DiskProjectionError, DiskScopedDiscovery,
    OnboardingDiscoveryPhase, OnboardingDiscoveryProgress,
};
use crate::modules::reconciliation::application::disk_reconcile::types::{
    OnboardingIndexingBackgroundGameStatus, OnboardingIndexingBackgroundPhase,
    OnboardingIndexingBackgroundStatus, OnboardingIndexingSession, OnboardingIndexingSnapshotPhase,
    OnboardingIndexingSnapshotProgress,
};
use crate::modules::settings::application::config::GameConfig;
use crate::shared::errors::AppError;

const SESSION_TTL: Duration = Duration::from_secs(15 * 60);
const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone)]
pub struct OnboardingIndexingSessionStore {
    sessions: Arc<Mutex<HashMap<String, OnboardingSession>>>,
    background_statuses: Arc<Mutex<HashMap<String, OnboardingIndexingBackgroundStatus>>>,
    activation_claims: Arc<Mutex<HashSet<(String, String)>>>,
    activation_claim_released: Arc<Notify>,
    background_status_changed: Arc<Notify>,
}

impl Default for OnboardingIndexingSessionStore {
    fn default() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            background_statuses: Arc::new(Mutex::new(HashMap::new())),
            activation_claims: Arc::new(Mutex::new(HashSet::new())),
            activation_claim_released: Arc::new(Notify::new()),
            background_status_changed: Arc::new(Notify::new()),
        }
    }
}

struct OnboardingSession {
    created_at: Instant,
    games: HashMap<String, PendingGame>,
    background_game_ids: BTreeSet<String>,
    cancelled: Arc<AtomicBool>,
    background_started: bool,
    journal_revision: Option<u64>,
    scan_priority: Arc<ScanPriority>,
}

#[derive(Default)]
struct ScanPriority {
    state: Mutex<ScanPriorityState>,
    wake: Condvar,
}

#[derive(Default)]
struct ScanPriorityState {
    pending: HashSet<String>,
    preferred: Option<String>,
    busy: bool,
    cancelled: bool,
}

struct ScanPermit {
    priority: Arc<ScanPriority>,
}

impl Drop for ScanPermit {
    fn drop(&mut self) {
        let mut state = crate::shared::sync::lock(&self.priority.state);
        state.busy = false;
        self.priority.wake.notify_all();
    }
}

impl ScanPriority {
    fn new(game_ids: impl IntoIterator<Item = String>) -> Arc<Self> {
        let pending = game_ids.into_iter().collect::<HashSet<_>>();
        Arc::new(Self {
            state: Mutex::new(ScanPriorityState {
                pending,
                ..ScanPriorityState::default()
            }),
            wake: Condvar::new(),
        })
    }

    fn prefer(&self, game_id: &str) {
        let mut state = crate::shared::sync::lock(&self.state);
        if state.pending.contains(game_id) {
            state.preferred = Some(game_id.to_string());
            self.wake.notify_all();
        }
    }

    fn finish(&self, game_id: &str) {
        let mut state = crate::shared::sync::lock(&self.state);
        state.pending.remove(game_id);
        if state.preferred.as_deref() == Some(game_id) {
            state.preferred = None;
        }
        self.wake.notify_all();
    }

    fn cancel(&self) {
        let mut state = crate::shared::sync::lock(&self.state);
        state.cancelled = true;
        self.wake.notify_all();
    }

    fn acquire(self: &Arc<Self>, game_id: &str) -> Result<ScanPermit, DiskProjectionError> {
        let mut state = crate::shared::sync::lock(&self.state);
        loop {
            if state.cancelled {
                return Err(DiskProjectionError::Failed(
                    "Onboarding indexing was cancelled".to_string(),
                ));
            }
            if !state.busy
                && state
                    .preferred
                    .as_deref()
                    .is_none_or(|preferred| preferred == game_id)
            {
                state.busy = true;
                return Ok(ScanPermit {
                    priority: Arc::clone(self),
                });
            }
            state = self
                .wake
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}

struct PendingGame {
    prepared: oneshot::Receiver<Result<PreparedGame, AppError>>,
    start: Option<oneshot::Sender<()>>,
}

struct PreparedGame {
    game: GameConfig,
    discovery: DiskScopedDiscovery,
    // This raw watcher deliberately bypasses workspace debounce/filtering so
    // every asset change invalidates the snapshot.
    watcher: RawOnboardingWatcher,
}

struct OnboardingSnapshotProgressReporter {
    session_id: String,
    game_id: String,
    prepared_games: Arc<AtomicU64>,
    total_games: u64,
    started_at: Instant,
    last_emitted_at: Mutex<Option<Duration>>,
    on_progress: Arc<dyn Fn(OnboardingIndexingSnapshotProgress) + Send + Sync>,
}

impl OnboardingSnapshotProgressReporter {
    fn new(
        session_id: String,
        game_id: String,
        prepared_games: Arc<AtomicU64>,
        total_games: u64,
        on_progress: Arc<dyn Fn(OnboardingIndexingSnapshotProgress) + Send + Sync>,
    ) -> Self {
        Self {
            session_id,
            game_id,
            prepared_games,
            total_games,
            started_at: Instant::now(),
            last_emitted_at: Mutex::new(None),
            on_progress,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit(
        &self,
        phase: OnboardingIndexingSnapshotPhase,
        completed_roots: usize,
        total_roots: usize,
        folders_classified: usize,
        current_root: Option<String>,
        force: bool,
    ) {
        let elapsed = self.started_at.elapsed();
        let completed_games = if matches!(phase, OnboardingIndexingSnapshotPhase::Ready) {
            self.prepared_games.fetch_add(1, Ordering::Relaxed) + 1
        } else {
            self.prepared_games.load(Ordering::Relaxed)
        };
        let Ok(mut last_emitted_at) = self.last_emitted_at.lock() else {
            return;
        };
        if !force
            && last_emitted_at
                .is_some_and(|last| elapsed.saturating_sub(last) < PROGRESS_EMIT_INTERVAL)
        {
            return;
        }
        *last_emitted_at = Some(elapsed);

        (self.on_progress)(OnboardingIndexingSnapshotProgress {
            session_id: self.session_id.clone(),
            game_id: self.game_id.clone(),
            phase,
            completed_games,
            total_games: self.total_games,
            completed_roots: u64::try_from(completed_roots).unwrap_or(u64::MAX),
            total_roots: u64::try_from(total_roots).unwrap_or(u64::MAX),
            folders_classified: u64::try_from(folders_classified).unwrap_or(u64::MAX),
            current_root,
            elapsed_ms: elapsed.as_millis().min(u64::MAX as u128) as u64,
        });
    }

    fn report_discovery(&self, progress: OnboardingDiscoveryProgress) {
        let phase = match progress.phase {
            OnboardingDiscoveryPhase::Metadata => OnboardingIndexingSnapshotPhase::Metadata,
            OnboardingDiscoveryPhase::Classifying => OnboardingIndexingSnapshotPhase::Classifying,
        };
        let is_phase_start = progress.completed_roots == 0 && progress.folders_classified == 0;
        let force = is_phase_start || progress.is_terminal;
        self.emit(
            phase,
            progress.completed_roots,
            progress.total_roots,
            progress.folders_classified,
            progress.current_root,
            force,
        );
    }
}

const RAW_WATCH_EVENT_CAPACITY: usize = 8_192;
const RAW_WATCH_SETTLE_TIME: Duration = Duration::from_millis(50);

struct RawOnboardingWatcher {
    _watcher: RecommendedWatcher,
    state: Arc<RawOnboardingWatchState>,
}

#[derive(Default)]
struct RawOnboardingWatchState {
    changed_paths: Mutex<Vec<String>>,
    requires_full_reconcile: AtomicBool,
}

impl RawOnboardingWatcher {
    fn start(path: &Path) -> Result<Self, AppError> {
        let state = Arc::new(RawOnboardingWatchState::default());
        let callback_state = Arc::clone(&state);
        let mut watcher = notify::recommended_watcher(
            move |result: notify::Result<notify::Event>| match result {
                Ok(event) if event.need_rescan() || event.paths.is_empty() => {
                    callback_state
                        .requires_full_reconcile
                        .store(true, Ordering::Release);
                }
                Ok(event) => {
                    let mut changed_paths =
                        crate::shared::sync::lock(&callback_state.changed_paths);
                    if changed_paths.len().saturating_add(event.paths.len())
                        > RAW_WATCH_EVENT_CAPACITY
                    {
                        callback_state
                            .requires_full_reconcile
                            .store(true, Ordering::Release);
                        changed_paths.clear();
                        return;
                    }
                    changed_paths.extend(
                        event
                            .paths
                            .into_iter()
                            .map(|path| path.to_string_lossy().to_string()),
                    );
                }
                Err(_) => {
                    callback_state
                        .requires_full_reconcile
                        .store(true, Ordering::Release);
                }
            },
        )
        .map_err(|error| AppError::Io(format!("Onboarding watcher failed: {error}")))?;
        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|error| AppError::Io(format!("Onboarding watcher failed: {error}")))?;
        Ok(Self {
            _watcher: watcher,
            state,
        })
    }

    fn take_changes(&self) -> Result<Vec<String>, ()> {
        if self
            .state
            .requires_full_reconcile
            .swap(false, Ordering::AcqRel)
        {
            crate::shared::sync::lock(&self.state.changed_paths).clear();
            return Err(());
        }
        Ok(std::mem::take(&mut *crate::shared::sync::lock(
            &self.state.changed_paths,
        )))
    }
}

pub enum ConsumedOnboardingSnapshot {
    Snapshot(Box<OnboardingSnapshotLease>),
    FullFallback,
}

/// Keeps the transient watcher alive while the captured discovery is applied.
/// If it observes another event before commit, the command runs a terminal
/// full reconcile after the snapshot projection.
pub struct OnboardingSnapshotLease {
    prepared: PreparedGame,
    journal_revision: Option<u64>,
}

pub struct ClaimedOnboardingSnapshot {
    pending: PendingGame,
    journal_revision: Option<u64>,
}

pub struct ActivationClaimGuard {
    session_id: String,
    game_id: String,
    claims: Arc<Mutex<HashSet<(String, String)>>>,
    released: Arc<Notify>,
}

impl Drop for ActivationClaimGuard {
    fn drop(&mut self) {
        crate::shared::sync::lock(&self.claims)
            .remove(&(self.session_id.clone(), self.game_id.clone()));
        self.released.notify_waiters();
    }
}

impl ClaimedOnboardingSnapshot {
    pub async fn resolve(self) -> Result<ConsumedOnboardingSnapshot, AppError> {
        let mut prepared = self
            .pending
            .prepared
            .await
            .map_err(|_| AppError::Cancelled)??;
        let changed_paths = match prepared.watcher.take_changes() {
            Ok(paths) => paths,
            Err(()) => return Ok(ConsumedOnboardingSnapshot::FullFallback),
        };
        if paths_contain_unmapped_path(&changed_paths, &prepared.game.mod_path)
            || paths_include_direct_root_path(
                &changed_paths,
                &prepared.game.mod_path,
                &prepared.discovery,
            )
        {
            return Ok(ConsumedOnboardingSnapshot::FullFallback);
        }
        if !changed_paths.is_empty() {
            refresh_changed_roots(&mut prepared, &changed_paths).await?;
        }
        Ok(ConsumedOnboardingSnapshot::Snapshot(Box::new(
            OnboardingSnapshotLease {
                prepared,
                journal_revision: self.journal_revision,
            },
        )))
    }
}

impl OnboardingSnapshotLease {
    pub fn matches_root(&self, root: &Path) -> bool {
        self.prepared.game.mod_path == root
    }

    pub fn journal_revision(&self) -> Option<u64> {
        self.journal_revision
    }

    pub fn journal_allows_game_snapshot(
        &self,
        coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
        game_id: &str,
    ) -> bool {
        let Some(captured) = self.journal_revision else {
            return false;
        };
        let Ok(current) = coordinator.current_journal_revision() else {
            return false;
        };
        if captured == current {
            return true;
        }
        // This watcher observes changes to this root, including committed
        // mutations. An unrelated game's journal entry must not force this
        // complete snapshot to be discarded. An unsettled commit for this
        // game still requires the full recovery path.
        coordinator
            .pending_disk_commits()
            .is_ok_and(|pending| pending.iter().all(|operation| operation.game_id != game_id))
    }

    pub fn discovery(&self) -> DiskScopedDiscovery {
        self.prepared.discovery.clone()
    }

    pub async fn observed_changes_during_apply(&self) -> bool {
        // The raw watcher has no debounce delay. Allow its callback queue one
        // brief turn before dropping the native registration after commit.
        tokio::time::sleep(RAW_WATCH_SETTLE_TIME).await;
        match self.prepared.watcher.take_changes() {
            Err(()) => true,
            Ok(paths) => !paths.is_empty(),
        }
    }
}

impl OnboardingIndexingSessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn background_statuses(&self) -> Vec<OnboardingIndexingBackgroundStatus> {
        self.purge_expired();
        let mut statuses = crate::shared::sync::lock(&self.background_statuses)
            .values()
            .cloned()
            .collect::<Vec<_>>();
        statuses.sort_by(|left, right| left.session_id.cmp(&right.session_id));
        statuses
    }

    pub fn promote_game(&self, game_id: &str) {
        let priorities = crate::shared::sync::lock(&self.sessions)
            .values()
            .map(|session| Arc::clone(&session.scan_priority))
            .collect::<Vec<_>>();
        for priority in priorities {
            priority.prefer(game_id);
        }
    }

    pub fn has_unfinished_game(&self, game_id: &str) -> bool {
        crate::shared::sync::lock(&self.background_statuses)
            .values()
            .flat_map(|status| status.games.iter())
            .any(|game| {
                game.game_id == game_id
                    && matches!(
                        game.phase,
                        OnboardingIndexingBackgroundPhase::Queued
                            | OnboardingIndexingBackgroundPhase::Preparing
                            | OnboardingIndexingBackgroundPhase::Prepared
                            | OnboardingIndexingBackgroundPhase::Applying
                    )
            })
    }

    /// Register persisted onboarding work resumed after an app restart. The
    /// original snapshots no longer exist, so this tracks only the serial
    /// full-reconcile lifecycle exposed to the dashboard.
    pub fn begin_resumed_background(
        &self,
        game_ids: &[String],
    ) -> Result<OnboardingIndexingBackgroundStatus, AppError> {
        if game_ids.is_empty() {
            return Err(AppError::Validation(
                "At least one resumed onboarding game is required".to_string(),
            ));
        }
        let unique_ids = game_ids.iter().collect::<BTreeSet<_>>();
        if unique_ids.len() != game_ids.len() || game_ids.iter().any(|game_id| game_id.is_empty()) {
            return Err(AppError::Validation(
                "Resumed onboarding game IDs must be unique and non-empty".to_string(),
            ));
        }

        let status = OnboardingIndexingBackgroundStatus {
            session_id: Uuid::new_v4().to_string(),
            completed_games: 0,
            total_games: game_ids.len() as u64,
            games: game_ids
                .iter()
                .cloned()
                .map(|game_id| OnboardingIndexingBackgroundGameStatus {
                    game_id,
                    phase: OnboardingIndexingBackgroundPhase::Queued,
                })
                .collect(),
        };
        crate::shared::sync::lock(&self.background_statuses)
            .insert(status.session_id.clone(), status.clone());
        Ok(status)
    }

    pub fn record_snapshot_progress(
        &self,
        progress: &OnboardingIndexingSnapshotProgress,
    ) -> Option<OnboardingIndexingBackgroundStatus> {
        let phase = match progress.phase {
            OnboardingIndexingSnapshotPhase::Metadata
            | OnboardingIndexingSnapshotPhase::Classifying => {
                OnboardingIndexingBackgroundPhase::Preparing
            }
            OnboardingIndexingSnapshotPhase::Ready => OnboardingIndexingBackgroundPhase::Prepared,
            OnboardingIndexingSnapshotPhase::Rechecking => {
                OnboardingIndexingBackgroundPhase::Applying
            }
        };
        self.set_background_phase(&progress.session_id, &progress.game_id, phase)
            .ok()
    }

    pub fn set_background_phase(
        &self,
        session_id: &str,
        game_id: &str,
        phase: OnboardingIndexingBackgroundPhase,
    ) -> Result<OnboardingIndexingBackgroundStatus, AppError> {
        let mut statuses = crate::shared::sync::lock(&self.background_statuses);
        let status = statuses.get_mut(session_id).ok_or_else(|| {
            AppError::NotFound("Onboarding indexing session was not found".to_string())
        })?;
        let game = status
            .games
            .iter_mut()
            .find(|game| game.game_id == game_id)
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "Game '{game_id}' is not available in this onboarding indexing session"
                ))
            })?;
        game.phase = phase;
        status.completed_games = status
            .games
            .iter()
            .filter(|game| matches!(game.phase, OnboardingIndexingBackgroundPhase::Ready))
            .count() as u64;
        let updated = status.clone();
        self.background_status_changed.notify_waiters();
        Ok(updated)
    }

    /// Update the one in-memory onboarding status that owns `game_id`. This
    /// is used when an explicit game activation finishes its recovery outside
    /// the original onboarding worker.
    pub fn set_background_phase_for_game(
        &self,
        game_id: &str,
        phase: OnboardingIndexingBackgroundPhase,
    ) -> Option<OnboardingIndexingBackgroundStatus> {
        let mut statuses = crate::shared::sync::lock(&self.background_statuses);
        let status = statuses
            .values_mut()
            .find(|status| status.games.iter().any(|game| game.game_id == game_id))?;
        let game = status
            .games
            .iter_mut()
            .find(|game| game.game_id == game_id)?;
        game.phase = phase;
        status.completed_games = status
            .games
            .iter()
            .filter(|game| matches!(game.phase, OnboardingIndexingBackgroundPhase::Ready))
            .count() as u64;
        let updated = status.clone();
        self.background_status_changed.notify_waiters();
        Some(updated)
    }

    pub fn mark_background_started(
        &self,
        session_id: &str,
        game_ids: &[String],
    ) -> Result<(), AppError> {
        if game_ids.is_empty() {
            return Err(AppError::Validation(
                "At least one queued onboarding game is required".to_string(),
            ));
        }
        let requested_games = game_ids.iter().collect::<BTreeSet<_>>();
        if requested_games.len() != game_ids.len() {
            return Err(AppError::Validation(
                "Background onboarding game IDs must be unique".to_string(),
            ));
        }

        let mut sessions = crate::shared::sync::lock(&self.sessions);
        let session = sessions.get_mut(session_id).ok_or_else(|| {
            AppError::NotFound("Onboarding indexing session was not found".to_string())
        })?;
        if session.background_started {
            return Err(AppError::Validation(
                "Background onboarding indexing has already started".to_string(),
            ));
        }
        if requested_games != session.background_game_ids.iter().collect::<BTreeSet<_>>() {
            return Err(AppError::Validation(
                "Background onboarding games must match the original background games".to_string(),
            ));
        }
        session.background_started = true;
        for game in session.games.values_mut() {
            if let Some(start) = game.start.take() {
                let _ = start.send(());
            }
        }
        if session.games.is_empty() {
            sessions.remove(session_id);
        }
        Ok(())
    }

    pub async fn begin(
        &self,
        games: Vec<GameConfig>,
    ) -> Result<OnboardingIndexingSession, AppError> {
        self.begin_with_progress(games, |_| {}).await
    }

    pub async fn begin_with_progress<F>(
        &self,
        games: Vec<GameConfig>,
        on_progress: F,
    ) -> Result<OnboardingIndexingSession, AppError>
    where
        F: Fn(OnboardingIndexingSnapshotProgress) + Send + Sync + 'static,
    {
        self.begin_with_progress_at_revision(games, None, on_progress)
            .await
    }

    pub async fn begin_with_progress_at_revision<F>(
        &self,
        games: Vec<GameConfig>,
        journal_revision: Option<u64>,
        on_progress: F,
    ) -> Result<OnboardingIndexingSession, AppError>
    where
        F: Fn(OnboardingIndexingSnapshotProgress) + Send + Sync + 'static,
    {
        if games.is_empty() {
            return Err(AppError::Validation(
                "At least one game is required for onboarding indexing".to_string(),
            ));
        }
        let unique_ids = games
            .iter()
            .map(|game| game.id.as_str())
            .collect::<BTreeSet<_>>();
        if unique_ids.len() != games.len() {
            return Err(AppError::Validation(
                "Onboarding indexing game IDs must be unique".to_string(),
            ));
        }

        let session_id = Uuid::new_v4().to_string();
        let total_games = games.len() as u64;
        let background_status = OnboardingIndexingBackgroundStatus {
            session_id: session_id.clone(),
            completed_games: 0,
            total_games,
            games: games
                .iter()
                .map(|game| OnboardingIndexingBackgroundGameStatus {
                    game_id: game.id.clone(),
                    phase: OnboardingIndexingBackgroundPhase::Queued,
                })
                .collect(),
        };
        let on_progress: Arc<dyn Fn(OnboardingIndexingSnapshotProgress) + Send + Sync> =
            Arc::new(on_progress);
        let cancelled = Arc::new(AtomicBool::new(false));
        let scan_priority = ScanPriority::new(games.iter().map(|game| game.id.clone()));
        scan_priority.prefer(&games[0].id);
        let background_game_ids = games.iter().skip(1).map(|game| game.id.clone()).collect();
        let mut pending_games = HashMap::with_capacity(games.len());
        let mut queued_games = Vec::with_capacity(games.len());
        for (index, game) in games.into_iter().enumerate() {
            let (sender, receiver) = oneshot::channel();
            let (start, wait_for_start) = if index == 0 {
                (None, None)
            } else {
                let (start, wait_for_start) = oneshot::channel();
                (Some(start), Some(wait_for_start))
            };
            pending_games.insert(
                game.id.clone(),
                PendingGame {
                    prepared: receiver,
                    start,
                },
            );
            queued_games.push((game, sender, wait_for_start));
        }
        self.purge_expired();
        let mut sessions = crate::shared::sync::lock(&self.sessions);
        sessions.insert(
            session_id.clone(),
            OnboardingSession {
                created_at: Instant::now(),
                games: pending_games,
                background_game_ids,
                cancelled: Arc::clone(&cancelled),
                background_started: false,
                journal_revision,
                scan_priority: Arc::clone(&scan_priority),
            },
        );
        drop(sessions);
        crate::shared::sync::lock(&self.background_statuses)
            .insert(session_id.clone(), background_status);
        self.schedule_expiration(session_id.clone(), SESSION_TTL, Arc::clone(&cancelled));

        let prepared_games = Arc::new(AtomicU64::new(0));
        let worker_session_id = session_id.clone();
        for (game, sender, wait_for_start) in queued_games {
            let cancelled = Arc::clone(&cancelled);
            let scan_priority = Arc::clone(&scan_priority);
            let on_progress = Arc::clone(&on_progress);
            let prepared_games = Arc::clone(&prepared_games);
            let worker_session_id = worker_session_id.clone();
            tokio::spawn(async move {
                if let Some(wait_for_start) = wait_for_start {
                    let _ = wait_for_start.await;
                }
                let progress_reporter = Arc::new(OnboardingSnapshotProgressReporter::new(
                    worker_session_id,
                    game.id.clone(),
                    prepared_games,
                    total_games,
                    on_progress,
                ));
                let game_id = game.id.clone();
                let preparation = if cancelled.load(Ordering::Acquire) {
                    Err(AppError::Cancelled)
                } else {
                    prepare_game(
                        game,
                        Arc::clone(&progress_reporter),
                        Arc::clone(&scan_priority),
                    )
                    .await
                };
                scan_priority.finish(&game_id);
                match preparation {
                    Ok(prepared) => {
                        progress_reporter.emit(
                            OnboardingIndexingSnapshotPhase::Ready,
                            prepared.discovery.census.top_level_roots,
                            prepared.discovery.census.top_level_roots,
                            prepared.discovery.scan_counts.classified_directories,
                            None,
                            true,
                        );
                        let _ = sender.send(Ok(prepared));
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error));
                    }
                }
            });
        }

        Ok(OnboardingIndexingSession { session_id })
    }

    /// Removes a game from its session before reconciliation. This makes each
    /// snapshot single-use and guarantees its native watcher is stopped on all
    /// terminal paths.
    pub async fn consume(
        &self,
        session_id: &str,
        game_id: &str,
    ) -> Result<ConsumedOnboardingSnapshot, AppError> {
        self.claim(session_id, game_id, false)?.resolve().await
    }

    pub fn claim_for_activation(
        &self,
        game_id: &str,
    ) -> Option<(ClaimedOnboardingSnapshot, ActivationClaimGuard)> {
        let session_id = crate::shared::sync::lock(&self.sessions)
            .iter()
            .find(|(_, session)| session.games.contains_key(game_id))
            .map(|(session_id, _)| session_id.clone())?;
        let claimed = self.claim(&session_id, game_id, true).ok()?;
        let guard = ActivationClaimGuard {
            session_id,
            game_id: game_id.to_string(),
            claims: Arc::clone(&self.activation_claims),
            released: Arc::clone(&self.activation_claim_released),
        };
        Some((claimed, guard))
    }

    pub fn is_claimed_by_activation(&self, session_id: &str, game_id: &str) -> bool {
        crate::shared::sync::lock(&self.activation_claims)
            .contains(&(session_id.to_string(), game_id.to_string()))
    }

    pub async fn wait_for_activation_claim(&self, session_id: &str, game_id: &str) {
        loop {
            let released = self.activation_claim_released.notified();
            tokio::pin!(released);
            released.as_mut().enable();
            if !self.is_claimed_by_activation(session_id, game_id) {
                return;
            }
            released.await;
        }
    }

    pub fn background_game_is_ready(&self, session_id: &str, game_id: &str) -> bool {
        crate::shared::sync::lock(&self.background_statuses)
            .get(session_id)
            .is_some_and(|status| {
                status.games.iter().any(|game| {
                    game.game_id == game_id
                        && game.phase == OnboardingIndexingBackgroundPhase::Ready
                })
            })
    }

    /// Returns a terminal phase only when the background worker already owns
    /// this game's prepared job. A queued job still owned by the session is
    /// instead claimed directly by activation.
    pub async fn wait_for_background_claimed_game(
        &self,
        game_id: &str,
    ) -> Option<OnboardingIndexingBackgroundPhase> {
        loop {
            let changed = self.background_status_changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let phase = {
                let statuses = crate::shared::sync::lock(&self.background_statuses);
                statuses
                    .values()
                    .flat_map(|status| &status.games)
                    .find(|game| game.game_id == game_id)
                    .map(|game| game.phase.clone())
            }?;
            match phase {
                OnboardingIndexingBackgroundPhase::Ready
                | OnboardingIndexingBackgroundPhase::NeedsAttention
                | OnboardingIndexingBackgroundPhase::Failed => return Some(phase),
                _ => changed.await,
            }
        }
    }

    fn claim(
        &self,
        session_id: &str,
        game_id: &str,
        activation: bool,
    ) -> Result<ClaimedOnboardingSnapshot, AppError> {
        let (pending, journal_revision) = {
            let mut sessions = crate::shared::sync::lock(&self.sessions);
            let expired = sessions.get(session_id).is_some_and(|session| {
                !session.background_started && session.created_at.elapsed() >= SESSION_TTL
            });
            if expired {
                if let Some(session) = sessions.remove(session_id) {
                    session.cancelled.store(true, Ordering::Release);
                    session.scan_priority.cancel();
                }
                crate::shared::sync::lock(&self.background_statuses).remove(session_id);
                return Err(AppError::Cancelled);
            }
            let session = sessions.get_mut(session_id).ok_or_else(|| {
                if self.is_claimed_by_activation(session_id, game_id) {
                    AppError::Cancelled
                } else {
                    AppError::NotFound("Onboarding indexing session was not found".to_string())
                }
            })?;
            let mut pending = session.games.remove(game_id).ok_or_else(|| {
                if self.is_claimed_by_activation(session_id, game_id) {
                    AppError::Cancelled
                } else {
                    AppError::NotFound(format!(
                        "Game '{game_id}' is not available in this onboarding indexing session"
                    ))
                }
            })?;
            if activation {
                crate::shared::sync::lock(&self.activation_claims)
                    .insert((session_id.to_string(), game_id.to_string()));
            }
            if let Some(start) = pending.start.take() {
                let _ = start.send(());
            }
            let journal_revision = session.journal_revision;
            if session.games.is_empty() && session.background_started {
                sessions.remove(session_id);
            }
            (pending, journal_revision)
        };
        Ok(ClaimedOnboardingSnapshot {
            pending,
            journal_revision,
        })
    }

    pub fn cancel(&self, session_id: &str) -> Result<(), AppError> {
        if let Some(session) = crate::shared::sync::lock(&self.sessions).remove(session_id) {
            session.cancelled.store(true, Ordering::Release);
            session.scan_priority.cancel();
        }
        crate::shared::sync::lock(&self.background_statuses).remove(session_id);
        self.background_status_changed.notify_waiters();
        Ok(())
    }

    fn purge_expired(&self) {
        let mut expired_session_ids = Vec::new();
        crate::shared::sync::lock(&self.sessions).retain(|session_id, session| {
            let expired =
                !session.background_started && session.created_at.elapsed() >= SESSION_TTL;
            if expired {
                session.cancelled.store(true, Ordering::Release);
                session.scan_priority.cancel();
                expired_session_ids.push(session_id.clone());
            }
            !expired
        });
        if !expired_session_ids.is_empty() {
            let mut statuses = crate::shared::sync::lock(&self.background_statuses);
            for session_id in expired_session_ids {
                statuses.remove(&session_id);
            }
        }
    }

    fn schedule_expiration(&self, session_id: String, delay: Duration, cancelled: Arc<AtomicBool>) {
        let sessions = Arc::clone(&self.sessions);
        let background_statuses = Arc::clone(&self.background_statuses);
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let expired = {
                let mut sessions = crate::shared::sync::lock(&sessions);
                let expired = sessions.get(&session_id).is_some_and(|session| {
                    !session.background_started && session.created_at.elapsed() >= delay
                });
                if expired {
                    if let Some(session) = sessions.remove(&session_id) {
                        session.scan_priority.cancel();
                    }
                }
                expired
            };
            if expired {
                cancelled.store(true, Ordering::Release);
                crate::shared::sync::lock(&background_statuses).remove(&session_id);
            }
        });
    }
}

async fn prepare_game(
    game: GameConfig,
    progress_reporter: Arc<OnboardingSnapshotProgressReporter>,
    scan_priority: Arc<ScanPriority>,
) -> Result<PreparedGame, AppError> {
    let watcher = RawOnboardingWatcher::start(&game.mod_path)?;
    let snapshot_game = game.clone();
    let discovery = tokio::task::spawn_blocking(move || {
        let on_progress = |progress| progress_reporter.report_discovery(progress);
        let gate = || {
            scan_priority
                .acquire(&snapshot_game.id)
                .map(|permit| Box::new(permit) as Box<dyn Send>)
        };
        let onboarding = collect_onboarding_disk_discovery_with_progress_and_gate(
            &snapshot_game.mod_path,
            Some(&on_progress),
            Some(&gate),
        )
        .map_err(snapshot_error)?;
        Ok::<_, AppError>(onboarding.discovery)
    })
    .await??;

    Ok(PreparedGame {
        game,
        discovery,
        watcher,
    })
}

async fn refresh_changed_roots(
    prepared: &mut PreparedGame,
    changed_paths: &[String],
) -> Result<(), AppError> {
    let changed_roots = crate::modules::reconciliation::application::disk_reconcile::path_classifier::collect_changed_roots(
        &prepared.game.mod_path,
        changed_paths,
    );
    if changed_roots.is_empty() {
        return Err(AppError::Internal(
            "Onboarding watcher events did not identify a changed root".to_string(),
        ));
    }
    let mods_path = prepared.game.mod_path.clone();
    let roots_for_scan = changed_roots.clone();
    let refreshed = tokio::task::spawn_blocking(move || {
        let onboarding = collect_scoped_onboarding_disk_discovery(&mods_path, &roots_for_scan)
            .map_err(snapshot_error)?;
        Ok::<_, AppError>(onboarding.discovery)
    })
    .await??;

    // A cross-root identity collision correctly promotes the discovery to a
    // full scan. Otherwise compose the changed roots into the original full
    // projection so the writer can retain full-scan prune semantics.
    if refreshed.scoped {
        let root_keys = changed_roots
            .iter()
            .map(|root| crate::shared::path_key::folder_path_key(root, None))
            .collect::<std::collections::HashSet<_>>();
        prepared
            .discovery
            .projection
            .objects
            .retain(|entry| !root_keys.contains(&entry.folder_path_key));
        prepared
            .discovery
            .projection
            .mods
            .retain(|entry| !root_keys.contains(&entry.object_folder_path_key));
        prepared
            .discovery
            .projection
            .objects
            .extend(refreshed.projection.objects);
        prepared
            .discovery
            .projection
            .mods
            .extend(refreshed.projection.mods);
        prepared.discovery.census = refreshed.census;
        prepared.discovery.scoped = false;
    } else {
        // Scoped discovery already performed a complete scan when cross-root
        // ambiguity was found. Reuse it rather than walking the disk again.
        prepared.discovery = refreshed;
    }

    Ok(())
}

fn paths_contain_unmapped_path(paths: &[String], mods_path: &Path) -> bool {
    paths
        .iter()
        .any(|path| !Path::new(path).starts_with(mods_path))
}

fn paths_include_direct_root_path(
    paths: &[String],
    mods_path: &Path,
    discovery: &DiskScopedDiscovery,
) -> bool {
    paths.iter().any(|path| {
        let path = Path::new(path);
        let Ok(relative) = path.strip_prefix(mods_path) else {
            return false;
        };
        if relative.components().count() != 1 {
            return false;
        }
        if path.is_dir() {
            return false;
        }
        let root_name = relative.to_string_lossy();
        !discovery
            .census
            .entries
            .iter()
            .any(|entry| entry.folder_path.eq_ignore_ascii_case(&root_name))
    })
}

fn snapshot_error(error: DiskProjectionError) -> AppError {
    match error {
        DiskProjectionError::SourceUnavailable(message) => AppError::Io(message),
        DiskProjectionError::Failed(message) => AppError::Internal(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_config(id: &str, mod_path: std::path::PathBuf) -> GameConfig {
        GameConfig {
            id: id.to_string(),
            name: id.to_string(),
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            instance_path: mod_path.clone(),
            mod_path,
            ready_to_move_path: None,
            launch_mode: Default::default(),
            game_exe: None,
            loader_exe: None,
            xxmi_launcher_exe: None,
            launch_args: None,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn preparation_progress_reports_phase_transitions_and_throttles_heartbeats() {
        let reports = Arc::new(Mutex::new(Vec::new()));
        let captured_reports = Arc::clone(&reports);
        let reporter = OnboardingSnapshotProgressReporter::new(
            "session".to_string(),
            "game".to_string(),
            Arc::new(AtomicU64::new(0)),
            1,
            Arc::new(move |progress| {
                crate::shared::sync::lock(&captured_reports).push(progress);
            }),
        );

        reporter.report_discovery(OnboardingDiscoveryProgress {
            phase: OnboardingDiscoveryPhase::Metadata,
            completed_roots: 0,
            total_roots: 2,
            folders_classified: 0,
            current_root: None,
            is_terminal: false,
        });
        reporter.report_discovery(OnboardingDiscoveryProgress {
            phase: OnboardingDiscoveryPhase::Metadata,
            completed_roots: 1,
            total_roots: 2,
            folders_classified: 0,
            current_root: Some("Alice".to_string()),
            is_terminal: false,
        });
        reporter.report_discovery(OnboardingDiscoveryProgress {
            phase: OnboardingDiscoveryPhase::Classifying,
            completed_roots: 0,
            total_roots: 2,
            folders_classified: 0,
            current_root: None,
            is_terminal: false,
        });
        reporter.emit(OnboardingIndexingSnapshotPhase::Ready, 2, 2, 4, None, true);

        let reports = crate::shared::sync::lock(&reports);
        assert_eq!(reports.len(), 3);
        assert_eq!(
            reports
                .iter()
                .map(|report| report.phase.clone())
                .collect::<Vec<_>>(),
            vec![
                OnboardingIndexingSnapshotPhase::Metadata,
                OnboardingIndexingSnapshotPhase::Classifying,
                OnboardingIndexingSnapshotPhase::Ready,
            ]
        );
        assert_eq!(reports[1].completed_roots, 0);
        assert_eq!(reports[2].completed_games, 1);
    }

    #[test]
    fn background_status_only_counts_games_after_their_projection_is_applied() {
        let store = OnboardingIndexingSessionStore::new();
        crate::shared::sync::lock(&store.background_statuses).insert(
            "session".to_string(),
            OnboardingIndexingBackgroundStatus {
                session_id: "session".to_string(),
                completed_games: 0,
                total_games: 2,
                games: vec![
                    OnboardingIndexingBackgroundGameStatus {
                        game_id: "first".to_string(),
                        phase: OnboardingIndexingBackgroundPhase::Queued,
                    },
                    OnboardingIndexingBackgroundGameStatus {
                        game_id: "second".to_string(),
                        phase: OnboardingIndexingBackgroundPhase::Queued,
                    },
                ],
            },
        );

        let prepared = store
            .record_snapshot_progress(&OnboardingIndexingSnapshotProgress {
                session_id: "session".to_string(),
                game_id: "first".to_string(),
                phase: OnboardingIndexingSnapshotPhase::Ready,
                completed_games: 1,
                total_games: 2,
                completed_roots: 1,
                total_roots: 1,
                folders_classified: 1,
                current_root: None,
                elapsed_ms: 1,
            })
            .expect("prepared progress should update the background status");
        assert_eq!(prepared.completed_games, 0);
        assert_eq!(
            prepared.games[0].phase,
            OnboardingIndexingBackgroundPhase::Prepared
        );

        let applying = store
            .set_background_phase(
                "session",
                "first",
                OnboardingIndexingBackgroundPhase::Applying,
            )
            .expect("applying phase should update the background status");
        assert_eq!(applying.completed_games, 0);

        let ready = store
            .set_background_phase("session", "first", OnboardingIndexingBackgroundPhase::Ready)
            .expect("ready phase should update the background status");
        assert_eq!(ready.completed_games, 1);
        assert_eq!(
            ready.games[1].phase,
            OnboardingIndexingBackgroundPhase::Queued
        );
    }

    #[tokio::test]
    async fn activation_join_waits_for_the_background_projection() {
        let store = OnboardingIndexingSessionStore::new();
        crate::shared::sync::lock(&store.background_statuses).insert(
            "session".to_string(),
            OnboardingIndexingBackgroundStatus {
                session_id: "session".to_string(),
                completed_games: 0,
                total_games: 1,
                games: vec![OnboardingIndexingBackgroundGameStatus {
                    game_id: "selected".to_string(),
                    phase: OnboardingIndexingBackgroundPhase::Applying,
                }],
            },
        );
        let waiting = {
            let store = store.clone();
            tokio::spawn(async move { store.wait_for_background_claimed_game("selected").await })
        };
        let second_waiting = {
            let store = store.clone();
            tokio::spawn(async move { store.wait_for_background_claimed_game("selected").await })
        };
        tokio::task::yield_now().await;
        assert!(!waiting.is_finished());
        assert!(!second_waiting.is_finished());
        store
            .set_background_phase(
                "session",
                "selected",
                OnboardingIndexingBackgroundPhase::Ready,
            )
            .expect("background projection should finish");
        let phase = tokio::time::timeout(Duration::from_secs(1), waiting)
            .await
            .expect("activation join should wake")
            .expect("join task should succeed");
        assert_eq!(phase, Some(OnboardingIndexingBackgroundPhase::Ready));
        let second_phase = tokio::time::timeout(Duration::from_secs(1), second_waiting)
            .await
            .expect("all activation joins should wake")
            .expect("second join task should succeed");
        assert_eq!(second_phase, Some(OnboardingIndexingBackgroundPhase::Ready));
    }

    #[test]
    fn unmapped_raw_watcher_paths_force_the_safe_fallback() {
        assert!(paths_contain_unmapped_path(
            &["E:/other/Mod.buf".to_string()],
            Path::new("E:/mods"),
        ));
    }

    #[test]
    fn deep_asset_paths_are_accepted_for_scoped_rescan() {
        assert!(!paths_contain_unmapped_path(
            &["E:/mods/Alice/Blue/mesh.buf".to_string()],
            Path::new("E:/mods"),
        ));
    }

    #[tokio::test]
    async fn raw_watcher_captures_deep_asset_changes_without_debounce() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let asset = temp.path().join("Alice").join("Blue").join("mesh.buf");
        std::fs::create_dir_all(asset.parent().expect("asset parent"))
            .expect("asset directory should be created");
        let watcher = RawOnboardingWatcher::start(temp.path()).expect("watcher should start");
        std::fs::write(&asset, b"asset").expect("asset should be written");

        let observed = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match watcher.take_changes() {
                    Ok(paths) if !paths.is_empty() => return paths,
                    Ok(_) => tokio::time::sleep(Duration::from_millis(10)).await,
                    Err(()) => panic!("raw watcher requested unexpected full fallback"),
                }
            }
        })
        .await
        .expect("raw watcher should receive asset change");
        assert!(observed.iter().any(|path| path.ends_with("mesh.buf")));
    }

    #[tokio::test]
    async fn session_streams_the_first_game_without_waiting_for_a_later_failure() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let first_mod = temp.path().join("first").join("Alice").join("Blue");
        std::fs::create_dir_all(&first_mod).expect("first mod folder should be created");
        std::fs::write(
            first_mod.join("mod.ini"),
            "[TextureOverrideTest]\nhash = abc\n",
        )
        .expect("first mod ini should be written");

        let store = OnboardingIndexingSessionStore::new();
        let session = store
            .begin(vec![
                game_config("first", temp.path().join("first")),
                game_config("later", temp.path().join("missing")),
            ])
            .await
            .expect("session should start before later preparation runs");

        assert_eq!(
            crate::shared::sync::lock(&store.sessions)
                .get(&session.session_id)
                .expect("session should be registered")
                .games
                .len(),
            2
        );
        let first = tokio::time::timeout(
            Duration::from_secs(3),
            store.consume(&session.session_id, "first"),
        )
        .await
        .expect("first game should not wait for the later game")
        .expect("first game should prepare successfully");
        assert!(matches!(first, ConsumedOnboardingSnapshot::Snapshot(_)));

        let later = tokio::time::timeout(
            Duration::from_secs(3),
            store.consume(&session.session_id, "later"),
        )
        .await
        .expect("later game should resolve its own preparation");
        assert!(matches!(later, Err(AppError::Io(_))));
    }

    #[tokio::test]
    async fn background_snapshot_starts_only_after_handoff() {
        let temp = tempfile::tempdir().expect("tempdir");
        let first = temp.path().join("first");
        let later = temp.path().join("later");
        std::fs::create_dir(&first).expect("first root");
        std::fs::create_dir(&later).expect("later root");
        let store = OnboardingIndexingSessionStore::new();
        let session = store
            .begin(vec![
                game_config("first", first),
                game_config("later", later),
            ])
            .await
            .expect("session");

        store
            .consume(&session.session_id, "first")
            .await
            .expect("first snapshot");
        assert!(crate::shared::sync::lock(&store.sessions)
            .get(&session.session_id)
            .and_then(|session| session.games.get("later"))
            .is_some_and(|game| game.start.is_some()));

        store
            .mark_background_started(&session.session_id, &["later".to_string()])
            .expect("background handoff");
        assert!(matches!(
            store.consume(&session.session_id, "later").await,
            Ok(ConsumedOnboardingSnapshot::Snapshot(_))
        ));
    }

    #[tokio::test]
    async fn background_handoff_accepts_a_game_claimed_by_activation() {
        let temp = tempfile::tempdir().expect("tempdir");
        let first = temp.path().join("first");
        let selected = temp.path().join("selected");
        let later = temp.path().join("later");
        for root in [&first, &selected, &later] {
            std::fs::create_dir(root).expect("mods root");
        }
        let store = OnboardingIndexingSessionStore::new();
        let session = store
            .begin(vec![
                game_config("first", first),
                game_config("selected", selected),
                game_config("later", later),
            ])
            .await
            .expect("session");
        store
            .consume(&session.session_id, "first")
            .await
            .expect("first snapshot");
        let (claimed, claim_guard) = store
            .claim_for_activation("selected")
            .expect("activation claim");

        assert!(matches!(
            store.consume(&session.session_id, "selected").await,
            Err(AppError::Cancelled)
        ));
        store
            .mark_background_started(
                &session.session_id,
                &["selected".to_string(), "later".to_string()],
            )
            .expect("handoff retains activation-owned game");
        assert!(matches!(
            store.consume(&session.session_id, "later").await,
            Ok(ConsumedOnboardingSnapshot::Snapshot(_))
        ));
        assert!(matches!(
            claimed.resolve().await,
            Ok(ConsumedOnboardingSnapshot::Snapshot(_))
        ));
        drop(claim_guard);
    }

    #[test]
    fn selected_game_takes_the_next_scan_batch() {
        let priority = ScanPriority::new(["first".to_string(), "second".to_string()]);
        priority.prefer("first");
        let first_batch = priority.acquire("first").expect("first batch");
        let second_priority = Arc::clone(&priority);
        let (sent, received) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _second_batch = second_priority.acquire("second").expect("second batch");
            sent.send(()).expect("signal selected game");
        });

        priority.prefer("second");
        drop(first_batch);
        received
            .recv_timeout(Duration::from_secs(3))
            .expect("selected game should take the next batch");
        worker.join().expect("scan worker should finish");
    }

    #[tokio::test]
    async fn activation_claims_the_prepared_job_instead_of_preparing_again() {
        let temp = tempfile::tempdir().expect("tempdir");
        let first = temp.path().join("first");
        let selected = temp.path().join("selected");
        std::fs::create_dir(&first).expect("first root");
        std::fs::create_dir(&selected).expect("selected root");
        let store = OnboardingIndexingSessionStore::new();
        let session = store
            .begin(vec![
                game_config("first", first),
                game_config("selected", selected),
            ])
            .await
            .expect("session");

        store.promote_game("selected");
        let (claimed, claim_guard) = store
            .claim_for_activation("selected")
            .expect("activation should own the pending scan");
        assert!(store.is_claimed_by_activation(&session.session_id, "selected"));
        assert!(matches!(
            store.consume(&session.session_id, "selected").await,
            Err(AppError::Cancelled)
        ));
        let resolved = tokio::time::timeout(Duration::from_secs(3), claimed.resolve())
            .await
            .expect("selected preparation should finish")
            .expect("selected snapshot");
        assert!(matches!(resolved, ConsumedOnboardingSnapshot::Snapshot(_)));
        drop(claim_guard);
        assert!(!store.is_claimed_by_activation(&session.session_id, "selected"));
    }

    #[tokio::test]
    async fn all_background_waiters_resume_when_activation_claim_is_released() {
        let temp = tempfile::tempdir().expect("tempdir");
        let selected = temp.path().join("selected");
        std::fs::create_dir(&selected).expect("selected root");
        let store = OnboardingIndexingSessionStore::new();
        let session = store
            .begin(vec![game_config("selected", selected)])
            .await
            .expect("session");
        let (_, claim_guard) = store
            .claim_for_activation("selected")
            .expect("activation claim");
        let first = {
            let store = store.clone();
            let session_id = session.session_id.clone();
            tokio::spawn(async move {
                store
                    .wait_for_activation_claim(&session_id, "selected")
                    .await;
            })
        };
        let second = {
            let store = store.clone();
            let session_id = session.session_id.clone();
            tokio::spawn(async move {
                store
                    .wait_for_activation_claim(&session_id, "selected")
                    .await;
            })
        };
        tokio::task::yield_now().await;
        assert!(!first.is_finished());
        assert!(!second.is_finished());
        drop(claim_guard);
        tokio::time::timeout(Duration::from_secs(1), first)
            .await
            .expect("first waiter should wake")
            .expect("first task should succeed");
        tokio::time::timeout(Duration::from_secs(1), second)
            .await
            .expect("second waiter should wake")
            .expect("second task should succeed");
    }

    #[tokio::test]
    async fn session_carries_the_revision_captured_before_indexing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        std::fs::create_dir(&mods).expect("mods root");
        let store = OnboardingIndexingSessionStore::new();
        let session = store
            .begin_with_progress_at_revision(vec![game_config("game", mods)], Some(7), |_| {})
            .await
            .expect("session");

        let consumed = store
            .consume(&session.session_id, "game")
            .await
            .expect("snapshot");
        match consumed {
            ConsumedOnboardingSnapshot::Snapshot(lease) => {
                assert_eq!(lease.journal_revision(), Some(7));
            }
            ConsumedOnboardingSnapshot::FullFallback => {
                panic!("unchanged root should keep snapshot")
            }
        }
    }

    #[tokio::test]
    async fn scheduled_expiration_removes_orphaned_session() {
        let store = OnboardingIndexingSessionStore::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        crate::shared::sync::lock(&store.sessions).insert(
            "expired".to_string(),
            OnboardingSession {
                created_at: Instant::now(),
                games: HashMap::new(),
                background_game_ids: BTreeSet::new(),
                cancelled: Arc::clone(&cancelled),
                background_started: false,
                journal_revision: None,
                scan_priority: ScanPriority::new(Vec::new()),
            },
        );
        store.schedule_expiration(
            "expired".to_string(),
            Duration::from_millis(1),
            Arc::clone(&cancelled),
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!crate::shared::sync::lock(&store.sessions).contains_key("expired"));
        assert!(cancelled.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn scheduled_expiration_keeps_a_started_background_session_alive() {
        let store = OnboardingIndexingSessionStore::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        crate::shared::sync::lock(&store.sessions).insert(
            "background".to_string(),
            OnboardingSession {
                created_at: Instant::now(),
                games: HashMap::new(),
                background_game_ids: BTreeSet::new(),
                cancelled: Arc::clone(&cancelled),
                background_started: true,
                journal_revision: None,
                scan_priority: ScanPriority::new(Vec::new()),
            },
        );
        crate::shared::sync::lock(&store.background_statuses).insert(
            "background".to_string(),
            OnboardingIndexingBackgroundStatus {
                session_id: "background".to_string(),
                completed_games: 0,
                total_games: 1,
                games: vec![OnboardingIndexingBackgroundGameStatus {
                    game_id: "later-game".to_string(),
                    phase: OnboardingIndexingBackgroundPhase::Preparing,
                }],
            },
        );

        store.schedule_expiration(
            "background".to_string(),
            Duration::from_millis(1),
            Arc::clone(&cancelled),
        );
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert!(crate::shared::sync::lock(&store.sessions).contains_key("background"));
        assert!(crate::shared::sync::lock(&store.background_statuses).contains_key("background"));
        assert!(!cancelled.load(Ordering::Acquire));
    }
}
