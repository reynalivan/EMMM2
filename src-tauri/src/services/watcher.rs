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

/// Translate a raw notify Event into our domain ModWatchEvents.
fn translate_event(event: Event) -> Vec<ModWatchEvent> {
    let mut results = Vec::new();

    let paths: Vec<String> = event
        .paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    match event.kind {
        EventKind::Create(_) => {
            for p in paths {
                results.push(ModWatchEvent::Created(p));
            }
        }
        EventKind::Modify(_) => {
            for p in paths {
                results.push(ModWatchEvent::Modified(p));
            }
        }
        EventKind::Remove(_) => {
            for p in paths {
                results.push(ModWatchEvent::Removed(p));
            }
        }
        _ => {} // Ignore Access, Other, etc.
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Covers: TC-2.4-02 — Watcher receives create event
    #[test]
    fn test_watcher_detects_file_creation() {
        let dir = TempDir::new().unwrap();
        let suppressed = Arc::new(AtomicBool::new(false));
        let (watcher, rx) = watch_mod_directory(dir.path(), suppressed).unwrap();

        // Give watcher time to initialize
        std::thread::sleep(Duration::from_millis(200));

        // Create a file
        fs::write(dir.path().join("new_mod.ini"), "content").unwrap();

        // Wait for event with timeout
        let mut received = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            if let Ok(event) = rx.recv_timeout(Duration::from_millis(100)) {
                if matches!(event, ModWatchEvent::Created(_)) {
                    received = true;
                    break;
                }
            }
        }

        assert!(received, "Expected to receive a Created event");
        drop(watcher);
    }

    // Covers: EC-2.06 (Watcher Suppression)
    #[test]
    fn test_watcher_suppression() {
        let dir = TempDir::new().unwrap();
        // Start suppressed
        let suppressed = Arc::new(AtomicBool::new(true));
        let (watcher, rx) = watch_mod_directory(dir.path(), suppressed.clone()).unwrap();

        std::thread::sleep(Duration::from_millis(200));

        // Create file while suppressed
        fs::write(dir.path().join("ignored_mod.ini"), "content").unwrap();

        // Should NOT receive event within reasonable time
        // We use a shorter timeout because we expect NOTHING
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        let mut unexpected_event = false;

        while std::time::Instant::now() < deadline {
            if let Ok(event) = rx.recv_timeout(Duration::from_millis(100)) {
                if matches!(event, ModWatchEvent::Created(_)) {
                    unexpected_event = true;
                    break;
                }
            }
        }

        // This assertion should FAIL in Red phase because we haven't implemented suppression logic
        assert!(
            !unexpected_event,
            "Received event while suppressed! (Expected Failure in Red Phase)"
        );

        // Now Unsuppress
        suppressed.store(false, Ordering::Relaxed);

        // Create another file
        fs::write(dir.path().join("detected_mod.ini"), "content").unwrap();

        // Should receive THIS event
        let mut received = false;
        let deadline2 = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < deadline2 {
            if let Ok(event) = rx.recv_timeout(Duration::from_millis(100)) {
                if let ModWatchEvent::Created(path) = event {
                    if path.contains("detected_mod.ini") {
                        received = true;
                        break;
                    }
                }
            }
        }

        assert!(received, "Did not receive event after unsuppressing");

        drop(watcher);
    }

    #[test]
    fn test_watcher_nonexistent_path() {
        let suppressed = Arc::new(AtomicBool::new(false));
        let result = watch_mod_directory(Path::new("/nonexistent/path"), suppressed);
        assert!(result.is_err());
    }
}
