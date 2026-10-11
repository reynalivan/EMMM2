use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn ordered_rename_endpoint_policy_preserves_original_event() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path().join("Mods");
    fs::create_dir(&root).unwrap();
    let runtime = fixture.path().join("d3dx.ini");
    let visible = root.join("Alice/Blue");
    let hidden = root.join(".staging/Blue");
    let outside = fixture.path().join("outside");
    for (from, to, expected) in [
        (&visible, &hidden, "removed"),
        (&hidden, &visible, "created"),
        (&outside, &visible, "created"),
        (&visible, &runtime, "renamed"),
        (&hidden, &outside, "none"),
    ] {
        let state = WatcherState::new();
        let session = state.prepare_session_with_runtime_config(&root, Some(&runtime));
        let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(from.clone())
            .add_path(to.clone());
        let original = event.paths.clone();
        let emitted = std::cell::RefCell::new(Vec::new());
        classify_event_with_runtime_config(
            &event,
            &root,
            Some(&runtime),
            &state.suppressor,
            &session,
            &|event| emitted.borrow_mut().push(event),
        );
        let output = emitted.into_inner();
        let actual = match output.as_slice() {
            [ModWatchEvent::Removed(_)] => "removed",
            [ModWatchEvent::Created(_)] => "created",
            [ModWatchEvent::Renamed { .. }] => "renamed",
            [] => "none",
            _ => panic!("unexpected rename delivery: {output:?}"),
        };
        assert_eq!(actual, expected);
        assert_eq!(event.paths, original);
    }
    let state = WatcherState::new();
    let session = state.begin_session(&root);
    let to = root.join("Bob/Blue");
    let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
        .add_path(visible.clone())
        .add_path(to.clone());
    let original = event.paths.clone();
    let _guard = state.suppressor.suppress_paths([&visible]);
    let emitted = std::cell::RefCell::new(Vec::new());
    classify_event(&event, &root, &state.suppressor, &session, &|event| {
        emitted.borrow_mut().push(event)
    });
    assert!(matches!(emitted.into_inner().as_slice(),
        [ModWatchEvent::Created(path)] if path == &to.to_string_lossy()));
    assert_eq!(event.paths, original);
}

#[test]
fn binding_evidence_cannot_restore_invalid_live_cache_or_change_generation() {
    let fixture = TempDir::new().unwrap();
    let state = WatcherState::new();
    let session = state.prepare_session(fixture.path());
    let original_generation = session.generation();
    let _newer_candidate = state.prepare_session(fixture.path());
    assert!(session.matches_current_binding(fixture.path(), None));
    assert_eq!(session.generation(), original_generation);
    session
        .identity_cache_validity()
        .store(false, Ordering::Release);
    assert!(!session.matches_current_binding(fixture.path(), None));
}

#[cfg(windows)]
#[tokio::test]
async fn inactive_binding_accepts_equivalent_windows_namespace_without_dropping_backlog() {
    let fixture = TempDir::new().unwrap();
    let state = WatcherState::new();
    let session = state.prepare_session(fixture.path());
    let generation = session.generation();
    let _newer_candidate = state.prepare_session(fixture.path());
    let (native, receiver) =
        watch_mod_directory(fixture.path(), state.suppressor.clone(), session.clone()).unwrap();
    state.install_inactive_watcher(
        "game".into(),
        fixture.path(),
        None,
        session,
        native,
        receiver,
    );
    let created = fixture.path().join("backlog.ini");
    fs::write(&created, "[Constants]\n").unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if !crate::shared::sync::lock(&state.inactive_watchers)
                .get("game")
                .unwrap()
                .receiver
                .receiver
                .is_empty()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let verbatim = fs::canonicalize(fixture.path()).unwrap();
    assert_ne!(fixture.path(), verbatim);
    assert_eq!(
        state.inactive_watcher_session("game", &verbatim, None),
        Some(generation)
    );
    assert!(!state.discard_inactive_watcher_unless_coverage("game", &verbatim, None));
    let (adopted, _native, mut receiver) = state
        .take_inactive_watcher_for_handoff("game", &verbatim, None)
        .unwrap();
    assert_eq!(adopted.generation(), generation);
    assert!(
        matches!(receiver.try_recv(), Ok(ModWatchEvent::Created(path)) if path == created.to_string_lossy())
    );
}

#[test]
fn configured_runtime_requires_live_parent_identity() {
    let fixture = TempDir::new().unwrap();
    let runtime = fixture.path().join("missing/d3dx.ini");
    let state = WatcherState::new();
    let session = state.prepare_session_with_runtime_config(fixture.path(), Some(&runtime));
    assert!(!session.matches_current_binding(fixture.path(), Some(&runtime)));
    let (native, receiver) = watch_mod_directory_with_runtime_config(
        fixture.path(),
        Some(&runtime),
        state.suppressor.clone(),
        session.clone(),
    )
    .unwrap();
    state.install_inactive_watcher(
        "game".into(),
        fixture.path(),
        Some(&runtime),
        session,
        native,
        receiver,
    );
    assert!(state
        .inactive_session_for_coverage("game", fixture.path(), Some(&runtime))
        .is_none());
    assert!(state
        .take_inactive_watcher_for_handoff("game", fixture.path(), Some(&runtime))
        .is_none());
    assert!(state.discard_inactive_watcher_unless_coverage("game", fixture.path(), Some(&runtime)));
}

#[test]
fn watcher_fix_invalid_inactive_cache_is_rejected_before_handoff() {
    let fixture = TempDir::new().unwrap();
    let state = WatcherState::new();
    let session = state.prepare_session(fixture.path());
    let (native, receiver) = watch_mod_directory(
        fixture.path(),
        Arc::clone(&state.suppressor),
        session.clone(),
    )
    .unwrap();
    session
        .identity_cache_validity()
        .store(false, Ordering::Release);
    state.install_inactive_watcher(
        "game".into(),
        fixture.path(),
        None,
        session,
        native,
        receiver,
    );
    assert!(state
        .inactive_watcher_session("game", fixture.path(), None)
        .is_none());
    assert!(state
        .take_inactive_watcher_for_handoff("game", fixture.path(), None)
        .is_none());
    assert!(state.discard_inactive_watcher_unless_coverage("game", fixture.path(), None));
}

#[tokio::test]
async fn watcher_fix_runtime_siblings_do_not_reach_authority_observer() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path().join("Mods");
    fs::create_dir(&root).unwrap();
    let runtime = fixture.path().join("d3dx.ini");
    fs::write(&runtime, "[Constants]\nold=1\n").unwrap();
    let state = WatcherState::new();
    let session = state.prepare_session_with_runtime_config(&root, Some(&runtime));
    state.publish_session(&session);
    let observed = Arc::new(Mutex::new(Vec::new()));
    let output = Arc::clone(&observed);
    let (native, mut receiver) = watch_mod_directory_with_runtime_config_and_observer(
        &root,
        Some(&runtime),
        Arc::clone(&state.suppressor),
        session,
        Some(Arc::new(move |paths, _, _| {
            output.lock().unwrap().extend_from_slice(paths)
        })),
    )
    .unwrap();
    let noise = fixture.path().join("irrelevant.log");
    fs::write(&noise, "importer log").unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), receiver.recv())
            .await
            .is_err()
    );
    assert!(!observed
        .lock()
        .unwrap()
        .iter()
        .any(|path| path.file_name() == noise.file_name()));
    fs::write(&runtime, "[Constants]\nchanged=1\n").unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .unwrap()
            .is_some()
    );
    assert!(observed
        .lock()
        .unwrap()
        .iter()
        .any(|path| path.file_name() == runtime.file_name()));
    drop(native);
}

#[cfg(target_os = "windows")]
#[path = "native_watcher_stress.rs"]
mod native_stress;

#[tokio::test]
async fn watcher_fix_suppressed_external_edit_wakes_without_a_second_event() {
    let fixture = TempDir::new().unwrap();
    let folder = fixture.path().join("Object/Mod");
    fs::create_dir_all(&folder).unwrap();
    let ini = folder.join("external.ini");
    fs::write(&ini, "[Constants]\nold=1\n").unwrap();
    let state = WatcherState::new();
    let session = state.begin_session(fixture.path());
    let observed = Arc::new(Mutex::new(Vec::new()));
    let output = Arc::clone(&observed);
    let (native, mut receiver) = watch_mod_directory_with_runtime_config_and_observer(
        fixture.path(),
        None,
        Arc::clone(&state.suppressor),
        session.clone(),
        Some(Arc::new(move |paths, _, _| {
            output.lock().unwrap().extend_from_slice(paths)
        })),
    )
    .unwrap();
    while tokio::time::timeout(Duration::from_millis(750), receiver.recv())
        .await
        .is_ok()
    {}
    observed.lock().unwrap().clear();
    let guard = state.suppressor.suppress_paths([&folder]);
    fs::write(&ini, "[Constants]\nchanged=1\n").unwrap();
    tokio::time::timeout(Duration::from_secs(5), session.wait_for_work())
        .await
        .unwrap();
    assert!(observed
        .lock()
        .unwrap()
        .iter()
        .any(|path| path.file_name() == ini.file_name()));
    assert!(receiver.try_recv().is_err());
    drop(guard);
    tokio::time::timeout(Duration::from_secs(1), session.wait_for_work())
        .await
        .unwrap();
    drop(native);
}

#[tokio::test]
async fn watcher_fix_work_wakes_coalesce_and_do_not_cross_sessions() {
    let fixture = TempDir::new().unwrap();
    let state = WatcherState::new();
    let first = state.begin_session(fixture.path());
    let clone = first.clone();
    for _ in 0..1000 {
        clone.request_work();
    }
    tokio::time::timeout(Duration::from_secs(1), first.wait_for_work())
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), first.wait_for_work())
            .await
            .is_err()
    );
    let successor = state.begin_session(fixture.path());
    first.request_work();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), successor.wait_for_work())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn watcher_fix_unseen_echo_expiry_wakes_only_its_owner() {
    let fixture = TempDir::new().unwrap();
    let old = fixture.path().join("Mod");
    let new = fixture.path().join("DISABLED Mod");
    fs::create_dir(&old).unwrap();
    let state = WatcherState::new();
    let owner = state.begin_session(fixture.path());
    let evidence = state.suppressor.expect_rename_echoes(
        "game",
        &owner,
        [ExpectedRenameEcho {
            old_path: old.clone(),
            new_path: new.clone(),
            expected_identity: crate::platform::fs::file_utils::filesystem_identity(&old).unwrap(),
        }],
    );
    fs::rename(&old, &new).unwrap();
    assert!(state.suppressor.commit_expected_rename_echoes(&evidence));
    let successor = state.begin_session(fixture.path());
    tokio::time::timeout(Duration::from_secs(13), owner.wait_for_work())
        .await
        .unwrap();
    assert!(state.suppressor.pending_repair(&successor).is_none());
    assert!(
        tokio::time::timeout(Duration::from_millis(50), successor.wait_for_work())
            .await
            .is_err()
    );
}

#[cfg(target_os = "windows")]
#[path = "native_file_moves.rs"]
mod native_file_moves;

#[test]
fn verified_rename_echo_is_not_delivered_but_pending_and_replacement_events_are() {
    let dir = TempDir::new().unwrap();
    let old = dir.path().join("Skin");
    let new = dir.path().join("DISABLED Skin");
    fs::create_dir(&old).unwrap();
    let state = WatcherState::new();
    let session = state.begin_session(dir.path());
    let evidence = state.suppressor.expect_rename_echoes(
        "game",
        &session,
        [ExpectedRenameEcho {
            expected_identity: crate::platform::fs::file_utils::filesystem_identity(&old).unwrap(),
            old_path: old.clone(),
            new_path: new.clone(),
        }],
    );
    fs::rename(&old, &new).unwrap();
    let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
        .add_path(old)
        .add_path(new.clone());
    let delivered = std::sync::Mutex::new(Vec::new());
    let send = |event| delivered.lock().unwrap().push(event);
    classify_event(&event, dir.path(), &state.suppressor, &session, &send);
    assert_eq!(delivered.lock().unwrap().len(), 1);
    delivered.lock().unwrap().clear();

    assert!(state.suppressor.commit_expected_rename_echoes(&evidence));
    assert!(state.suppressor.observe_expected_rename_echo(
        "game",
        &session,
        &event.kind,
        &event.paths,
        Arc::new(|_, _| panic!("committed echo should not need repair")),
    ));
    classify_event(&event, dir.path(), &state.suppressor, &session, &send);
    assert!(delivered.lock().unwrap().is_empty());
    assert!(state.suppressor.observe_expected_rename_echo(
        "game",
        &session,
        &event.kind,
        &event.paths,
        Arc::new(|_, _| panic!("verified duplicate should not need repair")),
    ));
    classify_event(&event, dir.path(), &state.suppressor, &session, &send);
    assert!(delivered.lock().unwrap().is_empty());

    assert!(!state.suppressor.observe_expected_rename_echo(
        "foreign-game",
        &session,
        &event.kind,
        &event.paths,
        Arc::new(|_, _| panic!("foreign observer must not consume this receipt")),
    ));
    fs::rename(&new, dir.path().join("OriginalIdentity")).unwrap();
    fs::create_dir(&new).unwrap();
    assert!(!state.suppressor.observe_expected_rename_echo(
        "game",
        &session,
        &event.kind,
        &event.paths,
        Arc::new(|_, _| panic!("replacement must not match the receipt")),
    ));
    classify_event(&event, dir.path(), &state.suppressor, &session, &send);
    assert_eq!(delivered.lock().unwrap().len(), 1);
}

#[cfg(windows)]
#[test]
fn event_filter_accepts_raw_and_verbatim_namespace_without_accepting_foreign_roots() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("Mods");
    fs::create_dir(&root).unwrap();
    let verbatim_root = fs::canonicalize(&root).unwrap();
    let raw_path = root.join("Character.v2/Skin.v1");
    assert!(should_keep_structural_event_path(&raw_path, &verbatim_root));
    assert!(should_keep_event_path(&raw_path, &verbatim_root));
    assert!(!should_keep_event_path(
        &root.join(".emmm_data/cache.json"),
        &verbatim_root
    ));
    assert!(!should_keep_structural_event_path(
        &dir.path().join("Other/Skin"),
        &verbatim_root
    ));

    let config = dir.path().join("d3dx.ini");
    fs::write(&config, "[Main]").unwrap();
    assert!(is_runtime_config_path(
        &config,
        Some(&fs::canonicalize(&config).unwrap())
    ));
    assert!(!is_runtime_config_path(
        &dir.path().join("other.ini"),
        Some(&config)
    ));
}

#[test]
fn inactive_watcher_does_not_adopt_a_disabled_alias_runtime_config() {
    let dir = TempDir::new().expect("tempdir");
    let config = dir.path().join("importer.ini");
    let other_config = dir.path().join("DISABLED importer.ini");
    fs::write(&config, "[Loader]").expect("runtime config");
    fs::write(&other_config, "[Loader]").expect("distinct runtime config");
    let state = WatcherState::new();
    let session = state.prepare_session_with_runtime_config(dir.path(), Some(&config));
    let generation = session.generation();
    let (watcher, receiver) = watch_mod_directory_with_runtime_config(
        dir.path(),
        Some(&config),
        state.suppressor.clone(),
        session.clone(),
    )
    .expect("watch root and config");
    state.install_inactive_watcher(
        "game".to_string(),
        dir.path(),
        Some(&config),
        session,
        watcher,
        receiver,
    );

    assert_eq!(
        state.inactive_watcher_session("game", dir.path(), Some(&config)),
        Some(generation)
    );
    assert!(state
        .inactive_watcher_session("game", dir.path(), Some(&other_config))
        .is_none());
    assert!(state
        .take_inactive_watcher_for_handoff("game", dir.path(), Some(&other_config))
        .is_none());
    assert!(state.discard_inactive_watcher_unless_coverage(
        "game",
        dir.path(),
        Some(&other_config)
    ));
}

#[test]
fn inactive_watcher_with_changed_root_identity_cannot_be_adopted() {
    let dir = TempDir::new().expect("tempdir");
    let state = WatcherState::new();
    let session = state.prepare_session(dir.path());
    let (watcher, receiver) =
        watch_mod_directory(dir.path(), state.suppressor.clone(), session.clone())
            .expect("watch root");
    state.install_inactive_watcher(
        "game-1".to_string(),
        dir.path(),
        None,
        session,
        watcher,
        receiver,
    );
    crate::shared::sync::lock(&state.inactive_watchers)
        .get_mut("game-1")
        .expect("inactive watcher")
        .binding
        .root_identity = Some("retired-root-id".to_string());

    assert!(state
        .inactive_watcher_session("game-1", dir.path(), None)
        .is_none());
    assert!(state
        .take_inactive_watcher_for_handoff("game-1", dir.path(), None)
        .is_none());
    assert!(state.discard_inactive_watcher_unless_coverage("game-1", dir.path(), None));
}

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
fn single_path_ambiguous_rename_requests_full_reconcile() {
    use notify::event::{ModifyKind, RenameMode};

    let root = Path::new(r"E:\\Mods");
    let renamed = root.join("Alice");
    let state = WatcherState::new();
    let session = state.begin_session(root);
    let emitted = std::cell::RefCell::new(Vec::new());
    let rename = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Any))).add_path(renamed);

    classify_event(&rename, root, &state.suppressor, &session, &|event| {
        emitted.borrow_mut().push(event);
    });

    assert!(matches!(
        emitted.into_inner().as_slice(),
        [ModWatchEvent::Error(message)] if message.contains("full disk reconcile")
    ));
}

#[test]
fn suppressed_single_path_ambiguous_rename_is_not_forwarded() {
    use notify::event::{ModifyKind, RenameMode};

    let root = Path::new(r"E:\\Mods");
    let renamed = root.join("Alice");
    let state = WatcherState::new();
    let session = state.begin_session(root);
    let _guard = state.suppressor.suppress_paths([renamed.as_path()]);
    let emitted = std::cell::RefCell::new(Vec::new());
    let rename = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Any))).add_path(renamed);

    classify_event(&rename, root, &state.suppressor, &session, &|event| {
        emitted.borrow_mut().push(event);
    });

    assert!(emitted.into_inner().is_empty());
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
    for (kind, expected) in [
        (ModifyKind::Any, "modified"),
        (ModifyKind::Name(RenameMode::From), "removed"),
        (ModifyKind::Name(RenameMode::To), "created"),
    ] {
        let event = Event::new(EventKind::Modify(kind)).add_path(d3dx.to_path_buf());
        let policy = WatcherCoverage::new(mods_root, Some(d3dx)).policy(&event);
        assert!(policy.authority_relevant);
        let emitted = std::cell::RefCell::new(Vec::new());
        classify_event_with_runtime_config(
            &event,
            mods_root,
            Some(d3dx),
            &state.suppressor,
            &session,
            &|event| emitted.borrow_mut().push(event),
        );
        let output = emitted.into_inner();
        let (actual, path) = match output.as_slice() {
            [ModWatchEvent::Modified(path)] => ("modified", path),
            [ModWatchEvent::Removed(path)] => ("removed", path),
            [ModWatchEvent::Created(path)] => ("created", path),
            _ => panic!("unexpected runtime config event: {output:?}"),
        };
        assert_eq!(actual, expected);
        assert_eq!(path, &d3dx.to_string_lossy());
    }
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

#[test]
fn watcher_session_coverage_includes_runtime_config_path() {
    let state = WatcherState::new();
    let fixture = tempfile::tempdir().expect("owned coverage fixture");
    let mods_root = fixture.path().join("Mods");
    let old_runtime = fixture.path().join("GameA/d3dx.ini");
    let new_runtime = fixture.path().join("GameB/d3dx.ini");
    for directory in [
        mods_root.as_path(),
        old_runtime.parent().expect("runtime parent"),
        new_runtime.parent().expect("runtime parent"),
    ] {
        std::fs::create_dir_all(directory).expect("physical coverage directories");
    }
    let session = state.prepare_session_with_runtime_config(&mods_root, Some(&old_runtime));
    state.publish_session(&session);

    assert!(state
        .current_session_for_coverage(&mods_root, Some(&old_runtime))
        .is_some());
    assert!(state
        .current_session_for_coverage(&mods_root, Some(&new_runtime))
        .is_none());
}
