//! File system watcher for mod directories.
//!
//! Uses `notify-debouncer-full` over the `notify` v8 recommended watcher:
//! debouncing, event dedup and rename From/To stitching (via Windows file
//! IDs) all happen in the debouncer, so this module only classifies, filters
//! and forwards typed events.
//!
//! No status detection here: enabled/disabled derives from the folder name
//! during disk reconcile — a rename is just a rename.
//!
//! # Covers: EC-2.06 (Watcher Suppression), TC-2.4-02

use crate::shared::errors::ScannerError;
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod event_filter;
mod events;
mod suppressor;

pub(crate) use event_filter::{should_keep_event_path, should_keep_structural_event_path};
pub use events::{ModWatchEvent, WatchEventPayload};
pub use suppressor::{PathSuppressionGuard, SuppressionGuard, WatcherSession, WatcherSuppressor};

/// One debounce window: long enough to stitch a Windows From/To rename pair
/// and coalesce a burst, short enough to feel immediate in the UI.
const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(500);
const WATCH_EVENT_BUFFER_CAPACITY: usize = 4096;
const WATCH_EVENT_OVERFLOW_ERROR: &str =
    "Watcher event buffer overflowed; a full disk reconcile is required";
const WATCH_BACKEND_RESCAN_REQUIRED: &str =
    "Watcher backend reported lost events; a full disk reconcile is required";

pub type ModWatcher = Debouncer<RecommendedWatcher, RecommendedCache>;

pub struct WatchEventReceiver {
    receiver: tokio::sync::mpsc::Receiver<ModWatchEvent>,
    overflowed: Arc<std::sync::atomic::AtomicBool>,
    pending: Option<ModWatchEvent>,
}

impl WatchEventReceiver {
    async fn recv(&mut self) -> Option<ModWatchEvent> {
        if let Some(event) = self.pending.take() {
            return Some(event);
        }

        let event = self.receiver.recv().await?;
        if self.overflowed.swap(false, Ordering::AcqRel) {
            self.pending = Some(event);
            return Some(ModWatchEvent::Error(WATCH_EVENT_OVERFLOW_ERROR.to_string()));
        }
        Some(event)
    }

    fn try_recv(&mut self) -> Result<ModWatchEvent, tokio::sync::mpsc::error::TryRecvError> {
        if let Some(event) = self.pending.take() {
            return Ok(event);
        }
        if self.overflowed.swap(false, Ordering::AcqRel) {
            return Ok(ModWatchEvent::Error(WATCH_EVENT_OVERFLOW_ERROR.to_string()));
        }
        self.receiver.try_recv()
    }
}

// ── Managed State ─────────────────────────────────────────────────────

/// Managed state for the watcher, accessible via Tauri commands.
pub struct WatcherState {
    pub suppressor: Arc<WatcherSuppressor>,
    pub watcher: std::sync::Mutex<Option<ModWatcher>>,
    session_generation: AtomicU64,
    current_session: Mutex<Option<WatcherSession>>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self {
            suppressor: Arc::new(WatcherSuppressor::new(false)),
            watcher: std::sync::Mutex::new(None),
            session_generation: AtomicU64::new(0),
            current_session: Mutex::new(None),
        }
    }

    pub fn begin_session(&self, root: &Path) -> WatcherSession {
        let session = self.prepare_session(root);
        self.publish_session(&session);
        session
    }

    pub(crate) fn prepare_session(&self, root: &Path) -> WatcherSession {
        let generation = self.session_generation.fetch_add(1, Ordering::AcqRel) + 1;
        WatcherSession::new(generation, root)
    }

    pub(crate) fn publish_session(&self, session: &WatcherSession) {
        let mut current = crate::shared::sync::lock(&self.current_session);
        self.suppressor.begin_session(session);
        *current = Some(session.clone());
    }

    pub fn invalidate_session(&self) {
        self.session_generation.fetch_add(1, Ordering::AcqRel);
        let mut current = crate::shared::sync::lock(&self.current_session);
        if let Some(session) = current.take() {
            self.suppressor.invalidate_session(&session);
        }
    }

    pub fn is_current_session(&self, session: &WatcherSession) -> bool {
        crate::shared::sync::lock(&self.current_session).as_ref() == Some(session)
    }

    /// Executes a synchronous publication while proving that its watcher
    /// session is still active. The session mutex makes the check and publish
    /// one critical section, so replacement cannot slip between them.
    pub(crate) fn with_current_session<T>(
        &self,
        session: &WatcherSession,
        publish: impl FnOnce() -> T,
    ) -> Option<T> {
        let current = crate::shared::sync::lock(&self.current_session);
        (current.as_ref() == Some(session)).then(publish)
    }
}

impl Default for WatcherState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Event classification ──────────────────────────────────────────────

/// A rename side that survives filtering and suppression.
fn is_runtime_config_path(path: &Path, runtime_config_path: Option<&Path>) -> bool {
    runtime_config_path.is_some_and(|expected| {
        path.to_string_lossy()
            .eq_ignore_ascii_case(&expected.to_string_lossy())
    })
}

fn keep_side(
    path: &Path,
    watcher_path: &Path,
    runtime_config_path: Option<&Path>,
    suppressor: &WatcherSuppressor,
) -> bool {
    (should_keep_event_path(path, watcher_path)
        || is_runtime_config_path(path, runtime_config_path))
        && !suppressor.is_path_suppressed(path)
}

fn keep_structural_side(
    path: &Path,
    watcher_path: &Path,
    runtime_config_path: Option<&Path>,
    suppressor: &WatcherSuppressor,
) -> bool {
    (should_keep_structural_event_path(path, watcher_path)
        || is_runtime_config_path(path, runtime_config_path))
        && !suppressor.is_path_suppressed(path)
}

fn classify_event_with_runtime_config(
    event: &Event,
    watcher_path: &Path,
    runtime_config_path: Option<&Path>,
    suppressor: &WatcherSuppressor,
    session: &WatcherSession,
    send: &impl Fn(ModWatchEvent),
) {
    // Blanket suppression (broad ops + frontend manual flag)
    if suppressor.load(Ordering::Acquire) {
        suppressor.mark_blanket_event_dropped(session);
        return;
    }

    // notify v8 emits a pathless Rescan flag when the platform backend loses
    // events. Scoped paths can no longer be trusted, so lifecycle performs a
    // full reconcile for the session.
    if event.need_rescan() {
        send(ModWatchEvent::Error(
            WATCH_BACKEND_RESCAN_REQUIRED.to_string(),
        ));
        return;
    }

    let path_str = |p: &Path| p.to_string_lossy().to_string();

    match event.kind {
        // Stitched rename: paths = [from, to]. Judge each side on its own —
        // a rename into or out of relevance degrades to Created/Removed.
        EventKind::Modify(ModifyKind::Name(RenameMode::Both | RenameMode::Any))
            if event.paths.len() >= 2 =>
        {
            let from = &event.paths[0];
            let to = &event.paths[1];
            match (
                keep_structural_side(from, watcher_path, runtime_config_path, suppressor),
                keep_structural_side(to, watcher_path, runtime_config_path, suppressor),
            ) {
                (true, true) => send(ModWatchEvent::Renamed {
                    from: path_str(from),
                    to: path_str(to),
                }),
                (true, false) => send(ModWatchEvent::Removed(path_str(from))),
                (false, true) => send(ModWatchEvent::Created(path_str(to))),
                (false, false) => {}
            }
        }

        // Unstitched halves (the counterpart never arrived in the window).
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
            for p in &event.paths {
                if keep_structural_side(p, watcher_path, runtime_config_path, suppressor) {
                    send(ModWatchEvent::Removed(path_str(p)));
                }
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            for p in &event.paths {
                if keep_structural_side(p, watcher_path, runtime_config_path, suppressor) {
                    send(ModWatchEvent::Created(path_str(p)));
                }
            }
        }
        EventKind::Modify(ModifyKind::Name(_)) => {}

        EventKind::Create(_) => {
            for p in &event.paths {
                if keep_side(p, watcher_path, runtime_config_path, suppressor) {
                    send(ModWatchEvent::Created(path_str(p)));
                }
            }
        }
        EventKind::Modify(_) => {
            for p in &event.paths {
                if keep_side(p, watcher_path, runtime_config_path, suppressor) {
                    send(ModWatchEvent::Modified(path_str(p)));
                }
            }
        }
        EventKind::Remove(_) => {
            for p in &event.paths {
                if keep_structural_side(p, watcher_path, runtime_config_path, suppressor) {
                    send(ModWatchEvent::Removed(path_str(p)));
                }
            }
        }

        // Access, Other, etc.
        _ => {}
    }
}

#[cfg(test)]
fn classify_event(
    event: &Event,
    watcher_path: &Path,
    suppressor: &WatcherSuppressor,
    session: &WatcherSession,
    send: &impl Fn(ModWatchEvent),
) {
    classify_event_with_runtime_config(event, watcher_path, None, suppressor, session, send);
}

// ── Watcher Factory ───────────────────────────────────────────────────

/// Create a debounced file watcher on a mod directory with suppression
/// support. Returns `(Debouncer handle, tokio Receiver)`.
///
/// # Covers: EC-2.06 (Watcher Suppression), TC-2.4-02
pub fn watch_mod_directory(
    path: &Path,
    is_suppressed: Arc<WatcherSuppressor>,
    session: WatcherSession,
) -> Result<(ModWatcher, WatchEventReceiver), ScannerError> {
    watch_mod_directory_with_runtime_config(path, None, is_suppressed, session)
}

/// Watch the effective Mods root recursively plus the importer configuration
/// file non-recursively. Only the exact config file passes classification, so
/// an importer root does not turn into a second asset watcher.
pub fn watch_mod_directory_with_runtime_config(
    path: &Path,
    runtime_config_path: Option<&Path>,
    is_suppressed: Arc<WatcherSuppressor>,
    session: WatcherSession,
) -> Result<(ModWatcher, WatchEventReceiver), ScannerError> {
    if !path.exists() || !path.is_dir() {
        return Err(ScannerError::Validation(format!(
            "Watch target does not exist: {}",
            path.display()
        )));
    }

    let (tx, rx) = tokio::sync::mpsc::channel(WATCH_EVENT_BUFFER_CAPACITY);
    let overflowed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let callback_overflowed = overflowed.clone();
    let watcher_path = path.to_path_buf();
    let runtime_config_path = runtime_config_path.map(Path::to_path_buf);
    let callback_runtime_config_path = runtime_config_path.clone();

    let mut debouncer = notify_debouncer_full::new_debouncer(
        DEBOUNCE_TIMEOUT,
        None,
        move |result: DebounceEventResult| {
            let send = |ev: ModWatchEvent| {
                if let Err(tokio::sync::mpsc::error::TrySendError::Full(_)) = tx.try_send(ev) {
                    callback_overflowed.store(true, Ordering::Release);
                }
            };
            match result {
                Ok(events) => {
                    for debounced in &events {
                        classify_event_with_runtime_config(
                            &debounced.event,
                            &watcher_path,
                            callback_runtime_config_path.as_deref(),
                            &is_suppressed,
                            &session,
                            &send,
                        );
                    }
                }
                Err(errors) => {
                    for error in errors {
                        send(ModWatchEvent::Error(error.to_string()));
                    }
                }
            }
        },
    )?;

    debouncer.watch(path, RecursiveMode::Recursive)?;
    if let Some(runtime_config_path) = runtime_config_path {
        if !runtime_config_path.starts_with(path) {
            let parent = runtime_config_path.parent().ok_or_else(|| {
                ScannerError::Validation(format!(
                    "Runtime config has no parent directory: {}",
                    runtime_config_path.display()
                ))
            })?;
            if parent.is_dir() {
                debouncer.watch(parent, RecursiveMode::NonRecursive)?;
            }
        }
    }

    Ok((
        debouncer,
        WatchEventReceiver {
            receiver: rx,
            overflowed,
            pending: None,
        },
    ))
}

pub mod lifecycle;

#[cfg(test)]
#[path = "../../scanner/tests/watcher_tests.rs"]
mod tests;
