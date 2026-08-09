//! File system watcher for mod directories.
//!
//! Uses `notify-debouncer-full` over the `notify` v7 recommended watcher:
//! debouncing, event dedup and rename From/To stitching (via Windows file
//! IDs) all happen in the debouncer, so this module only classifies, filters
//! and forwards typed events.
//!
//! No status detection here: enabled/disabled derives from the folder name
//! during disk reconcile — a rename is just a rename.
//!
//! # Covers: EC-2.06 (Watcher Suppression), TC-2.4-02

use crate::domain::errors::ScannerError;
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

mod event_filter;
mod events;
mod suppressor;

pub(crate) use event_filter::should_keep_event_path;
pub use events::{ModWatchEvent, WatchEventPayload};
pub use suppressor::{PathSuppressionGuard, SuppressionGuard, WatcherSuppressor};

/// One debounce window: long enough to stitch a Windows From/To rename pair
/// and coalesce a burst, short enough to feel immediate in the UI.
const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(500);

pub type ModWatcher = Debouncer<RecommendedWatcher, RecommendedCache>;

// ── Managed State ─────────────────────────────────────────────────────

/// Managed state for the watcher, accessible via Tauri commands.
pub struct WatcherState {
    pub suppressor: Arc<WatcherSuppressor>,
    pub watcher: std::sync::Mutex<Option<ModWatcher>>,
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

// ── Event classification ──────────────────────────────────────────────

/// A rename side that survives filtering and suppression.
fn keep_side(path: &Path, watcher_path: &Path, suppressor: &WatcherSuppressor) -> bool {
    should_keep_event_path(path, watcher_path) && !suppressor.is_path_suppressed(path)
}

fn classify_event(
    event: &Event,
    watcher_path: &Path,
    suppressor: &WatcherSuppressor,
    send: &impl Fn(ModWatchEvent),
) {
    // Blanket suppression (broad ops + frontend manual flag)
    if suppressor.load(Ordering::Acquire) {
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
                keep_side(from, watcher_path, suppressor),
                keep_side(to, watcher_path, suppressor),
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
                if keep_side(p, watcher_path, suppressor) {
                    send(ModWatchEvent::Removed(path_str(p)));
                }
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            for p in &event.paths {
                if keep_side(p, watcher_path, suppressor) {
                    send(ModWatchEvent::Created(path_str(p)));
                }
            }
        }
        EventKind::Modify(ModifyKind::Name(_)) => {}

        EventKind::Create(_) => {
            for p in &event.paths {
                if keep_side(p, watcher_path, suppressor) {
                    send(ModWatchEvent::Created(path_str(p)));
                }
            }
        }
        EventKind::Modify(_) => {
            for p in &event.paths {
                if keep_side(p, watcher_path, suppressor) {
                    send(ModWatchEvent::Modified(path_str(p)));
                }
            }
        }
        EventKind::Remove(_) => {
            for p in &event.paths {
                if keep_side(p, watcher_path, suppressor) {
                    send(ModWatchEvent::Removed(path_str(p)));
                }
            }
        }

        // Access, Other, etc.
        _ => {}
    }
}

// ── Watcher Factory ───────────────────────────────────────────────────

/// Create a debounced file watcher on a mod directory with suppression
/// support. Returns `(Debouncer handle, tokio Receiver)`.
///
/// # Covers: EC-2.06 (Watcher Suppression), TC-2.4-02
pub fn watch_mod_directory(
    path: &Path,
    is_suppressed: Arc<WatcherSuppressor>,
) -> Result<
    (
        ModWatcher,
        tokio::sync::mpsc::UnboundedReceiver<ModWatchEvent>,
    ),
    ScannerError,
> {
    if !path.exists() || !path.is_dir() {
        return Err(ScannerError::Validation(format!(
            "Watch target does not exist: {}",
            path.display()
        )));
    }

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let watcher_path = path.to_path_buf();

    let mut debouncer = notify_debouncer_full::new_debouncer(
        DEBOUNCE_TIMEOUT,
        None,
        move |result: DebounceEventResult| {
            let send = |ev: ModWatchEvent| {
                let _ = tx.send(ev);
            };
            match result {
                Ok(events) => {
                    for debounced in &events {
                        classify_event(&debounced.event, &watcher_path, &is_suppressed, &send);
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

    Ok((debouncer, rx))
}

pub mod lifecycle;

#[cfg(test)]
#[path = "../../scanner/tests/watcher_tests.rs"]
mod tests;
