//! File system watcher for mod directories.
//!
//! Uses the `notify` crate (v7, recommended watcher) to watch for
//! changes in mod directories, with debouncing to avoid event storms.
//!
//! # Architecture (post-refactor)
//!
//! - **Typed events**: `ModWatchEvent` (internal) → `WatchEventPayload` (Serde IPC)
//! - **Async-first**: `tokio::sync::mpsc` channel, consumed by `tokio::spawn` in lifecycle
//! - **Rename pairing**: Stateful buffer in watcher closure pairs Windows From/To events
//! - **Status detection**: Enable/disable detection happens at event creation, not downstream
//!
//! # Covers: EC-2.06 (Watcher Suppression), TC-2.4-02

use crate::services::path_key::path_file_name_lossy;
use notify::event::{ModifyKind, RenameMode};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ── Domain Events (internal) ──────────────────────────────────────────

/// Internal events produced by the watcher closure.
/// Consumed by the lifecycle event loop for DB sync.
#[derive(Debug, Clone)]
pub enum ModWatchEvent {
    /// A directory was created at depth 1 or 2.
    Created(String),
    /// A file was modified (content change, metadata).
    Modified(String),
    /// A directory was removed at depth 1 or 2.
    Removed(String),
    /// A directory was renamed (paired From→To).
    Renamed { from: String, to: String },
    /// A rename that changed enable/disable status (DISABLED prefix).
    StatusChanged {
        path: String,
        from_status: &'static str,
        to_status: &'static str,
    },
    /// A watcher error.
    Error(String),
}

// ── IPC Payload (Serde-typed, emitted to frontend) ────────────────────

/// Strongly-typed payload emitted to the frontend via `app.emit("mod_watch:event", payload)`.
/// Uses `#[serde(tag = "type")]` for discriminated union matching on the TS side.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WatchEventPayload {
    Created {
        path: String,
    },
    Modified {
        path: String,
    },
    Removed {
        path: String,
    },
    Renamed {
        from: String,
        to: String,
    },
    StatusChanged {
        path: String,
        from_status: String,
        to_status: String,
    },
    Error {
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        retry_count: Option<u32>,
    },
}

impl WatchEventPayload {
    /// Convert an internal `ModWatchEvent` into the IPC payload.
    pub fn from_event(event: &ModWatchEvent) -> Self {
        match event {
            ModWatchEvent::Created(p) => Self::Created { path: p.clone() },
            ModWatchEvent::Modified(p) => Self::Modified { path: p.clone() },
            ModWatchEvent::Removed(p) => Self::Removed { path: p.clone() },
            ModWatchEvent::Renamed { from, to } => Self::Renamed {
                from: from.clone(),
                to: to.clone(),
            },
            ModWatchEvent::StatusChanged {
                path,
                from_status,
                to_status,
            } => Self::StatusChanged {
                path: path.clone(),
                from_status: from_status.to_string(),
                to_status: to_status.to_string(),
            },
            ModWatchEvent::Error(e) => Self::Error {
                error: e.clone(),
                path: None,
                retry_count: None,
            },
        }
    }
}

// ── Managed State ─────────────────────────────────────────────────────

/// Managed state for the watcher, accessible via Tauri commands.
pub struct WatcherState {
    pub suppressor: Arc<WatcherSuppressor>,
    pub watcher: std::sync::Mutex<Option<RecommendedWatcher>>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self {
            suppressor: Arc::new(WatcherSuppressor::new(false)),
            watcher: std::sync::Mutex::new(None),
        }
    }
}

impl Default for WatcherState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WatcherSuppressor {
    depth: AtomicUsize,
}

impl WatcherSuppressor {
    pub fn new(suppressed: bool) -> Self {
        Self {
            depth: AtomicUsize::new(if suppressed { 1 } else { 0 }),
        }
    }

    pub fn load(&self, ordering: Ordering) -> bool {
        self.depth.load(ordering) > 0
    }

    pub fn store(&self, suppressed: bool, ordering: Ordering) {
        if suppressed {
            self.depth.fetch_add(1, ordering);
            return;
        }

        self.depth.store(0, ordering);
    }

    fn increment(&self) {
        self.depth.fetch_add(1, Ordering::AcqRel);
    }

    fn decrement(&self) {
        let _ = self
            .depth
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.saturating_sub(1))
            });
    }
}

/// RAII Guard for watcher suppression.
pub struct SuppressionGuard {
    suppressor: Arc<WatcherSuppressor>,
}

impl SuppressionGuard {
    pub fn new(suppressor: &Arc<WatcherSuppressor>) -> Self {
        suppressor.increment();
        Self {
            suppressor: suppressor.clone(),
        }
    }
}

impl Drop for SuppressionGuard {
    fn drop(&mut self) {
        self.suppressor.decrement();
    }
}

// ── Relevance Filter ──────────────────────────────────────────────────

/// Extensions relevant to the mod manager.
/// Everything else (`.dds`, `.buf`, `.hlsl`, `.blend`, `.dll`) is noise.
const RELEVANT_EXTENSIONS: &[&str] = &["ini", "json", "png", "jpg", "jpeg", "webp"];

/// Check if a path is relevant to the mod manager.
/// Directories (no extension) always pass; files must match the allowlist.
fn is_relevant_path(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("info.json"))
    {
        return true;
    }

    match path.extension().and_then(|e| e.to_str()) {
        None => true, // No extension = likely a directory
        Some(ext) => {
            let lower = ext.to_ascii_lowercase();
            RELEVANT_EXTENSIONS.contains(&lower.as_str())
        }
    }
}

// ── Status Change Detection ───────────────────────────────────────────

/// Detect if a rename represents an enable/disable status change.
/// Returns `Some((old_status, new_status))` if the DISABLED prefix was added/removed.
fn detect_status_change(from_path: &Path, to_path: &Path) -> Option<(&'static str, &'static str)> {
    use crate::services::scanner::core::normalizer;

    let old_name = path_file_name_lossy(from_path)?;
    let new_name = path_file_name_lossy(to_path)?;

    let old_disabled = normalizer::is_disabled_folder(&old_name);
    let new_disabled = normalizer::is_disabled_folder(&new_name);

    if old_disabled != new_disabled {
        let old_status = if old_disabled { "DISABLED" } else { "ENABLED" };
        let new_status = if new_disabled { "DISABLED" } else { "ENABLED" };
        Some((old_status, new_status))
    } else {
        None
    }
}

// ── Watcher Factory ───────────────────────────────────────────────────

/// Create a file watcher on a mod directory with suppression support.
///
/// Returns `(Watcher handle, tokio Receiver)`.
/// The closure handles:
/// - Suppression checks
/// - Rename pair buffering (Windows From/To pairing, 100ms timeout)
/// - Status-change detection (DISABLED prefix)
/// - Relevance filtering (directories + allowlisted extensions)
///
/// # Covers: EC-2.06 (Watcher Suppression), TC-2.4-02
pub fn watch_mod_directory(
    path: &Path,
    is_suppressed: Arc<WatcherSuppressor>,
) -> Result<
    (
        RecommendedWatcher,
        tokio::sync::mpsc::UnboundedReceiver<ModWatchEvent>,
    ),
    String,
> {
    if !path.exists() || !path.is_dir() {
        return Err(format!("Watch target does not exist: {}", path.display()));
    }

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let suppressed_clone = is_suppressed.clone();

    // Stateful rename pair buffer.
    // On Windows, `notify` emits renames as two separate events:
    //   Modify(Name(From)) then Modify(Name(To)).
    // We buffer the `From` path and pair it with the next `To` event.
    // If no `To` arrives within 100ms, the `From` is treated as a Removed event.
    let pending_from: Arc<std::sync::Mutex<Option<(String, Instant)>>> =
        Arc::new(std::sync::Mutex::new(None));
    let pending_clone = pending_from.clone();

    // Helper: send event, ignoring closed-channel errors (watcher shutting down).
    let send = {
        let tx = tx.clone();
        move |ev: ModWatchEvent| {
            let _ = tx.send(ev);
        }
    };

    let watcher_path = path.to_path_buf();

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| match result {
            Ok(mut event) => {
                // Check suppression flag
                if suppressed_clone.load(Ordering::Acquire) {
                    return;
                }

                // Pre-filter events based on depth (Opt-3)
                // We drop events deeper than Mod folder (depth 2) unless they are .ini or images
                event.paths.retain(|p| {
                    if let Ok(rel) = p.strip_prefix(&watcher_path) {
                        let components: Vec<_> = rel.components().collect();
                        let depth = components.len();

                        // Ignore hidden folders/files
                        if components
                            .iter()
                            .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
                        {
                            return false;
                        }

                        if depth > 2 {
                            if p.file_name()
                                .and_then(|value| value.to_str())
                                .is_some_and(|value| value.eq_ignore_ascii_case("info.json"))
                            {
                                return true;
                            }

                            if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                                let ext_lower = ext.to_lowercase();
                                if ext_lower != "ini"
                                    && ext_lower != "json"
                                    && ext_lower != "png"
                                    && ext_lower != "jpg"
                                    && ext_lower != "jpeg"
                                    && ext_lower != "webp"
                                {
                                    return false; // Drop deep structural items
                                }
                            } else {
                                return false; // Drop deep folders or extensionless files
                            }
                        }
                        true
                    } else {
                        false
                    }
                });

                if event.paths.is_empty() {
                    return; // All paths filtered out
                }

                // Flush any expired pending rename (>100ms old)
                {
                    let mut pending = pending_clone.lock().unwrap();
                    if let Some((ref from_path, ts)) = *pending {
                        if ts.elapsed() > Duration::from_millis(100) {
                            if is_relevant_path(Path::new(from_path)) {
                                send(ModWatchEvent::Removed(from_path.clone()));
                            }
                            *pending = None;
                        }
                    }
                }

                match event.kind {
                    // ── Rename: From (buffer it) ──
                    EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                        if let Some(p) = event.paths.first() {
                            let path_str = p.to_string_lossy().to_string();
                            let mut pending = pending_clone.lock().unwrap();
                            // Flush any existing pending From as Removed
                            if let Some((ref old_from, _)) = *pending {
                                if is_relevant_path(Path::new(old_from)) {
                                    send(ModWatchEvent::Removed(old_from.clone()));
                                }
                            }
                            *pending = Some((path_str, Instant::now()));
                        }
                    }

                    // ── Rename: To (pair with buffered From) ──
                    EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                        if let Some(p) = event.paths.first() {
                            let to_path = p.to_string_lossy().to_string();
                            let mut pending = pending_clone.lock().unwrap();
                            if let Some((from_path, _)) = pending.take() {
                                // Paired: check if it's a status change
                                if is_relevant_path(Path::new(&from_path))
                                    || is_relevant_path(Path::new(&to_path))
                                {
                                    let from_p = Path::new(&from_path);
                                    let to_p = Path::new(&to_path);
                                    if let Some((from_status, to_status)) =
                                        detect_status_change(from_p, to_p)
                                    {
                                        send(ModWatchEvent::StatusChanged {
                                            path: to_path,
                                            from_status,
                                            to_status,
                                        });
                                    } else {
                                        send(ModWatchEvent::Renamed {
                                            from: from_path,
                                            to: to_path,
                                        });
                                    }
                                }
                            } else {
                                // Orphan To → treat as Created
                                if is_relevant_path(p) {
                                    send(ModWatchEvent::Created(to_path));
                                }
                            }
                        }
                    }

                    // ── Rename: Both (some OS/backends emit both paths) ──
                    EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                        if event.paths.len() >= 2 {
                            let from = event.paths[0].to_string_lossy().to_string();
                            let to = event.paths[1].to_string_lossy().to_string();
                            if is_relevant_path(&event.paths[0])
                                || is_relevant_path(&event.paths[1])
                            {
                                let from_p = Path::new(&from);
                                let to_p = Path::new(&to);
                                if let Some((from_status, to_status)) =
                                    detect_status_change(from_p, to_p)
                                {
                                    send(ModWatchEvent::StatusChanged {
                                        path: to,
                                        from_status,
                                        to_status,
                                    });
                                } else {
                                    send(ModWatchEvent::Renamed { from, to });
                                }
                            }
                        }
                    }

                    // ── Rename: Any (safety net — should not be reached) ──
                    EventKind::Modify(ModifyKind::Name(_)) => {}

                    // ── Create ──
                    EventKind::Create(_) => {
                        for p in &event.paths {
                            if is_relevant_path(p) {
                                send(ModWatchEvent::Created(p.to_string_lossy().to_string()));
                            }
                        }
                    }

                    // ── Modify (content/metadata, not rename) ──
                    EventKind::Modify(_) => {
                        for p in &event.paths {
                            if is_relevant_path(p) {
                                send(ModWatchEvent::Modified(p.to_string_lossy().to_string()));
                            }
                        }
                    }

                    // ── Remove ──
                    EventKind::Remove(_) => {
                        for p in &event.paths {
                            if is_relevant_path(p) {
                                send(ModWatchEvent::Removed(p.to_string_lossy().to_string()));
                            }
                        }
                    }

                    // ── Ignored event kinds (Access, Other, etc.) ──
                    _ => {}
                }
            }
            Err(e) => {
                let _ = tx.send(ModWatchEvent::Error(e.to_string()));
            }
        },
        Config::default().with_poll_interval(Duration::from_millis(500)),
    )
    .map_err(|e| format!("Failed to create watcher: {e}"))?;

    watcher
        .watch(path, RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch path: {e}"))?;

    Ok((watcher, rx))
}

pub mod lifecycle;

#[cfg(test)]
#[path = "../../scanner/tests/watcher_tests.rs"]
mod tests;
