use super::super::{
    watch_mod_directory_with_runtime_config_and_observer, ExpectedRenameEcho, ModWatchEvent,
    WatchEventReceiver, WatchObservation, WatcherSession, WatcherState, WatcherSuppressor,
};
use crate::modules::reconciliation::api::disk_reconcile::{
    orchestrator::{AuthorityCatchUp, DiskReconcileState},
    types::{
        DiskReconcileReason, DiskReconcileResult, DiskReconcileScanScope, DiskReconcileStatus,
    },
};
use crate::platform::fs::file_utils::filesystem_identity;
use notify::EventKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

const GAME: &str = "native-watcher-test";
const EVENT_DEADLINE: Duration = Duration::from_secs(5);

#[derive(Debug)]
struct Observation {
    paths: Vec<PathBuf>,
    kind: Option<EventKind>,
    proven_internal: bool,
    events_lost: bool,
}

fn observer(
    suppressor: Arc<WatcherSuppressor>,
    authority: Arc<DiskReconcileState>,
    session: WatcherSession,
    root: &Path,
) -> (WatchObservation, UnboundedReceiver<Observation>) {
    let (tx, receiver) = unbounded_channel();
    let root = root.to_path_buf();
    let deferred_authority = Arc::clone(&authority);
    let deferred_root = root.clone();
    let generation = session.generation();
    let deferred_tx = tx.clone();
    let on_unproven = Arc::new(move |paths: &[PathBuf], events_lost| {
        deferred_authority.observe_authority_event(
            GAME,
            generation,
            &deferred_root,
            paths,
            events_lost,
        );
        let _ = deferred_tx.send(Observation {
            paths: paths.to_vec(),
            kind: None,
            proven_internal: false,
            events_lost,
        });
    });
    let callback = Arc::new(
        move |paths: &[PathBuf], kind: Option<&EventKind>, events_lost| {
            if events_lost {
                suppressor.invalidate_rename_echoes(&session);
            }
            let proven_internal = !events_lost
                && kind.is_some_and(|kind| {
                    suppressor.observe_expected_rename_echo(
                        GAME,
                        &session,
                        kind,
                        paths,
                        on_unproven.clone(),
                    )
                });
            if !proven_internal {
                authority.observe_authority_event(GAME, generation, &root, paths, events_lost);
            }
            let _ = tx.send(Observation {
                paths: paths.to_vec(),
                kind: kind.copied(),
                proven_internal,
                events_lost,
            });
        },
    );
    (callback, receiver)
}

async fn observation_for(
    receiver: &mut UnboundedReceiver<Observation>,
    paths: &[&Path],
) -> Observation {
    observation_matching(receiver, |observed| {
        observed.events_lost
            || observed
                .paths
                .iter()
                .any(|path| paths.contains(&path.as_path()))
    })
    .await
}

async fn observation_matching(
    receiver: &mut UnboundedReceiver<Observation>,
    matches: impl Fn(&Observation) -> bool,
) -> Observation {
    tokio::time::timeout(EVENT_DEADLINE, async {
        loop {
            let observed = receiver.recv().await.expect("live native watcher callback");
            if matches(&observed) {
                return observed;
            }
        }
    })
    .await
    .expect("native watcher did not observe the changed path within five seconds")
}

fn acknowledge_fixture_snapshot(
    authority: &DiskReconcileState,
    root: &Path,
    session: &WatcherSession,
) {
    let observed_generation = match authority.authority_catch_up(GAME, root, session.generation()) {
        AuthorityCatchUp::Clean {
            observed_generation,
            ..
        }
        | AuthorityCatchUp::Scoped {
            observed_generation,
            ..
        }
        | AuthorityCatchUp::Full {
            observed_generation,
        } => observed_generation,
    };
    let mut snapshot = DiskReconcileResult {
        game_id: GAME.to_string(),
        reconcile_revision: 0,
        reason: DiskReconcileReason::StartupBoot,
        status: DiskReconcileStatus::Applied,
        scan_scope: DiskReconcileScanScope::Full,
        folder_conflicts: Vec::new(),
        rename_confirmations: Vec::new(),
        error_message: None,
        changed_roots: Vec::new(),
        objects_changed: false,
        folders_changed: false,
        collections_changed: false,
        runtime_file_changed: false,
        thumbnail_roots: Vec::new(),
        cleared_selection_paths: Vec::new(),
        path_updates: Vec::new(),
        collection_reference_impact: Default::default(),
        change_summary: Default::default(),
        pending_runtime_effects: Default::default(),
        warnings: Vec::new(),
    };
    authority.record_result(GAME, &mut snapshot);
    assert!(authority.mark_authority_reconciled(
        GAME,
        root,
        session.generation(),
        observed_generation,
        &snapshot,
        &[],
    ));
}

fn verified_fixture_rename(state: &WatcherState, session: &WatcherSession, old: &Path, new: &Path) {
    let identity = filesystem_identity(old).expect("source identity");
    let evidence = state.suppressor.expect_rename_echoes(
        GAME,
        session,
        [ExpectedRenameEcho {
            old_path: old.to_path_buf(),
            new_path: new.to_path_buf(),
            expected_identity: identity.clone(),
        }],
    );
    std::fs::rename(old, new).expect("temporary fixture rename");
    assert_eq!(filesystem_identity(new).as_deref(), Some(identity.as_str()));
    assert!(!old.exists());
    assert!(state.suppressor.commit_expected_rename_echoes(&evidence));
}

fn assert_proven_or_repair(
    observed: &Observation,
    authority: &DiskReconcileState,
    root: &Path,
    session: &WatcherSession,
) {
    if !observed.proven_internal {
        assert!(
            !matches!(
                authority.authority_catch_up(GAME, root, session.generation()),
                AuthorityCatchUp::Clean { .. }
            ),
            "unproven native event must request repair: {observed:?}"
        );
    }
}

async fn typed_event_for(receiver: &mut WatchEventReceiver, paths: &[&Path]) {
    let keys = paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    tokio::time::timeout(EVENT_DEADLINE, async {
        loop {
            match receiver.recv().await.expect("handed-off event receiver") {
                ModWatchEvent::Renamed { from, to }
                    if keys.contains(&from) || keys.contains(&to) =>
                {
                    break
                }
                ModWatchEvent::Created(path) | ModWatchEvent::Removed(path)
                    if keys.contains(&path) =>
                {
                    break
                }
                ModWatchEvent::Error(_) => break,
                _ => {}
            }
        }
    })
    .await
    .expect("takeover lost the debouncer backlog");
}

#[tokio::test]
async fn live_debouncer_handles_slow_and_rapid_verified_folder_renames() {
    let temp = tempfile::tempdir().expect("temp root");
    let enabled = temp.path().join("Alice");
    let disabled = temp.path().join("DISABLED Alice");
    std::fs::create_dir(&enabled).expect("mod folder");
    let identity = filesystem_identity(&enabled).expect("initial identity");
    let state = WatcherState::new();
    let session = state.begin_session(temp.path());
    let authority = Arc::new(DiskReconcileState::new());
    authority.begin_authority_session(GAME, temp.path(), session.generation());
    let (callback, mut observations) = observer(
        state.suppressor.clone(),
        authority.clone(),
        session.clone(),
        temp.path(),
    );
    let (watcher, _events) = watch_mod_directory_with_runtime_config_and_observer(
        temp.path(),
        None,
        state.suppressor.clone(),
        session.clone(),
        Some(callback),
    )
    .expect("native watcher");
    let ready = temp.path().join("ready.ini");
    std::fs::write(&ready, "[Ready]").expect("startup handshake");
    observation_for(&mut observations, &[&ready]).await;
    acknowledge_fixture_snapshot(&authority, temp.path(), &session);

    let mut slow_proven = 0;
    for index in 0..4 {
        let (old, new) = if index % 2 == 0 {
            (&enabled, &disabled)
        } else {
            (&disabled, &enabled)
        };
        verified_fixture_rename(&state, &session, old, new);
        let observed = observation_for(&mut observations, &[old, new]).await;
        slow_proven += usize::from(observed.proven_internal);
        assert_proven_or_repair(&observed, &authority, temp.path(), &session);
    }
    assert!(
        slow_proven > 0,
        "the native backend never supplied a provable slow rename"
    );

    for index in 0..41 {
        let (old, new) = if index % 2 == 0 {
            (&enabled, &disabled)
        } else {
            (&disabled, &enabled)
        };
        verified_fixture_rename(&state, &session, old, new);
    }
    let rapid = observation_for(&mut observations, &[&enabled, &disabled]).await;
    assert_proven_or_repair(&rapid, &authority, temp.path(), &session);
    assert_eq!(
        filesystem_identity(&disabled).as_deref(),
        Some(identity.as_str())
    );
    assert!(!enabled.exists());
    let rapid_ini = disabled.join("rapid.ini");
    std::fs::write(&rapid_ini, "[Constants]\nvalue=1").expect("write under final rapid rename");
    let changed = observation_for(&mut observations, &[&rapid_ini]).await;
    assert!(
        !changed.proven_internal,
        "a descendant content edit must not be hidden by rename evidence"
    );
    assert_proven_or_repair(&changed, &authority, temp.path(), &session);
    eprintln!(
        "native slow proven={slow_proven}/4; rapid kind={:?}, proven={}, lost={}",
        rapid.kind, rapid.proven_internal, rapid.events_lost
    );
    drop(watcher);
}

#[tokio::test]
async fn live_takeover_preserves_backlog_and_detects_external_identity_replacement() {
    let temp = tempfile::tempdir().expect("temp root");
    let enabled = temp.path().join("Alice");
    let disabled = temp.path().join("DISABLED Alice");
    let replacement = temp.path().join("Replacement");
    let archived = temp.path().join("Archived Alice");
    std::fs::create_dir(&enabled).expect("original mod");
    std::fs::create_dir(&replacement).expect("replacement mod");
    let original_identity = filesystem_identity(&enabled).expect("original identity");
    let replacement_identity = filesystem_identity(&replacement).expect("replacement identity");
    let state = WatcherState::new();
    let session = state.prepare_session(temp.path());
    let authority = Arc::new(DiskReconcileState::new());
    authority.begin_authority_session(GAME, temp.path(), session.generation());
    let (callback, mut observations) = observer(
        state.suppressor.clone(),
        authority.clone(),
        session.clone(),
        temp.path(),
    );
    let (watcher, events) = watch_mod_directory_with_runtime_config_and_observer(
        temp.path(),
        None,
        state.suppressor.clone(),
        session.clone(),
        Some(callback),
    )
    .expect("inactive native watcher");
    let ready = temp.path().join("ready.ini");
    std::fs::write(&ready, "[Ready]").expect("startup handshake");
    observation_for(&mut observations, &[&ready]).await;
    acknowledge_fixture_snapshot(&authority, temp.path(), &session);
    state.install_inactive_watcher(
        GAME.to_string(),
        temp.path(),
        None,
        session.clone(),
        watcher,
        events,
    );
    verified_fixture_rename(&state, &session, &enabled, &disabled);
    let (taken_session, taken_watcher, mut taken_events) = state
        .take_inactive_watcher_for_handoff(GAME, temp.path(), None)
        .expect("same watcher takeover");
    assert_eq!(taken_session, session);
    state.publish_session(&taken_session);

    // Replace the intermediate destination before the debounce window flushes.
    std::fs::rename(&disabled, &archived).expect("external archive");
    std::fs::rename(&replacement, &disabled).expect("external replacement");
    let replaced = observation_matching(&mut observations, |observed| {
        !observed.proven_internal
            && (observed.events_lost
                || observed
                    .paths
                    .iter()
                    .any(|path| [&enabled, &disabled, &archived, &replacement].contains(&path)))
    })
    .await;
    assert_proven_or_repair(&replaced, &authority, temp.path(), &session);
    typed_event_for(
        &mut taken_events,
        &[&enabled, &disabled, &archived, &replacement],
    )
    .await;
    assert_eq!(
        filesystem_identity(&disabled).as_deref(),
        Some(replacement_identity.as_str())
    );
    assert_eq!(
        filesystem_identity(&archived).as_deref(),
        Some(original_identity.as_str())
    );

    let next_path = temp.path().join("Externally Renamed");
    std::fs::rename(&disabled, &next_path).expect("second external rename");
    let next = observation_for(&mut observations, &[&next_path]).await;
    assert!(!next.proven_internal);
    let ini = next_path.join("mod.ini");
    std::fs::write(&ini, "[Constants]\nvalue=1").expect("write below renamed folder");
    let modified = observation_for(&mut observations, &[&ini]).await;
    assert!(!modified.proven_internal);
    assert!(state.is_current_session(&session));
    drop(taken_watcher);
}
