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

use crate::modules::workspace::adapters::notify::identity_cache::WatcherIdentityCache;
use crate::shared::errors::ScannerError;
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer_opt, DebounceEventResult, Debouncer};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod activation_snapshot;
mod event_filter;
mod events;
mod registration;
mod suppressor;

pub(crate) use activation_snapshot::onboarding_snapshot_requires_repair;
#[cfg(test)]
use event_filter::is_runtime_config_path;
#[cfg(test)]
pub(crate) use event_filter::{should_keep_event_path, should_keep_structural_event_path};
use event_filter::{DirectoryFact, EventPolicy, WatcherCoverage};
pub use events::{ModWatchEvent, WatchEventPayload};
pub(crate) use registration::register_watcher_candidate;
pub(crate) use suppressor::ExpectedRenameEcho;
pub use suppressor::{PathSuppressionGuard, SuppressionGuard, WatcherSession, WatcherSuppressor};

/// One debounce window: long enough to stitch a Windows From/To rename pair
/// and coalesce a burst, short enough to feel immediate in the UI.
const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(500);
const WATCH_EVENT_BUFFER_CAPACITY: usize = 4096;
const WATCH_EVENT_OVERFLOW_ERROR: &str =
    "Watcher event buffer overflowed; a full disk reconcile is required";
const WATCH_BACKEND_RESCAN_REQUIRED: &str =
    "Watcher backend reported lost events; a full disk reconcile is required";
#[cfg(windows)]
const WATCH_IDENTITY_CACHE_FAILED: &str =
    "Watcher directory identities could not be maintained; retry indexing to verify disk state";

pub type ModWatcher = Debouncer<RecommendedWatcher, WatcherIdentityCache>;
pub(crate) type WatchObservation =
    Arc<dyn Fn(&[std::path::PathBuf], Option<&EventKind>, bool) + Send + Sync>;

pub struct WatchEventReceiver {
    receiver: tokio::sync::mpsc::Receiver<ModWatchEvent>,
    overflowed: Arc<std::sync::atomic::AtomicBool>,
    pending: Option<ModWatchEvent>,
}

impl WatchEventReceiver {
    pub(crate) fn is_closed(&self) -> bool {
        self.receiver.is_closed()
    }

    pub(crate) async fn recv(&mut self) -> Option<ModWatchEvent> {
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

    pub(crate) fn try_recv(
        &mut self,
    ) -> Result<ModWatchEvent, tokio::sync::mpsc::error::TryRecvError> {
        if let Some(event) = self.pending.take() {
            return Ok(event);
        }
        if self.overflowed.swap(false, Ordering::AcqRel) {
            return Ok(ModWatchEvent::Error(WATCH_EVENT_OVERFLOW_ERROR.to_string()));
        }
        self.receiver.try_recv()
    }

    #[cfg(test)]
    pub(crate) fn test_channel() -> (tokio::sync::mpsc::Sender<ModWatchEvent>, Self) {
        let (sender, receiver) = tokio::sync::mpsc::channel(WATCH_EVENT_BUFFER_CAPACITY);
        (
            sender,
            Self {
                receiver,
                overflowed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                pending: None,
            },
        )
    }
}

// ── Managed State ─────────────────────────────────────────────────────

/// Managed state for the watcher, accessible via Tauri commands.
pub struct WatcherState {
    pub suppressor: Arc<WatcherSuppressor>,
    pub watcher: std::sync::Mutex<Option<ModWatcher>>,
    session_generation: AtomicU64,
    current_session: Mutex<Option<WatcherSession>>,
    inactive_watchers: Mutex<HashMap<String, InactiveWatcher>>,
}

struct InactiveWatcher {
    binding: suppressor::WatcherBinding,
    session: WatcherSession,
    watcher: ModWatcher,
    receiver: WatchEventReceiver,
}

impl InactiveWatcher {
    fn matches_binding(&self, evidence: &suppressor::WatcherBinding) -> bool {
        self.binding.matches_coverage(evidence)
            && self.session.matches_binding_evidence(evidence)
            && !self.receiver.is_closed()
    }
}

impl WatcherState {
    pub fn new() -> Self {
        Self {
            suppressor: Arc::new(WatcherSuppressor::new(false)),
            watcher: std::sync::Mutex::new(None),
            session_generation: AtomicU64::new(0),
            current_session: Mutex::new(None),
            inactive_watchers: Mutex::new(HashMap::new()),
        }
    }

    pub fn begin_session(&self, root: &Path) -> WatcherSession {
        let session = self.prepare_session(root);
        self.publish_session(&session);
        session
    }

    pub(crate) fn prepare_session(&self, root: &Path) -> WatcherSession {
        self.prepare_session_with_runtime_config(root, None)
    }

    pub(crate) fn prepare_session_with_runtime_config(
        &self,
        root: &Path,
        runtime_config_path: Option<&Path>,
    ) -> WatcherSession {
        let generation = self.session_generation.fetch_add(1, Ordering::AcqRel) + 1;
        WatcherSession::new_with_runtime_config(generation, root, runtime_config_path)
    }

    pub(crate) fn publish_session(&self, session: &WatcherSession) {
        let mut current = crate::shared::sync::lock(&self.current_session);
        self.suppressor.begin_session(session);
        *current = Some(session.clone());
    }

    pub(crate) fn publish_registered_session(&self, session: &WatcherSession) -> bool {
        let mut current = crate::shared::sync::lock(&self.current_session);
        if !self.registration_generation_is_current(session) {
            return false;
        }
        self.suppressor.begin_session(session);
        *current = Some(session.clone());
        true
    }

    pub fn invalidate_session(&self) {
        let mut current = crate::shared::sync::lock(&self.current_session);
        self.session_generation.fetch_add(1, Ordering::AcqRel);
        if let Some(session) = current.take() {
            self.suppressor.invalidate_session(&session);
        }
    }

    pub fn is_current_session(&self, session: &WatcherSession) -> bool {
        crate::shared::sync::lock(&self.current_session).as_ref() == Some(session)
    }

    pub(crate) fn registration_generation_is_current(&self, session: &WatcherSession) -> bool {
        self.session_generation.load(Ordering::Acquire) == session.generation()
    }

    pub(crate) fn current_session_for_root(&self, root: &Path) -> Option<WatcherSession> {
        crate::shared::sync::lock(&self.current_session)
            .as_ref()
            .filter(|session| session.covers_root(root))
            .cloned()
    }

    pub(crate) fn current_session_for_coverage(
        &self,
        root: &Path,
        runtime_config_path: Option<&Path>,
    ) -> Option<WatcherSession> {
        crate::shared::sync::lock(&self.current_session)
            .as_ref()
            .filter(|session| session.covers(root, runtime_config_path))
            .cloned()
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

    pub(crate) fn inactive_watcher_session(
        &self,
        game_id: &str,
        root: &Path,
        runtime_config_path: Option<&Path>,
    ) -> Option<u64> {
        self.inactive_session_for_coverage(game_id, root, runtime_config_path)
            .map(|session| session.generation())
    }

    pub(crate) fn inactive_session_for_coverage(
        &self,
        game_id: &str,
        root: &Path,
        runtime_config_path: Option<&Path>,
    ) -> Option<WatcherSession> {
        let evidence = suppressor::WatcherBinding::capture(root, runtime_config_path);
        crate::shared::sync::lock(&self.inactive_watchers)
            .get(game_id)
            .filter(|watcher| watcher.matches_binding(&evidence))
            .map(|watcher| watcher.session.clone())
    }

    pub(crate) fn install_inactive_watcher(
        &self,
        game_id: String,
        root: &Path,
        runtime_config_path: Option<&Path>,
        session: WatcherSession,
        watcher: ModWatcher,
        receiver: WatchEventReceiver,
    ) {
        crate::shared::sync::lock(&self.inactive_watchers).insert(
            game_id,
            InactiveWatcher {
                binding: suppressor::WatcherBinding::capture(root, runtime_config_path),
                session,
                watcher,
                receiver,
            },
        );
    }

    pub(crate) fn take_inactive_watcher_for_handoff(
        &self,
        game_id: &str,
        root: &Path,
        runtime_config_path: Option<&Path>,
    ) -> Option<(WatcherSession, ModWatcher, WatchEventReceiver)> {
        let evidence = suppressor::WatcherBinding::capture(root, runtime_config_path);
        let mut watchers = crate::shared::sync::lock(&self.inactive_watchers);
        if watchers
            .get(game_id)
            .is_none_or(|watcher| !watcher.matches_binding(&evidence))
        {
            return None;
        }
        let watcher = watchers.remove(game_id)?;
        Some((watcher.session, watcher.watcher, watcher.receiver))
    }

    pub(crate) fn discard_inactive_watcher_unless_coverage(
        &self,
        game_id: &str,
        root: &Path,
        runtime_config_path: Option<&Path>,
    ) -> bool {
        let evidence = suppressor::WatcherBinding::capture(root, runtime_config_path);
        let mut watchers = crate::shared::sync::lock(&self.inactive_watchers);
        let mismatched = watchers
            .get(game_id)
            .is_some_and(|watcher| !watcher.matches_binding(&evidence));
        if mismatched {
            watchers.remove(game_id);
        }
        mismatched
    }
}

impl Default for WatcherState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Event classification ──────────────────────────────────────────────

#[cfg(test)]
fn classify_event_with_runtime_config(
    event: &Event,
    watcher_path: &Path,
    runtime_config_path: Option<&Path>,
    suppressor: &WatcherSuppressor,
    session: &WatcherSession,
    send: &impl Fn(ModWatchEvent),
) {
    let policy = WatcherCoverage::new(watcher_path, runtime_config_path).policy(event);
    classify_event_with_policy(event, &policy, suppressor, session, send);
}

fn classify_event_with_policy(
    event: &Event,
    policy: &EventPolicy,
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

    if suppressor.is_committed_rename_echo(session, &event.kind, &event.paths) {
        return;
    }

    let path_str = |p: &Path| p.to_string_lossy().to_string();
    let keep_structural = |index: usize| {
        policy.endpoints[index].structural && !suppressor.is_path_suppressed(&event.paths[index])
    };
    let keep_content = |index: usize| {
        let path = &event.paths[index];
        let endpoint = policy.endpoints[index];
        !suppressor.is_path_suppressed(path)
            && endpoint.keeps_content(
                endpoint
                    .needs_directory_fact
                    .then(|| DirectoryFact::capture(path)),
            )
    };

    match event.kind {
        // Stitched rename: paths = [from, to]. Judge each side on its own —
        // a rename into or out of relevance degrades to Created/Removed.
        EventKind::Modify(ModifyKind::Name(RenameMode::Both | RenameMode::Any))
            if event.paths.len() >= 2 =>
        {
            let from = &event.paths[0];
            let to = &event.paths[1];
            match (keep_structural(0), keep_structural(1)) {
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
            for (index, p) in event.paths.iter().enumerate() {
                if keep_structural(index) {
                    send(ModWatchEvent::Removed(path_str(p)));
                }
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            for (index, p) in event.paths.iter().enumerate() {
                if keep_structural(index) {
                    send(ModWatchEvent::Created(path_str(p)));
                }
            }
        }
        // A one-path `Any` rename is ambiguous: notify could not stitch its
        // counterpart, so treating it as a no-op would leave authority dirty
        // while lifecycle waits forever for another filesystem event. Request
        // a full reconcile unless this path is intentionally suppressed.
        EventKind::Modify(ModifyKind::Name(RenameMode::Any)) => {
            if (0..event.paths.len()).any(keep_structural) {
                send(ModWatchEvent::Error(
                    WATCH_BACKEND_RESCAN_REQUIRED.to_string(),
                ));
            }
        }
        EventKind::Modify(ModifyKind::Name(_)) => {}

        EventKind::Create(_) => {
            for (index, p) in event.paths.iter().enumerate() {
                if keep_content(index) {
                    send(ModWatchEvent::Created(path_str(p)));
                }
            }
        }
        EventKind::Modify(_) => {
            for (index, p) in event.paths.iter().enumerate() {
                if keep_content(index) {
                    send(ModWatchEvent::Modified(path_str(p)));
                }
            }
        }
        EventKind::Remove(_) => {
            for (index, p) in event.paths.iter().enumerate() {
                if keep_structural(index) {
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
    watch_mod_directory_with_runtime_config_and_observer(
        path,
        runtime_config_path,
        is_suppressed,
        session,
        None,
    )
}

pub(crate) fn watch_mod_directory_with_runtime_config_and_observer(
    path: &Path,
    runtime_config_path: Option<&Path>,
    is_suppressed: Arc<WatcherSuppressor>,
    session: WatcherSession,
    observer: Option<WatchObservation>,
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
    let runtime_config_path = runtime_config_path.map(Path::to_path_buf);
    let coverage = WatcherCoverage::new(path, runtime_config_path.as_deref());

    let runtime_parent = runtime_config_path
        .as_deref()
        .filter(|config| !config.starts_with(path))
        .map(|config| {
            config.parent().ok_or_else(|| {
                ScannerError::Validation("Runtime config has no parent directory".to_string())
            })
        })
        .transpose()?
        .filter(|parent| parent.is_dir());
    #[cfg(windows)]
    let cache = WatcherIdentityCache::for_roots(
        std::iter::once((path.to_path_buf(), RecursiveMode::Recursive))
            .chain(runtime_parent.map(|parent| (parent.to_path_buf(), RecursiveMode::NonRecursive)))
            .collect(),
        session.identity_cache_validity(),
    );
    #[cfg(not(windows))]
    let cache = WatcherIdentityCache::new();
    #[cfg(windows)]
    let diagnostics = cache.diagnostics();
    #[cfg(windows)]
    let callback_diagnostics = Arc::clone(&diagnostics);

    #[cfg(windows)]
    let registration_started = std::time::Instant::now();
    #[cfg(windows)]
    let watcher_generation = session.generation();
    let mut debouncer = new_debouncer_opt::<_, RecommendedWatcher, WatcherIdentityCache>(
        DEBOUNCE_TIMEOUT,
        None,
        move |result: DebounceEventResult| {
            let send = |ev: ModWatchEvent| {
                if let Err(tokio::sync::mpsc::error::TrySendError::Full(_)) = tx.try_send(ev) {
                    callback_overflowed.store(true, Ordering::Release);
                }
            };
            #[cfg(windows)]
            if let Some(kind) = callback_diagnostics.take_failure() {
                if let Some(observer) = observer.as_ref() {
                    observer(&[], None, true);
                }
                send(ModWatchEvent::Error(format!(
                    "{WATCH_IDENTITY_CACHE_FAILED}: {kind:?}"
                )));
            }
            match result {
                Ok(events) => {
                    for debounced in &events {
                        let policy = coverage.policy(&debounced.event);
                        if !policy.authority_relevant {
                            continue;
                        }
                        if let Some(observer) = observer.as_ref() {
                            observer(
                                &debounced.event.paths,
                                Some(&debounced.event.kind),
                                debounced.event.need_rescan(),
                            );
                        }
                        let emitted = std::cell::Cell::new(false);
                        classify_event_with_policy(
                            &debounced.event,
                            &policy,
                            &is_suppressed,
                            &session,
                            &|event| {
                                emitted.set(true);
                                send(event);
                            },
                        );
                        if !emitted.get() {
                            session.request_work();
                        }
                    }
                }
                Err(errors) => {
                    if let Some(observer) = observer.as_ref() {
                        observer(&[], None, true);
                    }
                    for error in errors {
                        send(ModWatchEvent::Error(error.to_string()));
                    }
                }
            }
        },
        cache,
        notify::Config::default(),
    )?;

    debouncer.watch(path, RecursiveMode::Recursive)?;
    if let Some(parent) = runtime_parent {
        debouncer.watch(parent, RecursiveMode::NonRecursive)?;
    }
    #[cfg(windows)]
    {
        if let Some(kind) = diagnostics.failure() {
            return Err(ScannerError::Validation(format!(
                "{WATCH_IDENTITY_CACHE_FAILED}: {kind:?}"
            )));
        }
        log::info!("watcher registration session={} policy=directories elapsed_ms={} directory_id_reads={} entries_visited={} cached_directories={}",
            watcher_generation, registration_started.elapsed().as_millis(),
            diagnostics.directory_id_reads.load(Ordering::Relaxed),
            diagnostics.entries_visited.load(Ordering::Relaxed),
            diagnostics.cached_directories.load(Ordering::Relaxed));
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
