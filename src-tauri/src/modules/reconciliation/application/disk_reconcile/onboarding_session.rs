//! Short-lived disk snapshots used by the onboarding indexer.
//!
//! The session watcher starts before the initial scan. A clean session can
//! therefore project the captured discovery directly; any uncertain watcher
//! state falls back to the normal full reconcile path.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use uuid::Uuid;

use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::{
    collect_onboarding_disk_discovery, collect_scoped_onboarding_disk_discovery,
    DiskProjectionError, DiskScopedDiscovery,
};
use crate::modules::reconciliation::application::disk_reconcile::types::{
    OnboardingIndexingSession, OnboardingIndexingSnapshotPhase, OnboardingIndexingSnapshotProgress,
    OnboardingIndexingWorkPlan, OnboardingIndexingWorkPlanUpdate,
};
use crate::modules::settings::application::config::GameConfig;
use crate::shared::errors::AppError;

const SESSION_TTL: Duration = Duration::from_secs(15 * 60);

pub struct OnboardingIndexingSessionStore {
    sessions: Arc<Mutex<HashMap<String, OnboardingSession>>>,
}

impl Default for OnboardingIndexingSessionStore {
    fn default() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

struct OnboardingSession {
    created_at: Instant,
    games: HashMap<String, PreparedGame>,
}

struct PreparedGame {
    game: GameConfig,
    work_plan: OnboardingIndexingWorkPlan,
    discovery: DiskScopedDiscovery,
    // This raw watcher deliberately bypasses workspace debounce/filtering so
    // every asset change invalidates the snapshot.
    watcher: RawOnboardingWatcher,
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
    Snapshot(OnboardingSnapshotLease),
    FullFallback,
}

/// Keeps the transient watcher alive while the captured discovery is applied.
/// If it observes another event before commit, the command runs a terminal
/// full reconcile after the snapshot projection.
pub struct OnboardingSnapshotLease {
    prepared: PreparedGame,
    work_plan_update: Option<OnboardingIndexingWorkPlanUpdate>,
}

impl OnboardingSnapshotLease {
    pub fn discovery(&self) -> DiskScopedDiscovery {
        self.prepared.discovery.clone()
    }

    pub fn work_plan_update(&self) -> Option<OnboardingIndexingWorkPlanUpdate> {
        self.work_plan_update.clone()
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
        let game_ids = games.iter().map(|game| game.id.clone()).collect::<Vec<_>>();
        let total_games = game_ids.len() as u64;
        let on_progress = Arc::new(on_progress);
        let mut prepared_games = HashMap::new();
        for (index, game) in games.into_iter().enumerate() {
            on_progress(OnboardingIndexingSnapshotProgress {
                session_id: session_id.clone(),
                game_id: game.id.clone(),
                phase: OnboardingIndexingSnapshotPhase::Scanning,
                completed_games: index as u64,
                total_games,
            });
            let prepared = prepare_game(game).await?;
            on_progress(OnboardingIndexingSnapshotProgress {
                session_id: session_id.clone(),
                game_id: prepared.game.id.clone(),
                phase: OnboardingIndexingSnapshotPhase::Ready,
                completed_games: index as u64 + 1,
                total_games,
            });
            prepared_games.insert(prepared.game.id.clone(), prepared);
        }
        let work_plans = game_ids
            .iter()
            .filter_map(|game_id| prepared_games.get(game_id))
            .map(|prepared| prepared.work_plan.clone())
            .collect::<Vec<_>>();
        self.purge_expired();
        let mut sessions = crate::shared::sync::lock(&self.sessions);
        sessions.insert(
            session_id.clone(),
            OnboardingSession {
                created_at: Instant::now(),
                games: prepared_games,
            },
        );
        self.schedule_expiration(session_id.clone(), SESSION_TTL);

        Ok(OnboardingIndexingSession {
            session_id,
            work_plans,
        })
    }

    /// Removes a game from its session before reconciliation. This makes each
    /// snapshot single-use and guarantees its native watcher is stopped on all
    /// terminal paths.
    pub async fn consume(
        &self,
        session_id: &str,
        game_id: &str,
    ) -> Result<ConsumedOnboardingSnapshot, AppError> {
        let mut prepared = {
            let mut sessions = crate::shared::sync::lock(&self.sessions);
            let expired = sessions
                .get(session_id)
                .is_some_and(|session| session.created_at.elapsed() >= SESSION_TTL);
            if expired {
                sessions.remove(session_id);
                return Ok(ConsumedOnboardingSnapshot::FullFallback);
            }
            let session = sessions.get_mut(session_id).ok_or_else(|| {
                AppError::NotFound("Onboarding indexing session was not found".to_string())
            })?;
            let prepared = session.games.remove(game_id).ok_or_else(|| {
                AppError::NotFound(format!(
                    "Game '{game_id}' is not available in this onboarding indexing session"
                ))
            })?;
            if session.games.is_empty() {
                sessions.remove(session_id);
            }
            prepared
        };

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
            return Ok(ConsumedOnboardingSnapshot::Snapshot(
                OnboardingSnapshotLease {
                    prepared,
                    work_plan_update: None,
                },
            ));
        }

        let update = refresh_changed_roots(session_id, &mut prepared, &changed_paths).await?;
        Ok(ConsumedOnboardingSnapshot::Snapshot(
            OnboardingSnapshotLease {
                prepared,
                work_plan_update: Some(update),
            },
        ))
    }

    pub fn cancel(&self, session_id: &str) -> Result<(), AppError> {
        crate::shared::sync::lock(&self.sessions).remove(session_id);
        Ok(())
    }

    fn purge_expired(&self) {
        crate::shared::sync::lock(&self.sessions)
            .retain(|_, session| session.created_at.elapsed() < SESSION_TTL);
    }

    fn schedule_expiration(&self, session_id: String, delay: Duration) {
        let sessions = Arc::clone(&self.sessions);
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            crate::shared::sync::lock(&sessions).remove(&session_id);
        });
    }
}

async fn prepare_game(game: GameConfig) -> Result<PreparedGame, AppError> {
    let watcher = RawOnboardingWatcher::start(&game.mod_path)?;
    let snapshot_game = game.clone();
    let (discovery, work_plan) = tokio::task::spawn_blocking(move || {
        let onboarding =
            collect_onboarding_disk_discovery(snapshot_game.id.clone(), &snapshot_game.mod_path)
                .map_err(snapshot_error)?;
        Ok::<_, AppError>((onboarding.discovery, onboarding.work_plan))
    })
    .await??;

    Ok(PreparedGame {
        game,
        work_plan,
        discovery,
        watcher,
    })
}

async fn refresh_changed_roots(
    session_id: &str,
    prepared: &mut PreparedGame,
    changed_paths: &[String],
) -> Result<OnboardingIndexingWorkPlanUpdate, AppError> {
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
    let game = prepared.game.clone();
    let refreshed = tokio::task::spawn_blocking(move || {
        let onboarding =
            collect_scoped_onboarding_disk_discovery(game.id.clone(), &mods_path, &roots_for_scan)
                .map_err(snapshot_error)?;
        Ok::<_, AppError>((onboarding.discovery, onboarding.work_plan))
    })
    .await??;

    // A cross-root identity collision correctly promotes the discovery to a
    // full scan. Otherwise compose the changed roots into the original full
    // projection so the writer can retain full-scan prune semantics.
    if refreshed.0.scoped {
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
            .extend(refreshed.0.projection.objects);
        prepared
            .discovery
            .projection
            .mods
            .extend(refreshed.0.projection.mods);
        prepared.discovery.census = refreshed.0.census;
        prepared.discovery.scoped = false;
    }
    if refreshed.0.scoped {
        let changed_root_names = changed_roots.into_iter().collect::<BTreeSet<_>>();
        prepared
            .work_plan
            .roots
            .retain(|root| !changed_root_names.contains(&root.root_name));
        prepared.work_plan.roots.extend(refreshed.1.roots);
        prepared
            .work_plan
            .roots
            .sort_by(|left, right| left.root_name.cmp(&right.root_name));
        prepared.work_plan.file_count = prepared
            .work_plan
            .roots
            .iter()
            .map(|root| root.file_count)
            .sum();
        prepared.work_plan.total_bytes = prepared
            .work_plan
            .roots
            .iter()
            .map(|root| root.total_bytes)
            .sum();
        prepared.work_plan.work_units = prepared
            .work_plan
            .roots
            .iter()
            .map(|root| root.work_units)
            .sum();
    } else {
        // Cross-root ambiguity already promoted projection discovery to a full
        // scan, so refresh the plan from the same full metadata pass.
        let game_id = prepared.game.id.clone();
        let mods_path = prepared.game.mod_path.clone();
        let full = tokio::task::spawn_blocking(move || {
            collect_onboarding_disk_discovery(game_id, &mods_path).map_err(snapshot_error)
        })
        .await??;
        prepared.discovery = full.discovery;
        prepared.work_plan = full.work_plan;
    }

    Ok(OnboardingIndexingWorkPlanUpdate {
        session_id: session_id.to_string(),
        work_plan: prepared.work_plan.clone(),
    })
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
    async fn scheduled_expiration_removes_orphaned_session() {
        let store = OnboardingIndexingSessionStore::new();
        crate::shared::sync::lock(&store.sessions).insert(
            "expired".to_string(),
            OnboardingSession {
                created_at: Instant::now(),
                games: HashMap::new(),
            },
        );
        store.schedule_expiration("expired".to_string(), Duration::from_millis(1));
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!crate::shared::sync::lock(&store.sessions).contains_key("expired"));
    }
}
