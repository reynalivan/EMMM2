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
        if let Ok(ModWatchEvent::Created(path)) = rx.recv_timeout(Duration::from_millis(100)) {
            if path.contains("detected_mod.ini") {
                received = true;
                break;
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
