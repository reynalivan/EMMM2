//! Caller-facing inputs: the shared context handle and the reconcile request
//! (manual or coalesced watcher batch).

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

use tauri::Emitter;

use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcilePhase, DiskReconcileProgress, DiskReconcileReason,
};
use crate::modules::workspace::application::scanner::watcher::{
    ModWatchEvent, WatcherSession, WatcherSuppressor,
};

use super::state::DiskReconcileState;

const WATCHER_FORCE_FULL_BATCH_SIZE: usize = 128;
const PROGRESS_EMIT_INTERVAL_MS: u128 = 125;
static NEXT_PROGRESS_RUN_ID: AtomicU64 = AtomicU64::new(1);

/// A short-lived, per-pass event emitter. Progress is derived from completed
/// top-level roots, never from a fabricated timer.
pub struct DiskReconcileProgressReporter {
    app: tauri::AppHandle,
    game_id: String,
    run_id: String,
    reason: DiskReconcileReason,
    started_at: Instant,
    last_emitted_at: Mutex<u128>,
}

impl DiskReconcileProgressReporter {
    pub fn new(
        app: tauri::AppHandle,
        game_id: impl Into<String>,
        reason: DiskReconcileReason,
    ) -> Self {
        let game_id = game_id.into();
        let started_at = Instant::now();
        Self {
            app,
            run_id: format!(
                "{}-{}",
                game_id,
                NEXT_PROGRESS_RUN_ID.fetch_add(1, Ordering::Relaxed)
            ),
            game_id,
            reason,
            started_at,
            last_emitted_at: Mutex::new(0),
        }
    }

    pub fn emit(
        &self,
        phase: DiskReconcilePhase,
        completed_units: u64,
        total_units: Option<u64>,
        current_root: Option<String>,
    ) {
        let elapsed_ms = self.started_at.elapsed().as_millis();
        let is_terminal_root = total_units.is_some_and(|total| total == completed_units);
        if phase == DiskReconcilePhase::ScanningRoots && completed_units > 0 && !is_terminal_root {
            let Ok(mut last_emitted_at) = self.last_emitted_at.lock() else {
                return;
            };
            if elapsed_ms.saturating_sub(*last_emitted_at) < PROGRESS_EMIT_INTERVAL_MS {
                return;
            }
            *last_emitted_at = elapsed_ms;
        }

        let eta_ms = estimate_eta_ms(elapsed_ms, completed_units, total_units);
        let progress = DiskReconcileProgress {
            game_id: self.game_id.clone(),
            run_id: self.run_id.clone(),
            reason: self.reason.clone(),
            phase,
            completed_units,
            total_units,
            current_root,
            elapsed_ms: elapsed_ms.min(u64::MAX as u128) as u64,
            eta_ms,
        };
        if let Err(error) = self.app.emit("disk_reconcile:progress", progress) {
            log::debug!("Could not emit disk reconcile progress: {error}");
        }
    }
}

fn estimate_eta_ms(
    elapsed_ms: u128,
    completed_units: u64,
    total_units: Option<u64>,
) -> Option<u64> {
    let total_units = total_units?;
    if completed_units < 2 || completed_units >= total_units {
        return None;
    }
    let remaining = u128::from(total_units - completed_units);
    let estimated = elapsed_ms
        .saturating_mul(remaining)
        .checked_div(u128::from(completed_units))?;
    Some(estimated.min(u64::MAX as u128) as u64)
}

#[derive(Clone)]
pub struct DiskReconcileContext<'a> {
    pub pool: &'a sqlx::SqlitePool,
    pub config: &'a crate::modules::settings::application::config::ConfigService,
    pub state: &'a DiskReconcileState,
    pub watcher_suppressor: Arc<WatcherSuppressor>,
    pub operation_lock: &'a crate::platform::fs::operation_lock::OperationLock,
    pub progress_reporter: Option<Arc<DiskReconcileProgressReporter>>,
}

pub struct DiskReconcileRequest {
    pub(super) game_id: String,
    pub(super) reason: DiskReconcileReason,
    pub(super) changed_paths: Vec<String>,
    pub(super) force_full: bool,
    pub(super) watcher_events: Vec<ModWatchEvent>,
    pub(super) watcher_session: Option<WatcherSession>,
    pub(super) path_hints: Vec<DiskReconcilePathHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskReconcilePathHint {
    pub old_path: String,
    pub new_path: String,
    pub target_object_id: String,
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
            watcher_session: None,
            path_hints: Vec::new(),
        }
    }

    pub fn manual_with_path_hints(
        game_id: String,
        reason: DiskReconcileReason,
        changed_paths: Vec<String>,
        path_hints: Vec<DiskReconcilePathHint>,
    ) -> Self {
        Self {
            game_id,
            reason,
            changed_paths,
            force_full: false,
            watcher_events: Vec::new(),
            watcher_session: None,
            path_hints,
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
            watcher_session: None,
            path_hints: Vec::new(),
        }
    }

    pub fn rename_resolutions(game_id: String, watcher_events: Vec<ModWatchEvent>) -> Self {
        let changed_paths =
            crate::modules::reconciliation::application::disk_reconcile::watcher_batch::collect_changed_paths(&watcher_events);
        Self {
            game_id,
            reason: DiskReconcileReason::ManualRepair,
            changed_paths,
            force_full: true,
            watcher_events,
            watcher_session: None,
            path_hints: Vec::new(),
        }
    }

    pub(crate) fn for_watcher_session(mut self, session: WatcherSession) -> Self {
        self.watcher_session = Some(session);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::estimate_eta_ms;

    #[test]
    fn eta_is_only_available_after_two_completed_roots() {
        assert_eq!(estimate_eta_ms(1_000, 1, Some(5)), None);
        assert_eq!(estimate_eta_ms(1_000, 2, Some(5)), Some(1_500));
        assert_eq!(estimate_eta_ms(1_000, 5, Some(5)), None);
    }
}
