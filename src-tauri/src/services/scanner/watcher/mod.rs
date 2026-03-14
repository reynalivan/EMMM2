//! File system watcher for mod directories.
//!
//! Uses the `notify` crate (v7, recommended watcher) to watch for
//! changes in mod directories, with debouncing to avoid event storms.
//!
//! # Covers: EC-2.06 (Watcher Suppression), TC-2.4-02

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

/// Events emitted by the mod folder watcher.
#[derive(Debug, Clone)]
pub enum ModWatchEvent {
    /// A file or folder was created.
    Created(String),
    /// A file or folder was modified.
    Modified(String),
    /// A file or folder was removed.
    Removed(String),
    /// A file or folder was renamed.
    Renamed { from: String, to: String },
    /// A folder's enable/disable status changed (DISABLED prefix added/removed).
    StatusChanged {
        path: String,
        from_status: String,
        to_status: String,
    },
    /// An error occurred during watching.
    Error(String),
}

/// Configuration for the mod folder watcher.
pub struct WatcherConfig {
    /// Debounce duration to combine rapid events.
    pub debounce_ms: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self { debounce_ms: 500 }
    }
}

/// Managed state for the watcher, accessible via Tauri commands.
pub struct WatcherState {
    pub suppressor: Arc<AtomicBool>,
    pub watcher: std::sync::Mutex<Option<RecommendedWatcher>>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self {
            suppressor: Arc::new(AtomicBool::new(false)),
            watcher: std::sync::Mutex::new(None),
        }
    }
}

impl Default for WatcherState {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII Guard for watcher suppression.
/// Sets suppression to TRUE on creation, and FALSE on drop.
pub struct SuppressionGuard {
    suppressor: Arc<AtomicBool>,
}

impl SuppressionGuard {
    pub fn new(suppressor: &Arc<AtomicBool>) -> Self {
        suppressor.store(true, Ordering::Relaxed);
        Self {
            suppressor: suppressor.clone(),
        }
    }
}

impl Drop for SuppressionGuard {
    fn drop(&mut self) {
        self.suppressor.store(false, Ordering::Relaxed);
    }
}

/// Create a file watcher on a mod directory with suppression support.
///
/// Returns a tuple of (Watcher handle, Receiver for events).
/// The watcher runs on a background thread. Drop the handle to stop watching.
///
/// # Covers: EC-2.06 (Watcher Suppression), TC-2.4-02
pub fn watch_mod_directory(
    path: &Path,
    is_suppressed: Arc<AtomicBool>,
) -> Result<(RecommendedWatcher, mpsc::Receiver<ModWatchEvent>), String> {
    if !path.exists() || !path.is_dir() {
        return Err(format!("Watch target does not exist: {}", path.display()));
    }

    let (tx, rx) = mpsc::channel();

    // We clone the Arc to move into the closure
    // Ideally we check this inside the event handler
    let _suppressed_clone = is_suppressed.clone();

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| match result {
            Ok(event) => {
                // Check suppression flag
                if _suppressed_clone.load(Ordering::Relaxed) {
                    return;
                }

                let events = translate_event(event);
                for ev in events {
                    let _ = tx.send(ev);
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

/// Extensions relevant to the mod manager.
/// Everything else (`.dds`, `.buf`, `.hlsl`, `.blend`, `.dll`) is noise.
const RELEVANT_EXTENSIONS: &[&str] = &["ini", "png", "jpg", "jpeg", "webp"];

/// Check if a path is relevant to the mod manager.
/// Directories (no extension) always pass; files must match the allowlist.
fn is_relevant_path(path: &std::path::Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        None => true, // No extension = likely a directory
        Some(ext) => {
            let lower = ext.to_ascii_lowercase();
            RELEVANT_EXTENSIONS.contains(&lower.as_str())
        }
    }
}

/// Translate a raw notify Event into our domain ModWatchEvents.
///
/// Filters: only relevant paths (directories + ini/image files) are forwarded.
/// `Modify` events are now forwarded for status tracking and metadata updates.
fn translate_event(event: Event) -> Vec<ModWatchEvent> {
    let mut results = Vec::new();

    match event.kind {
        EventKind::Create(_) => {
            for p in &event.paths {
                if is_relevant_path(p) {
                    results.push(ModWatchEvent::Created(p.to_string_lossy().to_string()));
                }
            }
        }
        EventKind::Modify(_) => {
            // Forward Modified events for relevant paths
            for p in &event.paths {
                if is_relevant_path(p) {
                    results.push(ModWatchEvent::Modified(p.to_string_lossy().to_string()));
                }
            }
        }
        EventKind::Remove(_) => {
            for p in &event.paths {
                if is_relevant_path(p) {
                    results.push(ModWatchEvent::Removed(p.to_string_lossy().to_string()));
                }
            }
        }
        _ => {} // Ignore Access, Other, etc.
    }

    results
}

pub mod lifecycle;

#[cfg(test)]
#[path = "../../scanner/tests/watcher_tests.rs"]
mod tests;
