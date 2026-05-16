use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::services::disk_reconcile::reconcile::{
    reconcile_disk_projection, ReconcileDiskProjectionRequest, ReconcileOutcome,
};
use crate::services::disk_reconcile::types::{
    DiskReconcileReason, DiskReconcileResult, DiskReconcileStatus,
};
use crate::services::scanner::watcher::{ModWatchEvent, WatcherSuppressor};

const WATCHER_FORCE_FULL_BATCH_SIZE: usize = 128;

#[derive(Clone)]
pub struct DiskReconcileContext<'a> {
    pub pool: &'a sqlx::SqlitePool,
    pub config: &'a crate::services::config::ConfigService,
    pub state: &'a DiskReconcileState,
    pub watcher_suppressor: Arc<WatcherSuppressor>,
}

pub struct DiskReconcileRequest {
    game_id: String,
    reason: DiskReconcileReason,
    changed_paths: Vec<String>,
    force_full: bool,
    watcher_events: Vec<ModWatchEvent>,
}

impl DiskReconcileRequest {
    pub fn manual(
        game_id: String,
        reason: DiskReconcileReason,
        changed_paths: Vec<String>,
        force_full: bool,
    ) -> Self {
        Self {
            game_id,
            reason,
            changed_paths,
            force_full,
            watcher_events: Vec::new(),
        }
    }

    pub fn watcher_batch(
        game_id: String,
        changed_paths: Vec<String>,
        watcher_events: &[ModWatchEvent],
    ) -> Self {
        let force_full = watcher_events.len() >= WATCHER_FORCE_FULL_BATCH_SIZE
            || (!watcher_events.is_empty() && changed_paths.is_empty());

        Self {
            game_id,
            reason: DiskReconcileReason::WatcherBatch,
            changed_paths,
            force_full,
            watcher_events: watcher_events.to_vec(),
        }
    }
}

#[derive(Debug, Clone)]
struct PendingSyncRequest {
    changed_paths: BTreeSet<String>,
    force_full: bool,
    reason: DiskReconcileReason,
    max_version: u64,
    watcher_events: Vec<ModWatchEvent>,
}

#[derive(Debug, Default, Clone)]
struct GameSyncState {
    next_version: u64,
    completed_version: u64,
    pending: Option<PendingSyncRequest>,
    last_result: Option<DiskReconcileResult>,
}

#[derive(Default)]
pub struct DiskReconcileState {
    locks: std::sync::Mutex<HashMap<String, Arc<Mutex<()>>>>,
    games: std::sync::Mutex<HashMap<String, GameSyncState>>,
}

impl DiskReconcileState {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock_for_game(&self, game_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().expect("disk reconcile locks poisoned");
        locks
            .entry(game_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    fn enqueue_request(
        &self,
        game_id: &str,
        reason: DiskReconcileReason,
        changed_paths: &[String],
        force_full: bool,
        watcher_events: &[ModWatchEvent],
    ) -> u64 {
        let mut games = self.games.lock().expect("disk reconcile state poisoned");
        let state = games.entry(game_id.to_string()).or_default();
        state.next_version += 1;
        let version = state.next_version;

        match state.pending.as_mut() {
            Some(pending) => {
                pending.changed_paths.extend(changed_paths.iter().cloned());
                pending.force_full |= force_full;
                pending.reason = reason;
                pending.max_version = version;
                pending
                    .watcher_events
                    .extend(watcher_events.iter().cloned());
            }
            None => {
                state.pending = Some(PendingSyncRequest {
                    changed_paths: changed_paths.iter().cloned().collect(),
                    force_full,
                    reason,
                    max_version: version,
                    watcher_events: watcher_events.to_vec(),
                });
            }
        }

        version
    }

    fn take_pending_or_cached(
        &self,
        game_id: &str,
        requested_version: u64,
    ) -> Result<Option<PendingSyncRequest>, String> {
        let mut games = self.games.lock().expect("disk reconcile state poisoned");
        let state = games.entry(game_id.to_string()).or_default();

        if state.completed_version >= requested_version {
            return Ok(None);
        }

        state
            .pending
            .take()
            .ok_or_else(|| format!("Disk Reconcile request lost for game '{game_id}'"))
            .map(Some)
    }

    fn finish_run(
        &self,
        game_id: &str,
        completed_version: u64,
        result: &DiskReconcileResult,
    ) -> bool {
        let mut games = self.games.lock().expect("disk reconcile state poisoned");
        let state = games.entry(game_id.to_string()).or_default();
        state.completed_version = completed_version;
        state.last_result = Some(result.clone());
        state.pending.is_some()
    }

    fn last_result(&self, game_id: &str) -> Result<DiskReconcileResult, String> {
        let games = self.games.lock().expect("disk reconcile state poisoned");
        games
            .get(game_id)
            .and_then(|state| state.last_result.clone())
            .ok_or_else(|| format!("Disk Reconcile result missing for game '{game_id}'"))
    }
}

struct RuntimeEffectsRequest<'a> {
    context: DiskReconcileContext<'a>,
    game_id: &'a str,
    reason: DiskReconcileReason,
    outcome: ReconcileOutcome,
}

async fn finalize_runtime_effects(request: RuntimeEffectsRequest<'_>) -> DiskReconcileResult {
    let collections_changed = request.outcome.status == DiskReconcileStatus::Applied
        && (request.outcome.folders_changed
            || request.outcome.objects_changed
            || request.outcome.runtime_file_changed);

    let overlay_refresh_triggered = if request.outcome.status == DiskReconcileStatus::Applied {
        match crate::services::app::runtime_effects::finalize_runtime_side_effects(
            request.context.pool,
            request.context.config,
            request.context.watcher_suppressor,
            request.game_id,
            &[true, false],
            collections_changed,
            request.outcome.folders_changed || request.outcome.runtime_file_changed,
        )
        .await
        {
            Ok(triggered) => triggered,
            Err(error) => {
                log::warn!(
                    "Disk Reconcile runtime side-effects failed for game '{}': {}",
                    request.game_id,
                    error
                );
                false
            }
        }
    } else {
        false
    };

    DiskReconcileResult {
        game_id: request.game_id.to_string(),
        reason: request.reason,
        status: request.outcome.status,
        error_message: request.outcome.error_message,
        changed_roots: request.outcome.changed_roots,
        objects_changed: request.outcome.objects_changed,
        folders_changed: request.outcome.folders_changed,
        collections_changed,
        runtime_file_changed: request.outcome.runtime_file_changed,
        overlay_refresh_triggered,
        thumbnail_roots: request.outcome.thumbnail_roots,
        cleared_selection_paths: request.outcome.cleared_selection_paths,
        path_updates: request.outcome.path_updates,
        collection_reference_impact: request.outcome.collection_reference_impact,
        change_summary: request.outcome.change_summary,
    }
}

struct RefreshRequest<'a> {
    context: DiskReconcileContext<'a>,
    game_id: &'a str,
    reason: DiskReconcileReason,
    changed_paths: Vec<String>,
    force_full: bool,
    watcher_events: Vec<ModWatchEvent>,
}

async fn run_refresh_once(request: RefreshRequest<'_>) -> Result<DiskReconcileResult, String> {
    let settings = request.context.config.get_settings();
    let game = settings
        .games
        .iter()
        .find(|entry| entry.id == request.game_id)
        .ok_or_else(|| format!("Game '{}' not found for disk reconcile", request.game_id))?;
    let watcher_events = if request.watcher_events.is_empty() {
        None
    } else {
        Some(request.watcher_events.as_slice())
    };

    let reconcile = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: request.context.pool,
        game_id: request.game_id,
        mods_path: &game.mod_path,
        safe_mode_keywords: &settings.safe_mode.keywords,
        reason: &request.reason,
        changed_paths: &request.changed_paths,
        force_full: request.force_full,
        watcher_events,
    })
    .await?;

    Ok(finalize_runtime_effects(RuntimeEffectsRequest {
        context: request.context,
        game_id: request.game_id,
        reason: request.reason,
        outcome: reconcile,
    })
    .await)
}

/// Disk Reconcile keeps runtime projection aligned with filesystem reality.
/// Watcher, focus, and Mods view entry must call this path only.
/// Do not add Deep Match Scanner logic here.
pub async fn reconcile_disk_state(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
) -> Result<DiskReconcileResult, String> {
    reconcile_disk_state_internal(context, request).await
}

/// Disk Reconcile watcher batches must stay disk-only.
/// Watcher must never invoke the Deep Match Scanner pipeline.
pub async fn reconcile_disk_state_from_watcher_batch(
    context: DiskReconcileContext<'_>,
    game_id: String,
    changed_paths: Vec<String>,
    watcher_events: &[ModWatchEvent],
) -> Result<DiskReconcileResult, String> {
    reconcile_disk_state_internal(
        context,
        DiskReconcileRequest::watcher_batch(game_id, changed_paths, watcher_events),
    )
    .await
}

async fn reconcile_disk_state_internal(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
) -> Result<DiskReconcileResult, String> {
    let game_id = request.game_id;
    let requested_version = context.state.enqueue_request(
        &game_id,
        request.reason.clone(),
        &request.changed_paths,
        request.force_full,
        &request.watcher_events,
    );
    let game_lock = context.state.lock_for_game(&game_id);
    let _guard = game_lock.lock().await;

    loop {
        let Some(pending) = context
            .state
            .take_pending_or_cached(&game_id, requested_version)?
        else {
            return context.state.last_result(&game_id);
        };

        let result = run_refresh_once(RefreshRequest {
            context: context.clone(),
            game_id: &game_id,
            reason: pending.reason,
            changed_paths: pending.changed_paths.into_iter().collect(),
            force_full: pending.force_full,
            watcher_events: pending.watcher_events,
        })
        .await?;

        let has_pending = context
            .state
            .finish_run(&game_id, pending.max_version, &result);
        if !has_pending {
            return Ok(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_request_preserves_watcher_events_across_coalesced_batches() {
        let state = DiskReconcileState::new();
        let first_events = vec![ModWatchEvent::Renamed {
            from: "E:/Mods/Alice/Old".to_string(),
            to: "E:/Mods/Alice/New".to_string(),
        }];
        let second_events = vec![ModWatchEvent::Created("E:/Mods/Bob/Blue".to_string())];

        state.enqueue_request(
            "game-1",
            DiskReconcileReason::WatcherBatch,
            &["E:/Mods/Alice/Old".to_string()],
            false,
            &first_events,
        );
        let version = state.enqueue_request(
            "game-1",
            DiskReconcileReason::WatcherBatch,
            &["E:/Mods/Bob/Blue".to_string()],
            false,
            &second_events,
        );

        let request = state
            .take_pending_or_cached("game-1", version)
            .expect("pending request should be readable")
            .expect("pending request should exist");

        assert_eq!(request.watcher_events.len(), 2);
        assert!(matches!(
            request.watcher_events[0],
            ModWatchEvent::Renamed { .. }
        ));
        assert!(matches!(
            request.watcher_events[1],
            ModWatchEvent::Created(_)
        ));
    }
}
