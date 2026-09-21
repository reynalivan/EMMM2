//! Short-lived disk snapshots used by the onboarding indexer.
//!
//! The session watcher starts before the initial scan. A clean session can
//! therefore project the captured discovery directly; any uncertain watcher
//! state falls back to the normal full reconcile path.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::{
    collect_onboarding_disk_discovery, collect_onboarding_disk_discovery_with_progress,
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
}

impl Default for OnboardingIndexingSessionStore {
    fn default() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            background_statuses: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

struct OnboardingSession {
    created_at: Instant,
    games: HashMap<String, PendingGame>,
    cancelled: Arc<AtomicBool>,
    background_started: bool,
}

struct PendingGame {
    prepared: oneshot::Receiver<Result<PreparedGame, AppError>>,
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
        let is_phase_start = matches!(progress.phase, OnboardingDiscoveryPhase::Metadata)
            && progress.completed_roots == 0
            && progress.folders_classified == 0
            || matches!(progress.phase, OnboardingDiscoveryPhase::Classifying)
                && progress.completed_roots == 0
                && progress.folders_classified == 0;
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
}

impl OnboardingSnapshotLease {
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
        Ok(status.clone())
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
        Some(status.clone())
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
        let pending_games = session.games.keys().collect::<BTreeSet<_>>();
        if requested_games != pending_games {
            return Err(AppError::Validation(
                "Background onboarding games must match the remaining session games".to_string(),
            ));
        }
        session.background_started = true;
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
        let mut pending_games = HashMap::with_capacity(games.len());
        let mut queued_games = Vec::with_capacity(games.len());
        for game in games {
            let (sender, receiver) = oneshot::channel();
            pending_games.insert(game.id.clone(), PendingGame { prepared: receiver });
            queued_games.push((game, sender));
        }
        self.purge_expired();
        let mut sessions = crate::shared::sync::lock(&self.sessions);
        sessions.insert(
            session_id.clone(),
            OnboardingSession {
                created_at: Instant::now(),
                games: pending_games,
                cancelled: Arc::clone(&cancelled),
                background_started: false,
            },
        );
        drop(sessions);
        crate::shared::sync::lock(&self.background_statuses)
            .insert(session_id.clone(), background_status);
        self.schedule_expiration(session_id.clone(), SESSION_TTL, Arc::clone(&cancelled));

        let prepared_games = Arc::new(AtomicU64::new(0));
        let worker_session_id = session_id.clone();
        tokio::spawn(async move {
            for (game, sender) in queued_games {
                if cancelled.load(Ordering::Acquire) {
                    let _ = sender.send(Err(AppError::Cancelled));
                    continue;
                }
                let progress_reporter = Arc::new(OnboardingSnapshotProgressReporter::new(
                    worker_session_id.clone(),
                    game.id.clone(),
                    Arc::clone(&prepared_games),
                    total_games,
                    Arc::clone(&on_progress),
                ));
                match prepare_game(game, Arc::clone(&progress_reporter)).await {
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
            }
        });

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
        let prepared = {
            let mut sessions = crate::shared::sync::lock(&self.sessions);
            let expired = sessions.get(session_id).is_some_and(|session| {
                !session.background_started && session.created_at.elapsed() >= SESSION_TTL
            });
            if expired {
                if let Some(session) = sessions.remove(session_id) {
                    session.cancelled.store(true, Ordering::Release);
                }
                crate::shared::sync::lock(&self.background_statuses).remove(session_id);
                return Ok(ConsumedOnboardingSnapshot::FullFallback);
            }
            let session = sessions.get_mut(session_id).ok_or_else(|| {
                AppError::NotFound("Onboarding indexing session was not found".to_string())
            })?;
            let pending = session.games.remove(game_id).ok_or_else(|| {
                AppError::NotFound(format!(
                    "Game '{game_id}' is not available in this onboarding indexing session"
                ))
            })?;
            if session.games.is_empty() {
                sessions.remove(session_id);
            }
            pending.prepared
        };
        let mut prepared = prepared.await.map_err(|_| AppError::Cancelled)??;

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
        if changed_paths.is_empty() {
            return Ok(ConsumedOnboardingSnapshot::Snapshot(Box::new(
                OnboardingSnapshotLease { prepared },
            )));
        }

        refresh_changed_roots(&mut prepared, &changed_paths).await?;
        Ok(ConsumedOnboardingSnapshot::Snapshot(Box::new(
            OnboardingSnapshotLease { prepared },
        )))
    }

    pub fn cancel(&self, session_id: &str) -> Result<(), AppError> {
        if let Some(session) = crate::shared::sync::lock(&self.sessions).remove(session_id) {
            session.cancelled.store(true, Ordering::Release);
        }
        crate::shared::sync::lock(&self.background_statuses).remove(session_id);
        Ok(())
    }

    fn purge_expired(&self) {
        let mut expired_session_ids = Vec::new();
        crate::shared::sync::lock(&self.sessions).retain(|session_id, session| {
            let expired =
                !session.background_started && session.created_at.elapsed() >= SESSION_TTL;
            if expired {
                session.cancelled.store(true, Ordering::Release);
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
                    sessions.remove(&session_id);
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
) -> Result<PreparedGame, AppError> {
    let watcher = RawOnboardingWatcher::start(&game.mod_path)?;
    let snapshot_game = game.clone();
    let discovery = tokio::task::spawn_blocking(move || {
        let on_progress = |progress| progress_reporter.report_discovery(progress);
        let onboarding = collect_onboarding_disk_discovery_with_progress(
            &snapshot_game.mod_path,
            Some(&on_progress),
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
        // Cross-root ambiguity already promoted discovery to a full scan.
        let mods_path = prepared.game.mod_path.clone();
        let full = tokio::task::spawn_blocking(move || {
            collect_onboarding_disk_discovery(&mods_path).map_err(snapshot_error)
        })
        .await??;
        prepared.discovery = full.discovery;
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
    async fn scheduled_expiration_removes_orphaned_session() {
        let store = OnboardingIndexingSessionStore::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        crate::shared::sync::lock(&store.sessions).insert(
            "expired".to_string(),
            OnboardingSession {
                created_at: Instant::now(),
                games: HashMap::new(),
                cancelled: Arc::clone(&cancelled),
                background_started: false,
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
                cancelled: Arc::clone(&cancelled),
                background_started: true,
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
