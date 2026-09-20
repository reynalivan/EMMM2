//! Caller-facing inputs: the shared context handle and the reconcile request
//! (manual or coalesced watcher batch).

use std::collections::BTreeSet;
use std::path::{Component, Path};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

use tauri::{Emitter, Manager};

use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::DiskScopedDiscovery;
use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcilePhase, DiskReconcileProgress, DiskReconcileReason,
};
use crate::modules::workspace::application::scanner::watcher::{
    ModWatchEvent, WatcherSession, WatcherState, WatcherSuppressor,
};

use super::state::DiskReconcileState;

/// Once a batch touches this many distinct top-level roots, a complete pass
/// avoids a near-full scoped projection while keeping the decision tied to
/// filesystem coverage rather than noisy per-file events.
const WATCHER_FORCE_FULL_ROOT_COUNT: usize = 128;
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
    watcher_session: Option<WatcherSession>,
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
            watcher_session: None,
        }
    }

    /// Restrict progress from a watcher-owned request to its active watcher
    /// session. Manual, startup, and mutation reconcile progress are not
    /// session-bound.
    pub fn for_watcher_session(mut self, watcher_session: WatcherSession) -> Self {
        self.watcher_session = Some(watcher_session);
        self
    }

    fn watcher_session_is_current(&self) -> bool {
        let Some(session) = &self.watcher_session else {
            return true;
        };
        self.app
            .try_state::<WatcherState>()
            .is_some_and(|state| state.is_current_session(session))
    }

    pub fn emit(
        &self,
        phase: DiskReconcilePhase,
        completed_units: u64,
        total_units: Option<u64>,
        current_root: Option<String>,
    ) {
        if !self.watcher_session_is_current() {
            return;
        }
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
        let emission = match &self.watcher_session {
            Some(session) => self
                .app
                .state::<WatcherState>()
                .with_current_session(session, || {
                    self.app.emit("disk_reconcile:progress", progress)
                }),
            None => Some(self.app.emit("disk_reconcile:progress", progress)),
        };
        let Some(emission) = emission else {
            return;
        };
        if let Err(error) = emission {
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
    /// Only a durable internal mutation, or its locked identity-validated
    /// regional preflight, may set this. It bypasses the global directory
    /// census but never the strict classifier for the affected roots.
    pub(super) trusted_mutation_scope: bool,
    /// The caller owns a later, better-defined overlay boundary. A Mods-root
    /// change publishes only after its replacement watcher is active.
    pub(super) defer_overlay_sync: bool,
    pub(super) precomputed_discovery: Option<DiskScopedDiscovery>,
}

fn watcher_path_root_key(mods_path: &Path, changed_path: &str) -> Option<String> {
    let mods_path_key = crate::shared::path_key::canonical_path_key_for_path(mods_path);
    let prefix = format!("{mods_path_key}/");
    let path = Path::new(changed_path);
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return None;
    }

    let path_key = crate::shared::path_key::canonical_path_key_for_path(path);
    let relative_key = path_key.strip_prefix(&prefix)?;
    let root_key = relative_key.split('/').next()?;
    if root_key.is_empty() || root_key.starts_with('.') {
        return None;
    }

    Some(root_key.to_string())
}

fn watcher_batch_root_keys(mods_path: &Path, changed_paths: &[String]) -> Option<BTreeSet<String>> {
    changed_paths
        .iter()
        .map(|path| watcher_path_root_key(mods_path, path))
        .collect()
}

fn watcher_event_has_cross_root_rename(mods_path: &Path, event: &ModWatchEvent) -> bool {
    match event {
        ModWatchEvent::Renamed { from, to } => watcher_rename_crosses_roots(mods_path, from, to),
        ModWatchEvent::RenameResolution {
            from: Some(from),
            to: Some(to),
            apply_as_rename: true,
            ..
        } => watcher_rename_crosses_roots(mods_path, from, to),
        ModWatchEvent::RenameResolution {
            apply_as_rename: true,
            ..
        } => true,
        ModWatchEvent::Created(_)
        | ModWatchEvent::Removed(_)
        | ModWatchEvent::Modified(_)
        | ModWatchEvent::RenameResolution { .. }
        | ModWatchEvent::Error(_) => false,
    }
}

fn watcher_rename_crosses_roots(mods_path: &Path, from: &str, to: &str) -> bool {
    match (
        watcher_path_root_key(mods_path, from),
        watcher_path_root_key(mods_path, to),
    ) {
        (Some(from_root), Some(to_root)) => from_root != to_root,
        _ => true,
    }
}

fn watcher_batch_requires_full_scan(
    mods_path: &Path,
    changed_paths: &[String],
    watcher_events: &[ModWatchEvent],
) -> bool {
    if watcher_events.is_empty() {
        return false;
    }

    if watcher_events
        .iter()
        .any(|event| matches!(event, ModWatchEvent::Error(_)))
        || changed_paths.is_empty()
        || watcher_events
            .iter()
            .any(|event| watcher_event_has_cross_root_rename(mods_path, event))
    {
        return true;
    }

    watcher_batch_root_keys(mods_path, changed_paths)
        .is_none_or(|root_keys| root_keys.len() >= WATCHER_FORCE_FULL_ROOT_COUNT)
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
            trusted_mutation_scope: false,
            defer_overlay_sync: false,
            precomputed_discovery: None,
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
            trusted_mutation_scope: false,
            defer_overlay_sync: false,
            precomputed_discovery: None,
        }
    }

    pub fn watcher_batch(
        game_id: String,
        mods_path: &Path,
        changed_paths: Vec<String>,
        watcher_events: &[ModWatchEvent],
    ) -> Self {
        let force_full =
            watcher_batch_requires_full_scan(mods_path, &changed_paths, watcher_events);

        Self {
            game_id,
            reason: DiskReconcileReason::WatcherBatch,
            changed_paths,
            force_full,
            watcher_events: watcher_events.to_vec(),
            watcher_session: None,
            path_hints: Vec::new(),
            trusted_mutation_scope: false,
            // A watcher owns only disk observation and projection. Runtime
            // publication is queued by its lifecycle after this request has
            // returned and released both mutation leases.
            defer_overlay_sync: true,
            precomputed_discovery: None,
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
            trusted_mutation_scope: false,
            defer_overlay_sync: false,
            precomputed_discovery: None,
        }
    }

    pub(crate) fn for_watcher_session(mut self, session: WatcherSession) -> Self {
        self.watcher_session = Some(session);
        self
    }

    pub(crate) fn defer_overlay_sync(mut self) -> Self {
        self.defer_overlay_sync = true;
        self
    }

    pub(in crate::modules::reconciliation::application::disk_reconcile) fn trust_durable_mutation_scope(
        mut self,
    ) -> Self {
        debug_assert!(matches!(self.reason, DiskReconcileReason::InternalMutation));
        self.trusted_mutation_scope = true;
        self
    }

    pub(in crate::modules::reconciliation::application::disk_reconcile) fn trust_locked_regional_preflight(
        mut self,
    ) -> Self {
        debug_assert!(matches!(self.reason, DiskReconcileReason::InternalMutation));
        self.trusted_mutation_scope = true;
        self
    }

    pub(crate) fn with_precomputed_discovery(mut self, discovery: DiskScopedDiscovery) -> Self {
        self.precomputed_discovery = Some(discovery);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{estimate_eta_ms, DiskReconcileRequest};
    use crate::modules::workspace::application::scanner::watcher::ModWatchEvent;

    fn modified(path: &std::path::Path) -> ModWatchEvent {
        ModWatchEvent::Modified(path.to_string_lossy().into_owned())
    }

    #[test]
    fn eta_is_only_available_after_two_completed_roots() {
        assert_eq!(estimate_eta_ms(1_000, 1, Some(5)), None);
        assert_eq!(estimate_eta_ms(1_000, 2, Some(5)), Some(1_500));
        assert_eq!(estimate_eta_ms(1_000, 5, Some(5)), None);
    }

    #[test]
    fn watcher_batch_with_many_events_in_one_normalized_root_stays_scoped() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        let paths = (0..128)
            .map(|index| {
                mods_path
                    .join("Alice")
                    .join("Blue")
                    .join(format!("variant-{index}.ini"))
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        let events = paths
            .iter()
            .map(|path| modified(std::path::Path::new(path)))
            .collect::<Vec<_>>();

        let request =
            DiskReconcileRequest::watcher_batch("game-1".to_string(), &mods_path, paths, &events);

        assert!(!request.force_full);
        assert!(request.defer_overlay_sync);
    }

    #[test]
    fn watcher_batch_forces_full_scan_for_wide_or_ambiguous_coverage() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        let paths = (0..128)
            .map(|index| {
                mods_path
                    .join(format!("Root-{index}"))
                    .join("mod.ini")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        let events = paths
            .iter()
            .map(|path| modified(std::path::Path::new(path)))
            .collect::<Vec<_>>();

        let wide_request =
            DiskReconcileRequest::watcher_batch("game-1".to_string(), &mods_path, paths, &events);
        assert!(wide_request.force_full);

        let outside_path = temp.path().join("Elsewhere").join("mod.ini");
        let ambiguous_request = DiskReconcileRequest::watcher_batch(
            "game-1".to_string(),
            &mods_path,
            vec![outside_path.to_string_lossy().into_owned()],
            &[modified(&outside_path)],
        );
        assert!(ambiguous_request.force_full);
    }

    #[test]
    fn watcher_batch_errors_always_force_full_scan() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        let path = mods_path.join("Alice").join("Blue").join("mod.ini");

        let request = DiskReconcileRequest::watcher_batch(
            "game-1".to_string(),
            &mods_path,
            vec![path.to_string_lossy().into_owned()],
            &[ModWatchEvent::Error("watcher overflow".to_string())],
        );

        assert!(request.force_full);
    }

    #[test]
    fn watcher_batch_cross_root_rename_forces_full_scan() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        let from = mods_path.join("Alice").join("Blue");
        let to = mods_path.join("Bob").join("Blue");

        let request = DiskReconcileRequest::watcher_batch(
            "game-1".to_string(),
            &mods_path,
            vec![
                from.to_string_lossy().into_owned(),
                to.to_string_lossy().into_owned(),
            ],
            &[ModWatchEvent::Renamed {
                from: from.to_string_lossy().into_owned(),
                to: to.to_string_lossy().into_owned(),
            }],
        );

        assert!(request.force_full);
    }

    #[test]
    fn watcher_batch_partial_rename_resolution_forces_full_scan() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        let path = mods_path.join("Alice").join("Blue");

        for event in [
            ModWatchEvent::RenameResolution {
                group_id: "rename-1".to_string(),
                from: None,
                to: Some(path.to_string_lossy().into_owned()),
                apply_as_rename: true,
            },
            ModWatchEvent::RenameResolution {
                group_id: "rename-2".to_string(),
                from: Some(path.to_string_lossy().into_owned()),
                to: None,
                apply_as_rename: true,
            },
        ] {
            let request = DiskReconcileRequest::watcher_batch(
                "game-1".to_string(),
                &mods_path,
                vec![path.to_string_lossy().into_owned()],
                &[event],
            );

            assert!(request.force_full);
        }
    }
}
