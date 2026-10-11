use super::super::*;
use crate::modules::reconciliation::api::disk_reconcile::orchestrator::{
    reconcile_disk_state, reconcile_disk_state_for_watcher, AuthorityCatchUp, DiskReconcileContext,
    DiskReconcileRequest, DiskReconcileState, WatcherReconcileOutcome,
};
use crate::modules::reconciliation::api::disk_reconcile::types::{
    DiskReconcileReason, DiskReconcileResult, DiskReconcileStatus,
};
use crate::modules::reconciliation::api::disk_reconcile::watcher_batch::collect_changed_paths;
use crate::modules::settings::api::config::ConfigService;
use crate::platform::fs::operation_lock::OperationLock;
use std::fs;

async fn settle_authority(
    context: DiskReconcileContext<'_>,
    watcher: &WatcherState,
    session: &WatcherSession,
    root: &Path,
    result: &DiskReconcileResult,
    watermark: u64,
    paths: &[String],
) {
    let state = context.state;
    let generation = session.generation();
    let mut accepted =
        state.mark_authority_reconciled("file-move", root, generation, watermark, result, paths);
    const MAX_CATCH_UP_PASSES: usize = 4;
    for _ in 0..MAX_CATCH_UP_PASSES {
        if accepted {
            return;
        }
        let (changed_paths, force_full, observed) =
            match state.authority_catch_up("file-move", root, generation) {
                AuthorityCatchUp::Scoped {
                    changed_paths,
                    observed_generation,
                } => (changed_paths, false, observed_generation),
                AuthorityCatchUp::Full {
                    observed_generation,
                }
                | AuthorityCatchUp::Clean {
                    observed_generation,
                    ..
                } => (Vec::new(), true, observed_generation),
            };
        let result = reconcile_disk_state_for_watcher(
            context.clone(),
            DiskReconcileRequest::manual(
                "file-move".into(),
                DiskReconcileReason::WatcherBatch,
                changed_paths.clone(),
                force_full,
            )
            .defer_overlay_sync(),
            watcher,
            session.clone(),
        )
        .await
        .expect("native catch-up");
        let WatcherReconcileOutcome::Applied(result) = result else {
            panic!("live session must repair");
        };
        accepted = state.mark_authority_reconciled(
            "file-move",
            root,
            generation,
            observed,
            &result,
            &changed_paths,
        );
    }
    assert!(
        accepted,
        "native observations must settle conservatively: {:?}; cache_valid={}",
        state.authority_catch_up("file-move", root, generation),
        session.identity_cache_validity().load(Ordering::Acquire)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn untracked_cross_root_ini_move_repairs_both_projection_scopes() {
    cross_root_file_move("mod.ini", "mod.ini").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn untracked_cross_root_asset_move_repairs_both_projection_scopes() {
    cross_root_file_move("mesh.buf", "moved_mesh.buf").await;
}

async fn cross_root_file_move(source_file: &str, destination_file: &str) {
    let temp = tempfile::tempdir().expect("owned fixture");
    let root = temp.path().join("Mods");
    let source = root.join("Alpha/Mod");
    let destination = root.join("Beta/Mod");
    fs::create_dir_all(&source).expect("source mod");
    fs::create_dir_all(&destination).expect("destination mod");
    fs::write(source.join("mesh.buf"), b"asset").expect("source asset");
    fs::write(destination.join("mesh.buf"), b"asset").expect("destination asset");
    fs::write(
        source.join("mod.ini"),
        "[TextureOverrideSkin]\nhash = abcdef12\n",
    )
    .expect("mod ini");
    for directory in [&source, &destination] {
        fs::write(
            directory.join("remaining.ini"),
            "[TextureOverrideSkin]\nhash = abcdef12\n",
        )
        .expect("both physical mod roots remain classified");
    }
    let root = fs::canonicalize(root).expect("native root namespace");
    let old_ini = root.join("Alpha/Mod").join(source_file);
    let new_ini = root.join("Beta/Mod").join(destination_file);
    let db = crate::test_utils::init_test_db().await;
    crate::test_utils::insert_test_game(
        &db.pool,
        &crate::test_utils::TestGameFixture {
            id: "file-move",
            name: "Game",
            game_type: crate::modules::games::api::testing::domain::models::GameType::Gimi,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(root.to_string_lossy().as_ref()),
        },
    )
    .await
    .expect("configured game");
    let config = ConfigService::new_for_test_async(db.pool.clone()).await;
    let state = Arc::new(DiskReconcileState::new());
    let watcher_state = WatcherState::new();
    let session = watcher_state.begin_session(&root);
    let observed_state = Arc::clone(&state);
    let observed_root = root.clone();
    let generation = session.generation();
    let (watcher, mut receiver) = watch_mod_directory_with_runtime_config_and_observer(
        &root,
        None,
        Arc::clone(&watcher_state.suppressor),
        session.clone(),
        Some(Arc::new(move |paths, _, lost| {
            observed_state.observe_authority_event(
                "file-move",
                generation,
                &observed_root,
                paths,
                lost,
            );
        })),
    )
    .expect("directory identity watcher");
    *crate::shared::sync::lock(&watcher_state.watcher) = Some(watcher);
    state.begin_authority_session_with_coverage("file-move", &root, &session);
    let lock = OperationLock::new();
    let context = DiskReconcileContext {
        pool: &db.pool,
        config: &config,
        state: state.as_ref(),
        watcher_suppressor: Arc::clone(&watcher_state.suppressor),
        operation_lock: &lock,
        progress_reporter: None,
    };
    let baseline_generation = state
        .authority_event_generation("file-move", generation)
        .expect("registered baseline authority");
    let baseline = reconcile_disk_state(
        context.clone(),
        DiskReconcileRequest::manual(
            "file-move".into(),
            DiskReconcileReason::StartupBoot,
            Vec::new(),
            true,
        )
        .defer_overlay_sync(),
    )
    .await
    .expect("baseline projection");
    settle_authority(
        context.clone(),
        &watcher_state,
        &session,
        &root,
        &baseline,
        baseline_generation,
        &[],
    )
    .await;
    sqlx::query("UPDATE mods SET size_bytes = 777 WHERE game_id = 'file-move'")
        .execute(&db.pool)
        .await
        .expect("existing storage values");
    fs::rename(&old_ini, &new_ini).expect("external file move");
    let mut events = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(event) = tokio::time::timeout(Duration::from_millis(50), receiver.recv()).await {
                events.push(event.expect("live receiver"));
            }
            let removed = events.iter().any(
                |event| matches!(event, ModWatchEvent::Removed(path) if Path::new(path) == old_ini),
            );
            let created = events.iter().any(
                |event| matches!(event, ModWatchEvent::Created(path) if Path::new(path) == new_ini),
            );
            let both_roots_observed = matches!(state.authority_catch_up("file-move", &root, generation), AuthorityCatchUp::Scoped { changed_paths, .. } if changed_paths.len() == 2);
            if removed && both_roots_observed && (created || !source_file.ends_with(".ini")) {
                break;
            }
        }
    })
    .await
    .expect("both file move observations");
    assert!(!events.iter().any(|event| matches!(event, ModWatchEvent::Renamed { from, to } if Path::new(from) == old_ini && Path::new(to) == new_ini)));
    let mut paths = collect_changed_paths(&events);
    if let AuthorityCatchUp::Scoped { changed_paths, .. } =
        state.authority_catch_up("file-move", &root, generation)
    {
        paths.extend(changed_paths);
    }
    let observed = state
        .authority_event_generation("file-move", generation)
        .expect("observed revision");
    let result = reconcile_disk_state_for_watcher(
        context.clone(),
        DiskReconcileRequest::watcher_batch("file-move".into(), &root, paths.clone(), &events),
        &watcher_state,
        session.clone(),
    )
    .await
    .expect("two-root watcher projection");
    let WatcherReconcileOutcome::Applied(result) = result else {
        panic!("current watcher must project");
    };
    assert_eq!(result.status, DiskReconcileStatus::Applied);
    if source_file.ends_with(".ini") {
        assert!(result.runtime_file_changed);
    }
    settle_authority(
        context.clone(),
        &watcher_state,
        &session,
        &root,
        &result,
        observed,
        &paths,
    )
    .await;
    let rows: Vec<(String, i64, Option<String>)> = sqlx::query_as(
        "SELECT folder_path, size_bytes, filesystem_identity FROM mods WHERE game_id = 'file-move'",
    )
    .fetch_all(&db.pool)
    .await
    .expect("fresh mod rows");
    assert_eq!(rows.len(), 2);
    for (folder, size_bytes, identity) in rows {
        assert!(
            root.join(&folder).is_dir(),
            "project the live physical folder"
        );
        assert_eq!(size_bytes, 777, "file moves do not trigger storage sizing");
        assert_eq!(
            identity,
            crate::platform::fs::file_utils::filesystem_identity(&root.join(&folder))
        );
    }
}
