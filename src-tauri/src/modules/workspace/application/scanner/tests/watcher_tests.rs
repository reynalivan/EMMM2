use super::*;
use std::fs;
use tempfile::TempDir;

// Covers: TC-2.4-02 — Watcher receives create event
#[tokio::test]
async fn test_watcher_detects_file_creation() {
    let dir = TempDir::new().unwrap();
    let state = WatcherState::new();
    let session = state.begin_session(dir.path());
    let (watcher, mut rx) =
        watch_mod_directory(dir.path(), state.suppressor.clone(), session).unwrap();

    // Give watcher time to initialize
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Create a file
    fs::write(dir.path().join("new_mod.ini"), "content").unwrap();

    // Wait for event with timeout
    let mut received = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
            if matches!(event, ModWatchEvent::Created(_)) {
                received = true;
                break;
            }
        }
    }

    assert!(received, "Expected to receive a Created event");
    drop(watcher);
}

#[tokio::test]
async fn bounded_watcher_receiver_reports_overflow_before_buffered_events() {
    let (tx, receiver) = tokio::sync::mpsc::channel(1);
    let overflowed = Arc::new(std::sync::atomic::AtomicBool::new(true));
    tx.try_send(ModWatchEvent::Created("E:/Mods/Alice".to_string()))
        .expect("seed buffered event");
    let mut receiver = WatchEventReceiver {
        receiver,
        overflowed,
        pending: None,
    };

    assert!(matches!(
        receiver.recv().await,
        Some(ModWatchEvent::Error(_))
    ));
    assert!(matches!(
        receiver.recv().await,
        Some(ModWatchEvent::Created(path)) if path == "E:/Mods/Alice"
    ));
}

// Covers: EC-2.06 (Watcher Suppression)
#[tokio::test]
async fn test_watcher_suppression() {
    let dir = TempDir::new().unwrap();
    let state = WatcherState::new();
    let session = state.begin_session(dir.path());
    let suppressed = state.suppressor.clone();
    let guard = SuppressionGuard::new(&suppressed);
    let (watcher, mut rx) = watch_mod_directory(dir.path(), suppressed.clone(), session).unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Create file while suppressed
    fs::write(dir.path().join("ignored_mod.ini"), "content").unwrap();

    // Should NOT receive event within reasonable time
    // We use a shorter timeout because we expect NOTHING
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let mut unexpected_event = false;

    while std::time::Instant::now() < deadline {
        if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
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

    drop(guard);

    // Create another file
    fs::write(dir.path().join("detected_mod.ini"), "content").unwrap();

    // Should receive THIS event
    let mut received = false;
    let deadline2 = std::time::Instant::now() + Duration::from_secs(3);
    while std::time::Instant::now() < deadline2 {
        if let Ok(Some(ModWatchEvent::Created(path))) =
            tokio::time::timeout(Duration::from_millis(100), rx.recv()).await
        {
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
fn test_nested_suppression_guards_keep_watcher_suppressed_until_last_drop() {
    let state = WatcherState::default();

    assert!(!state.suppressor.load(Ordering::Acquire));

    let first_guard = SuppressionGuard::new(&state.suppressor);
    assert!(state.suppressor.load(Ordering::Acquire));

    {
        let _second_guard = SuppressionGuard::new(&state.suppressor);
        assert!(state.suppressor.load(Ordering::Acquire));
    }

    assert!(state.suppressor.load(Ordering::Acquire));

    drop(first_guard);
    assert!(!state.suppressor.load(Ordering::Acquire));
}

#[test]
fn blanket_suppression_tracks_dropped_events_until_a_generation_is_repaired() {
    let state = WatcherState::new();
    let session = state.begin_session(Path::new(r"E:\Mods"));
    let suppressor = &state.suppressor;
    assert!(!suppressor.has_unrepaired_drops());

    suppressor.mark_blanket_event_dropped(&session);
    let first_generation = suppressor
        .pending_repair(&session)
        .expect("first dirty generation");
    assert!(suppressor.has_unrepaired_drops());

    suppressor.mark_blanket_event_dropped(&session);
    assert!(suppressor.mark_repaired_through(&first_generation));
    assert!(suppressor.has_unrepaired_drops());

    let final_generation = suppressor
        .pending_repair(&session)
        .expect("final dirty generation");
    assert!(suppressor.mark_repaired_through(&final_generation));
    assert!(!suppressor.has_unrepaired_drops());
}

#[test]
fn test_path_scoped_suppression_covers_both_spellings_and_children_only() {
    let state = WatcherState::default();
    let alice = Path::new(r"E:\Mods\Alice");
    let alice_disabled = Path::new(r"E:\Mods\DISABLED Alice");
    let alice_child = Path::new(r"E:\Mods\DISABLED Alice\Blue\mod.ini");
    let bob = Path::new(r"E:\Mods\Bob");
    // Same identity prefix as a sibling name — must NOT match "Alice".
    let alice_v2 = Path::new(r"E:\Mods\Alice V2");

    let guard = state.suppressor.suppress_paths([alice]);

    // Blanket flag untouched: unrelated events keep flowing.
    assert!(!state.suppressor.load(Ordering::Acquire));

    assert!(state.suppressor.is_path_suppressed(alice));
    assert!(state.suppressor.is_path_suppressed(alice_disabled));
    assert!(state.suppressor.is_path_suppressed(alice_child));
    assert!(!state.suppressor.is_path_suppressed(bob));
    assert!(!state.suppressor.is_path_suppressed(alice_v2));

    drop(guard);
    assert!(!state.suppressor.is_path_suppressed(alice_disabled));
    assert!(!state.suppressor.is_path_suppressed(bob));
}

#[test]
fn external_rename_and_delete_after_path_guard_drop_are_observed() {
    use notify::event::{ModifyKind, RemoveKind, RenameMode};

    let root = Path::new(r"E:\Mods");
    let renamed_from = root.join("Alice");
    let renamed_to = root.join("Alicia");
    let deleted = root.join("Bob");
    let state = WatcherState::new();
    let session = state.begin_session(root);
    let suppressor = state.suppressor.clone();
    let guard = suppressor.suppress_paths([renamed_from.as_path(), deleted.as_path()]);
    drop(guard);

    let emitted = std::cell::RefCell::new(Vec::new());
    let rename = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
        .add_path(renamed_from.clone())
        .add_path(renamed_to.clone());
    classify_event(&rename, root, &suppressor, &session, &|event| {
        emitted.borrow_mut().push(event);
    });
    let remove = Event::new(EventKind::Remove(RemoveKind::Folder)).add_path(deleted.clone());
    classify_event(&remove, root, &suppressor, &session, &|event| {
        emitted.borrow_mut().push(event);
    });

    let emitted = emitted.into_inner();
    assert!(matches!(
        emitted.as_slice(),
        [
            ModWatchEvent::Renamed { from, to },
            ModWatchEvent::Removed(removed),
        ] if from == &renamed_from.to_string_lossy()
            && to == &renamed_to.to_string_lossy()
            && removed == &deleted.to_string_lossy()
    ));
}

#[test]
fn repair_acknowledgment_does_not_cross_watcher_session_or_root() {
    let state = WatcherState::new();
    let stale_session = state.begin_session(Path::new(r"E:\ModsA"));
    state.suppressor.mark_blanket_event_dropped(&stale_session);
    let stale_evidence = state
        .suppressor
        .pending_repair(&stale_session)
        .expect("stale session evidence");

    let current_session = state.begin_session(Path::new(r"E:\ModsB"));
    assert!(!state.is_current_session(&stale_session));
    assert!(state.is_current_session(&current_session));
    state
        .suppressor
        .mark_blanket_event_dropped(&current_session);

    // A full reconcile owned by the stale session/root must not acknowledge
    // evidence recorded by the current watcher session/root.
    assert!(!state.suppressor.mark_repaired_through(&stale_evidence));

    assert!(state.suppressor.pending_repair(&current_session).is_some());
}

#[test]
fn test_watcher_keeps_deep_directory_events_but_filters_deep_asset_noise() {
    let root = Path::new(r"E:\Mods");
    let deep_dir = root.join("Alice").join("Nested").join("Blue Dress");
    let deep_asset = deep_dir.join("mesh.buf");
    let runtime_file = deep_dir.join("mod.ini");

    assert!(should_keep_event_path(&deep_dir, root));
    assert!(should_keep_event_path(&runtime_file, root));
    assert!(!should_keep_event_path(&deep_asset, root));
}

#[test]
fn backend_rescan_flag_requests_full_reconcile() {
    let root = Path::new(r"E:\Mods");
    let state = WatcherState::new();
    let session = state.begin_session(root);
    let event = Event::new(EventKind::Other).set_flag(notify::event::Flag::Rescan);
    let emitted = std::cell::RefCell::new(Vec::new());

    classify_event(&event, root, &state.suppressor, &session, &|event| {
        emitted.borrow_mut().push(event);
    });

    assert!(matches!(
        emitted.into_inner().as_slice(),
        [ModWatchEvent::Error(message)] if message.contains("full disk reconcile")
    ));
}

#[test]
fn importer_d3dx_ini_is_forwarded_without_watching_importer_assets() {
    let mods_root = Path::new(r"E:\Importer\Mods");
    let d3dx = Path::new(r"E:\Importer\d3dx.ini");
    let state = WatcherState::new();
    let session = state.begin_session(mods_root);
    let event = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(d3dx.to_path_buf());
    let emitted = std::cell::RefCell::new(Vec::new());

    classify_event_with_runtime_config(
        &event,
        mods_root,
        Some(d3dx),
        &state.suppressor,
        &session,
        &|event| emitted.borrow_mut().push(event),
    );

    assert!(matches!(
        emitted.into_inner().as_slice(),
        [ModWatchEvent::Modified(path)] if path == &d3dx.to_string_lossy()
    ));
}

// Regression: folders with dots in the name ("Mod v1.2") must not be
// mistaken for files with irrelevant extensions and dropped.
#[test]
fn test_watcher_keeps_dot_named_folders_at_any_depth() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    let deep_dot_dir = root.join("Alice").join("Nested").join("Variant v1.2");
    fs::create_dir_all(&deep_dot_dir).unwrap();

    assert!(should_keep_event_path(&root.join("Char v2.0"), root));
    assert!(should_keep_event_path(
        &root.join("Alice").join("Mod v1.2"),
        root
    ));
    assert!(should_keep_event_path(&deep_dot_dir, root));
    // Deep asset noise is still filtered.
    assert!(!should_keep_event_path(
        &deep_dot_dir.join("mesh.buf"),
        root
    ));
}

#[test]
fn test_watcher_keeps_removed_deep_dot_folder_without_stat() {
    let root = Path::new(r"E:\Mods");
    let removed = root.join("Alice").join("Variants").join("Variant v1.2");

    assert!(should_keep_structural_event_path(&removed, root));
    assert!(!should_keep_structural_event_path(
        &Path::new(r"E:\Other").join("Variant v1.2"),
        root
    ));
    assert!(!should_keep_structural_event_path(
        &root.join("Alice").join(".emmm-stage").join("Variant v1.2"),
        root
    ));
}

#[tokio::test]
async fn test_watcher_detects_rename_of_dot_named_folder() {
    let dir = TempDir::new().unwrap();
    let state = WatcherState::new();
    let session = state.begin_session(dir.path());

    let old_dir = dir.path().join("Alice").join("Mod v1.2");
    fs::create_dir_all(&old_dir).unwrap();

    let (watcher, mut rx) =
        watch_mod_directory(dir.path(), state.suppressor.clone(), session).unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let new_dir = dir.path().join("Alice").join("DISABLED Mod v1.2");
    fs::rename(&old_dir, &new_dir).unwrap();

    let mut received = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        if let Ok(Some(
            ModWatchEvent::Renamed { .. } | ModWatchEvent::Created(_) | ModWatchEvent::Removed(_),
        )) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await
        {
            received = true;
            break;
        }
    }

    assert!(
        received,
        "Expected rename/status event for dot-named folder"
    );
    drop(watcher);
}

#[test]
fn test_watcher_nonexistent_path() {
    let path = Path::new("/nonexistent/path");
    let state = WatcherState::new();
    let session = state.begin_session(path);
    let result = watch_mod_directory(path, state.suppressor.clone(), session);
    assert!(result.is_err());
}

#[test]
fn watcher_session_generation_invalidates_old_roots() {
    let state = WatcherState::new();
    let first = state.begin_session(Path::new(r"E:\ModsA"));
    assert!(state.is_current_session(&first));

    let second = state.begin_session(Path::new(r"E:\ModsB"));
    assert!(!state.is_current_session(&first));
    assert!(state.is_current_session(&second));

    state.invalidate_session();
    assert!(!state.is_current_session(&second));
}

#[test]
fn replaced_watcher_session_cannot_publish_after_the_current_session_changes() {
    let state = WatcherState::new();
    let first = state.begin_session(Path::new(r"E:\ModsA"));
    let mut publishes = 0;

    assert_eq!(
        state.with_current_session(&first, || {
            publishes += 1;
            "first"
        }),
        Some("first")
    );
    state.begin_session(Path::new(r"E:\ModsB"));

    assert_eq!(
        state.with_current_session(&first, || {
            publishes += 1;
            "stale"
        }),
        None
    );
    assert_eq!(publishes, 1);
}
