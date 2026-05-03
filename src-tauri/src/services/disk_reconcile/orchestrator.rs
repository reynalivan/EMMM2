use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::services::disk_reconcile::reconcile::reconcile_disk_projection;
use crate::services::disk_reconcile::types::{
    DiskReconcileReason, DiskReconcileResult, DiskReconcileStatus,
};
use crate::services::scanner::watcher::{ModWatchEvent, WatcherSuppressor};

const WATCHER_FORCE_FULL_BATCH_SIZE: usize = 128;

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

async fn finalize_runtime_effects(
    pool: &sqlx::SqlitePool,
    config: &crate::services::config::ConfigService,
    watcher_suppressor: Arc<WatcherSuppressor>,
    game_id: &str,
    reason: DiskReconcileReason,
    changed_roots: Vec<String>,
    thumbnail_roots: Vec<String>,
    objects_changed: bool,
    folders_changed: bool,
    runtime_file_changed: bool,
    cleared_selection_paths: Vec<String>,
    path_updates: Vec<crate::services::disk_reconcile::types::DiskReconcilePathUpdate>,
    change_summary: crate::services::disk_reconcile::types::DiskReconcileChangeSummary,
    status: DiskReconcileStatus,
    error_message: Option<String>,
) -> DiskReconcileResult {
    let collections_changed = status == DiskReconcileStatus::Applied
        && (folders_changed || objects_changed || runtime_file_changed);

    let overlay_refresh_triggered = if status == DiskReconcileStatus::Applied {
        match crate::services::app::runtime_effects::finalize_runtime_side_effects(
            pool,
            config,
            watcher_suppressor,
            game_id,
            &[true, false],
            collections_changed,
            folders_changed || runtime_file_changed,
        )
        .await
        {
            Ok(triggered) => triggered,
            Err(error) => {
                log::warn!(
                    "Disk Reconcile runtime side-effects failed for game '{}': {}",
                    game_id,
                    error
                );
                false
            }
        }
    } else {
        false
    };

    DiskReconcileResult {
        game_id: game_id.to_string(),
        reason,
        status,
        error_message,
        changed_roots,
        objects_changed,
        folders_changed,
        collections_changed,
        runtime_file_changed,
        overlay_refresh_triggered,
        thumbnail_roots,
        cleared_selection_paths,
        path_updates,
        change_summary,
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_refresh_once(
    _app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    config: &crate::services::config::ConfigService,
    watcher_suppressor: Arc<WatcherSuppressor>,
    game_id: &str,
    reason: DiskReconcileReason,
    changed_paths: Vec<String>,
    force_full: bool,
    watcher_events: Option<&[ModWatchEvent]>,
) -> Result<DiskReconcileResult, String> {
    let settings = config.get_settings();
    let game = settings
        .games
        .iter()
        .find(|entry| entry.id == game_id)
        .ok_or_else(|| format!("Game '{}' not found for disk reconcile", game_id))?;

    let reconcile = reconcile_disk_projection(
        pool,
        game_id,
        &game.mod_path,
        &settings.safe_mode.keywords,
        &reason,
        &changed_paths,
        force_full,
        watcher_events,
    )
    .await?;

    Ok(finalize_runtime_effects(
        pool,
        config,
        watcher_suppressor,
        game_id,
        reason,
        reconcile.changed_roots,
        reconcile.thumbnail_roots,
        reconcile.objects_changed,
        reconcile.folders_changed,
        reconcile.runtime_file_changed,
        reconcile.cleared_selection_paths,
        reconcile.path_updates,
        reconcile.change_summary,
        reconcile.status,
        reconcile.error_message,
    )
    .await)
}

#[allow(clippy::too_many_arguments)]
/// Disk Reconcile keeps runtime projection aligned with filesystem reality.
/// Watcher, focus, and Mods view entry must call this path only.
/// Do not add Deep Match Scanner logic here.
pub async fn reconcile_disk_state(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    config: &crate::services::config::ConfigService,
    disk_reconcile_state: &DiskReconcileState,
    watcher_suppressor: Arc<WatcherSuppressor>,
    game_id: String,
    reason: DiskReconcileReason,
    changed_paths: Vec<String>,
    force_full: bool,
) -> Result<DiskReconcileResult, String> {
    reconcile_disk_state_internal(
        app,
        pool,
        config,
        disk_reconcile_state,
        watcher_suppressor,
        game_id,
        reason,
        changed_paths,
        force_full,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
/// Disk Reconcile watcher batches must stay disk-only.
/// Watcher must never invoke the Deep Match Scanner pipeline.
pub async fn reconcile_disk_state_from_watcher_batch(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    config: &crate::services::config::ConfigService,
    disk_reconcile_state: &DiskReconcileState,
    watcher_suppressor: Arc<WatcherSuppressor>,
    game_id: String,
    changed_paths: Vec<String>,
    watcher_events: &[ModWatchEvent],
) -> Result<DiskReconcileResult, String> {
    let force_full = watcher_events.len() >= WATCHER_FORCE_FULL_BATCH_SIZE
        || (watcher_events.len() > 0 && changed_paths.is_empty());
    reconcile_disk_state_internal(
        app,
        pool,
        config,
        disk_reconcile_state,
        watcher_suppressor,
        game_id,
        DiskReconcileReason::WatcherBatch,
        changed_paths,
        force_full,
        Some(watcher_events),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn reconcile_disk_state_internal(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    config: &crate::services::config::ConfigService,
    disk_reconcile_state: &DiskReconcileState,
    watcher_suppressor: Arc<WatcherSuppressor>,
    game_id: String,
    reason: DiskReconcileReason,
    changed_paths: Vec<String>,
    force_full: bool,
    watcher_events: Option<&[ModWatchEvent]>,
) -> Result<DiskReconcileResult, String> {
    let requested_version = disk_reconcile_state.enqueue_request(
        &game_id,
        reason.clone(),
        &changed_paths,
        force_full,
        watcher_events.unwrap_or(&[]),
    );
    let game_lock = disk_reconcile_state.lock_for_game(&game_id);
    let _guard = game_lock.lock().await;

    loop {
        let Some(request) =
            disk_reconcile_state.take_pending_or_cached(&game_id, requested_version)?
        else {
            return disk_reconcile_state.last_result(&game_id);
        };

        let result = run_refresh_once(
            app,
            pool,
            config,
            watcher_suppressor.clone(),
            &game_id,
            request.reason,
            request.changed_paths.into_iter().collect(),
            request.force_full,
            if request.watcher_events.is_empty() {
                None
            } else {
                Some(&request.watcher_events)
            },
        )
        .await?;

        let has_pending = disk_reconcile_state.finish_run(&game_id, request.max_version, &result);
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
