//! Per-game queue state: version counters, the coalesced pending request, and
//! the last published result.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::services::disk_reconcile::types::{DiskReconcileReason, DiskReconcileResult};
use crate::services::scanner::watcher::ModWatchEvent;

#[derive(Debug, Clone)]
pub(super) struct PendingSyncRequest {
    pub(super) changed_paths: BTreeSet<String>,
    pub(super) force_full: bool,
    pub(super) reason: DiskReconcileReason,
    pub(super) max_version: u64,
    pub(super) watcher_events: Vec<ModWatchEvent>,
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

    pub(super) fn lock_for_game(&self, game_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().expect("disk reconcile locks poisoned");
        locks
            .entry(game_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub(super) fn enqueue_request(
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

    pub(super) fn take_pending_or_cached(
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

    /// Put a taken pending request back after a failed run so queued
    /// waiters and coalesced watcher events are not lost.
    pub(super) fn requeue_pending(&self, game_id: &str, taken: PendingSyncRequest) {
        let mut games = self.games.lock().expect("disk reconcile state poisoned");
        let state = games.entry(game_id.to_string()).or_default();
        match state.pending.as_mut() {
            Some(pending) => {
                pending.changed_paths.extend(taken.changed_paths);
                pending.force_full |= taken.force_full;
                pending.max_version = pending.max_version.max(taken.max_version);
                let mut events = taken.watcher_events;
                events.append(&mut pending.watcher_events);
                pending.watcher_events = events;
            }
            None => state.pending = Some(taken),
        }
    }

    pub(super) fn finish_run(
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

    pub(super) fn last_result(&self, game_id: &str) -> Result<DiskReconcileResult, String> {
        let games = self.games.lock().expect("disk reconcile state poisoned");
        games
            .get(game_id)
            .and_then(|state| state.last_result.clone())
            .ok_or_else(|| format!("Disk Reconcile result missing for game '{game_id}'"))
    }
}
