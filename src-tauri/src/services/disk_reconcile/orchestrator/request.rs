//! Caller-facing inputs: the shared context handle and the reconcile request
//! (manual or coalesced watcher batch).

use std::sync::Arc;

use crate::services::disk_reconcile::types::DiskReconcileReason;
use crate::services::scanner::watcher::{ModWatchEvent, WatcherSuppressor};

use super::state::DiskReconcileState;

const WATCHER_FORCE_FULL_BATCH_SIZE: usize = 128;

#[derive(Clone)]
pub struct DiskReconcileContext<'a> {
    pub pool: &'a sqlx::SqlitePool,
    pub config: &'a crate::services::config::ConfigService,
    pub state: &'a DiskReconcileState,
    pub watcher_suppressor: Arc<WatcherSuppressor>,
}

pub struct DiskReconcileRequest {
    pub(super) game_id: String,
    pub(super) reason: DiskReconcileReason,
    pub(super) changed_paths: Vec<String>,
    pub(super) force_full: bool,
    pub(super) watcher_events: Vec<ModWatchEvent>,
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
