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
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod event_filter;
mod events;
mod suppressor;

pub(crate) use event_filter::should_keep_event_path;
use event_filter::{is_relevant_path, RENAME_PAIR_TIMEOUT};
pub use events::{ModWatchEvent, WatchEventPayload};
pub use suppressor::{SuppressionGuard, WatcherSuppressor};

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

                event
                    .paths
                    .retain(|p| should_keep_event_path(p, &watcher_path));

                if event.paths.is_empty() {
                    return; // All paths filtered out
                }

                // Flush any expired pending rename (>100ms old)
                {
                    let mut pending = pending_clone.lock().unwrap();
                    if let Some((ref from_path, ts)) = *pending {
                        if ts.elapsed() > RENAME_PAIR_TIMEOUT {
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
                            *pending = Some((path_str.clone(), Instant::now()));
                            let pending_for_timeout = pending_clone.clone();
                            let tx_for_timeout = tx.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(RENAME_PAIR_TIMEOUT);
                                let mut pending = pending_for_timeout.lock().unwrap();
                                let Some((pending_path, pending_at)) = pending.as_ref() else {
                                    return;
                                };
                                if pending_path != &path_str
                                    || pending_at.elapsed() < RENAME_PAIR_TIMEOUT
                                {
                                    return;
                                }

                                let removed_path = pending_path.clone();
                                *pending = None;
                                if is_relevant_path(Path::new(&removed_path)) {
                                    let _ =
                                        tx_for_timeout.send(ModWatchEvent::Removed(removed_path));
                                }
                            });
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
                                            from: from_path,
                                            to: to_path,
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
                                        from,
                                        to,
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
