use super::*;
use crate::modules::mutation::journal::{
    DatabaseProjectionStatus, MutationStepKind, Operation, OperationStatus, OperationStep,
    StepStatus,
};
use std::path::Path;

#[tokio::test]
async fn scoped_projection_failure_does_not_start_a_full_scan() {
    for failure in [
        AppError::Db("database is locked".into()),
        AppError::Io("temporary sharing failure".into()),
        AppError::Validation("source identity changed".into()),
        AppError::Cancelled,
    ] {
        let expected = serde_json::to_value(&failure).unwrap();
        let full_calls = std::cell::Cell::new(0);
        let result = project_toggle_scope(
            Some(vec!["Mods/A".into(), "Mods/DISABLED A".into()]),
            |_| async { Err(failure) },
            || async {
                full_calls.set(full_calls.get() + 1);
                Err(AppError::Internal("unexpected full scan".into()))
            },
        )
        .await;
        assert_eq!(
            full_calls.get(),
            0,
            "an error is not lost coverage evidence"
        );
        assert_eq!(serde_json::to_value(result.unwrap_err()).unwrap(), expected);
    }
}

#[tokio::test]
async fn unproven_toggle_scope_starts_exactly_one_full_scan() {
    let full_calls = std::cell::Cell::new(0);
    let result = project_toggle_scope(
        None,
        |_| async { panic!("unproven scope must not use trusted projection") },
        || async {
            full_calls.set(full_calls.get() + 1);
            Err(AppError::Db("full scan failed".into()))
        },
    )
    .await;
    assert_eq!(full_calls.get(), 1);
    assert!(matches!(result, Err(AppError::Db(_))));
}

#[tokio::test]
async fn non_applied_scoped_projection_preserves_its_resolution_status() {
    use crate::modules::reconciliation::application::disk_reconcile::types::{
        DiskReconcileReason, DiskReconcileScanScope,
    };
    for status in [
        DiskReconcileStatus::SourceUnavailable,
        DiskReconcileStatus::NeedsRenameConfirmation,
    ] {
        let expected = status.clone();
        let observed = DiskReconcileResult {
            game_id: "game".into(),
            reconcile_revision: 1,
            reason: DiskReconcileReason::InternalMutation,
            status,
            scan_scope: DiskReconcileScanScope::Scoped,
            folder_conflicts: Vec::new(),
            rename_confirmations: Vec::new(),
            error_message: Some("original scoped diagnostic".into()),
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
        let result = project_toggle_scope(
            Some(vec!["Mods/A".into(), "Mods/DISABLED A".into()]),
            |_| async { Ok(observed) },
            || async { panic!("an unavailable source or confirmation is not a full-scan request") },
        )
        .await
        .unwrap();
        assert_eq!(result.status, expected);
        assert_eq!(
            result.error_message.as_deref(),
            Some("original scoped diagnostic")
        );
    }
}

#[tokio::test]
async fn capture_without_pending_revision_still_rejects_replaced_root() {
    let ctx = crate::test_utils::init_test_db().await;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    std::fs::create_dir(&root).unwrap();
    crate::test_utils::insert_test_game(
        &ctx.pool,
        &crate::test_utils::TestGameFixture {
            id: "capture-root",
            name: "Game",
            game_type: crate::modules::games::domain::models::GameType::Gimi,
            path: temp.path().to_str().unwrap(),
            mods_path: Some(root.to_str().unwrap()),
        },
    )
    .await
    .unwrap();
    let config = ConfigService::new_for_test_async(ctx.pool.clone()).await;
    let epoch = projection_source_epoch(&config, "capture-root").unwrap();
    ctx.pool.close().await;
    checkpoint_current_projection(&config, &ctx.pool, "capture-root", &epoch, None)
        .await
        .unwrap();
    std::fs::rename(&root, temp.path().join("Previous Mods")).unwrap();
    std::fs::create_dir(&root).unwrap();
    let error = checkpoint_current_projection(&config, &ctx.pool, "capture-root", &epoch, None)
        .await
        .expect_err("no-pending capture must not accept a replacement root");
    assert!(error.to_string().contains("root identity changed"));
}

fn pending_toggle(steps: Vec<OperationStep>) -> Operation {
    Operation {
        id: "pending-toggle".to_string(),
        kind: "workspace-switch".to_string(),
        game_id: "game-1".to_string(),
        source_epoch: None,
        created_at: String::new(),
        status: OperationStatus::DiskCommitted,
        steps,
        database_projection_status: DatabaseProjectionStatus::NotStarted,
        last_error: None,
        disk_revision: Some(1),
    }
}

fn applied_rename(old: &str, new: &str) -> OperationStep {
    OperationStep {
        sequence: 0,
        kind: MutationStepKind::Rename,
        old_path: Some(old.into()),
        new_path: Some(new.into()),
        stage_path: None,
        expected_identity: Some("physical-folder".to_string()),
        source_root_identity: None,
        status: StepStatus::Applied,
    }
}

#[test]
fn rapid_and_bulk_toggle_projection_uses_complete_union_scope() {
    let first = pending_toggle(vec![applied_rename("Mods/DISABLED A", "Mods/A")]);
    let second = pending_toggle(vec![applied_rename("Mods/DISABLED B", "Mods/B")]);
    assert_eq!(
        trusted_projection_paths(&[first, second]),
        Some(vec![
            "Mods/DISABLED A".to_string(),
            "Mods/A".to_string(),
            "Mods/DISABLED B".to_string(),
            "Mods/B".to_string(),
        ])
    );

    let bulk = pending_toggle(vec![
        applied_rename("Mods/DISABLED A", "Mods/A"),
        applied_rename("Mods/DISABLED B", "Mods/B"),
    ]);
    assert_eq!(trusted_projection_paths(&[bulk]).unwrap().len(), 4);
}

#[test]
fn trusted_projection_scope_rejects_unverified_or_missing_rename_steps() {
    let mut unverified = applied_rename("Mods/DISABLED A", "Mods/A");
    unverified.expected_identity = None;
    assert!(trusted_projection_paths(&[pending_toggle(vec![unverified])]).is_none());

    let mut skipped = applied_rename("Mods/DISABLED A", "Mods/A");
    skipped.status = StepStatus::Skipped;
    assert!(trusted_projection_paths(&[pending_toggle(vec![skipped])]).is_none());

    let mut skipped = applied_rename("Mods/DISABLED B", "Mods/B");
    skipped.status = StepStatus::Skipped;
    assert_eq!(
        trusted_projection_paths(&[pending_toggle(vec![
            applied_rename("Mods/DISABLED A", "Mods/A"),
            skipped,
        ])]),
        Some(vec!["Mods/DISABLED A".to_string(), "Mods/A".to_string()])
    );

    let mut missing_target = applied_rename("Mods/DISABLED A", "Mods/A");
    missing_target.new_path = None;
    assert!(trusted_projection_paths(&[pending_toggle(vec![missing_target])]).is_none());
}

#[test]
fn proven_projection_events_follow_child_reverse_after_parent_move_and_keep_bulk_sibling() {
    use crate::modules::library::application::mods::core_ops::plan_toggle_rename;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let child = root.join("Alice/Blue");
    let sibling = root.join("Bob/Red");
    std::fs::create_dir_all(&child).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();
    let mut steps = Vec::new();
    for (path, enable) in [
        (child.clone(), false),
        (root.join("Alice"), false),
        (sibling.clone(), false),
        (root.join("DISABLED Alice/DISABLED Blue"), true),
    ] {
        let plan = plan_toggle_rename(&path, enable).unwrap().unwrap();
        plan.apply("mod").unwrap();
        let mut step = applied_rename(
            &plan.old_path().to_string_lossy(),
            &plan.new_path().to_string_lossy(),
        );
        step.expected_identity = Some(plan.expected_identity().to_string());
        step.sequence = u32::try_from(steps.len()).unwrap();
        steps.push(step);
    }
    let events = proven_projection_events(&[pending_toggle(steps)], &root).unwrap();
    let physical_root = std::fs::canonicalize(&root).unwrap();
    let expected_child = physical_root.join(child.strip_prefix(&root).unwrap());
    let expected_sibling = physical_root.join(sibling.strip_prefix(&root).unwrap());
    assert!(events.iter().any(|event| matches!(event, crate::modules::workspace::application::scanner::watcher::ModWatchEvent::Renamed { from, to } if Path::new(from) == expected_child && Path::new(to) == physical_root.join("DISABLED Alice/Blue"))));
    assert!(events.iter().any(|event| matches!(event, crate::modules::workspace::application::scanner::watcher::ModWatchEvent::Renamed { from, to } if Path::new(from) == expected_sibling && Path::new(to) == physical_root.join("Bob/DISABLED Red"))));
}

#[tokio::test]
async fn projection_checkpoint_is_monotonic_and_isolated_by_source_epoch() {
    let pool = crate::test_utils::init_test_db().await.pool;
    assert_eq!(
        read_projection_checkpoint(&pool, "game", "root-a")
            .await
            .unwrap(),
        0
    );
    persist_projection_checkpoint(&pool, "game", "root-a", 42)
        .await
        .unwrap();
    persist_projection_checkpoint(&pool, "game", "root-a", 17)
        .await
        .unwrap();
    assert_eq!(
        read_projection_checkpoint(&pool, "game", "root-a")
            .await
            .unwrap(),
        42
    );
    assert_eq!(
        read_projection_checkpoint(&pool, "game", "root-b")
            .await
            .unwrap(),
        0
    );
}

#[test]
fn legacy_projection_requires_the_final_physical_lineage() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let old = root.join("A");
    let disabled = root.join("DISABLED A");
    std::fs::create_dir_all(&old).unwrap();
    let identity = crate::platform::fs::file_utils::filesystem_identity(&old).unwrap();
    let mut first = applied_rename(&old.to_string_lossy(), &disabled.to_string_lossy());
    first.expected_identity = Some(identity.clone());
    let mut second = applied_rename(&disabled.to_string_lossy(), &old.to_string_lossy());
    second.expected_identity = Some(identity);
    let operations = [pending_toggle(vec![first]), pending_toggle(vec![second])];
    assert!(legacy_projection_matches_root(&operations, &root));
    assert!(!legacy_projection_matches_root(
        &operations,
        &temp.path().join("Other")
    ));
    std::fs::rename(&old, root.join("Original")).unwrap();
    std::fs::create_dir(&old).unwrap();
    assert!(!legacy_projection_matches_root(&operations, &root));
}

#[test]
fn projection_retry_is_bounded_without_acknowledging_failure() {
    let mut failures = 0;
    let mut delay = Duration::from_millis(75);
    for _ in 0..100 {
        delay = next_projection_retry_delay(&mut failures, delay);
    }
    assert_eq!(delay, Duration::from_secs(30));
}

async fn commit_fixture_toggle(
    coordinator: &MutationCoordinator,
    root: &Path,
    path: &Path,
) -> String {
    use crate::modules::library::api::mods::core_ops::plan_toggle_rename;
    use crate::modules::mutation::api::{OperationPlan, PlannedStep};
    let rename = plan_toggle_rename(path, false).unwrap().unwrap();
    let operation = coordinator
        .acquire_operation(
            OperationPlan::new(
                "workspace-switch",
                "game-1",
                vec![PlannedStep::rename(
                    0,
                    rename.old_path().to_path_buf(),
                    rename.new_path().to_path_buf(),
                )
                .with_expected_identity(Some(rename.expected_identity().to_owned()))],
            )
            .with_source_epoch(crate::platform::fs::file_utils::filesystem_identity(root).unwrap()),
        )
        .await
        .unwrap();
    rename.apply("mod").unwrap();
    operation.mark_step_applied(0).unwrap();
    operation.mark_disk_committed().unwrap();
    operation.operation_id().unwrap().to_owned()
}

#[tokio::test]
async fn rename_confirmation_preserves_unrelated_pending_projection_and_repair_clamp() {
    use crate::modules::games::domain::models::GameType;
    use crate::modules::mutation::journal::OperationJournal;
    use crate::modules::reconciliation::application::disk_reconcile::reconcile::{
        reconcile_disk_projection, ReconcileDiskProjectionRequest,
    };
    use crate::modules::reconciliation::application::disk_reconcile::types::{
        DiskReconcileReason, DiskReconcileScanScope,
    };
    use crate::platform::fs::operation_lock::OperationLock;
    use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};

    let pool = init_test_db().await.pool;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    for path in ["Alice/Old", "Alice/Blue", "Bob/Red"] {
        std::fs::create_dir_all(root.join(path)).unwrap();
        std::fs::write(
            root.join(path).join("mod.ini"),
            "[TextureOverride]\nhash = abc\n",
        )
        .unwrap();
    }
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "game-1",
            name: "Game",
            game_type: GameType::Gimi,
            path: temp.path().to_str().unwrap(),
            mods_path: root.to_str(),
        },
    )
    .await
    .unwrap();
    reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id: "game-1",
        mods_path: &root,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::ManualRepair,
        changed_paths: &[],
        force_full: true,
        watcher_events: None,
        path_hints: &[],
        trusted_mutation_scope: false,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .unwrap();
    sqlx::query(
        "UPDATE mods SET filesystem_identity = NULL WHERE game_id = ? AND actual_name = 'Old'",
    )
    .bind("game-1")
    .execute(&pool)
    .await
    .unwrap();
    std::fs::rename(root.join("Alice/Old"), root.join("Alice/New")).unwrap();
    let journal =
        std::sync::Arc::new(OperationJournal::open(temp.path().join("journal.json"), 16).unwrap());
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal);
    let alice = commit_fixture_toggle(&coordinator, &root, &root.join("Alice/Blue")).await;
    let bob = commit_fixture_toggle(&coordinator, &root, &root.join("Bob/Red")).await;
    let pending = coordinator.pending_disk_commits().unwrap();
    let paths = trusted_projection_paths(&pending).unwrap();
    let events = proven_projection_events(&pending, &root).unwrap();
    let result = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id: "game-1",
        mods_path: &root,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::InternalMutation,
        changed_paths: &paths,
        force_full: false,
        watcher_events: Some(&events),
        path_hints: &[],
        trusted_mutation_scope: true,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .unwrap();
    assert_eq!(result.status, DiskReconcileStatus::NeedsRenameConfirmation);
    let affected = rename_confirmation_operation_ids(&pending, &root, &result.rename_confirmations);
    assert_eq!(affected.as_slice(), std::slice::from_ref(&alice));
    let mut object_confirmation = result.rename_confirmations[0].clone();
    object_confirmation.kind = crate::modules::reconciliation::application::disk_reconcile::types::RenameConfirmationKind::Object;
    object_confirmation.scope_key = "__top_level__".to_owned();
    object_confirmation.previous_paths = vec!["Previous Alice".to_owned()];
    object_confirmation.current_paths = vec!["Alice".to_owned()];
    assert_eq!(
        rename_confirmation_operation_ids(&pending, &root, &[object_confirmation]),
        [alice.clone(), bob.clone()],
        "top-level object ambiguity protects all pending roots"
    );
    coordinator
        .isolate_projection_for_repair(&alice, "Alice rename confirmation")
        .unwrap();
    let remaining = coordinator.pending_disk_commits().unwrap();
    assert_eq!(
        remaining
            .iter()
            .map(|operation| &operation.id)
            .collect::<Vec<_>>(),
        [&bob]
    );
    let paths = trusted_projection_paths(&remaining).unwrap();
    let events = proven_projection_events(&remaining, &root).unwrap();
    let retry = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id: "game-1",
        mods_path: &root,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::InternalMutation,
        changed_paths: &paths,
        force_full: false,
        watcher_events: Some(&events),
        path_hints: &[],
        trusted_mutation_scope: true,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .unwrap();
    assert_eq!(retry.status, DiskReconcileStatus::Applied);
    assert_eq!(retry.scan_scope, DiskReconcileScanScope::Scoped);
    let bob_status: i64 =
        sqlx::query_scalar("SELECT status FROM mods WHERE game_id = ? AND actual_name = 'Red'")
            .bind("game-1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(bob_status, 0);
    coordinator.complete_disk_projection(&[bob]).unwrap();
    let epoch = crate::platform::fs::file_utils::filesystem_identity(&root).unwrap();
    let (hole, _) = coordinator
        .earliest_toggle_projection_repair("game-1", &epoch)
        .unwrap()
        .unwrap();
    assert_eq!(
        projection_revision_before_repairs(
            &coordinator,
            "game-1",
            &epoch,
            remaining[0].disk_revision
        )
        .unwrap(),
        Some(hole.saturating_sub(1))
    );
}

#[tokio::test]
async fn replacement_proof_is_terminal_repair_but_missing_target_is_retryable() {
    use crate::modules::mutation::journal::OperationJournal;
    use crate::platform::fs::operation_lock::OperationLock;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    for path in ["Alice/Blue", "Bob/Red"] {
        std::fs::create_dir_all(root.join(path)).unwrap();
        std::fs::write(root.join(path).join("marker.ini"), "owned payload").unwrap();
    }
    let journal =
        std::sync::Arc::new(OperationJournal::open(temp.path().join("journal.json"), 16).unwrap());
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let alice = commit_fixture_toggle(&coordinator, &root, &root.join("Alice/Blue")).await;
    let bob = commit_fixture_toggle(&coordinator, &root, &root.join("Bob/Red")).await;
    let pending = coordinator.pending_disk_commits().unwrap();
    assert!(matches!(
        projection_rename_proof(&pending, &root).unwrap(),
        ProjectionRenameProof::Proven(_)
    ));
    let target = root.join("Alice/DISABLED Blue");
    let original = temp.path().join("Original Blue");
    std::fs::rename(&target, &original).unwrap();
    assert!(projection_rename_proof(&pending, &root).is_err());
    assert_eq!(coordinator.pending_disk_commits().unwrap().len(), 2);
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("replacement.ini"), "external payload").unwrap();
    let ProjectionRenameProof::Replaced(affected) =
        projection_rename_proof(&pending, &root).unwrap()
    else {
        panic!("an existing foreign target must become actionable repair");
    };
    assert_eq!(affected.as_slice(), std::slice::from_ref(&alice));
    coordinator
        .isolate_projection_for_repair(&alice, "Projection target was replaced")
        .unwrap();
    let remaining = coordinator.pending_disk_commits().unwrap();
    assert_eq!(
        remaining
            .iter()
            .map(|operation| &operation.id)
            .collect::<Vec<_>>(),
        [&bob]
    );
    assert!(matches!(
        projection_rename_proof(&remaining, &root).unwrap(),
        ProjectionRenameProof::Proven(_)
    ));
    assert_eq!(
        std::fs::read_to_string(target.join("replacement.ini")).unwrap(),
        "external payload"
    );
    assert_eq!(
        std::fs::read_to_string(original.join("marker.ini")).unwrap(),
        "owned payload"
    );
    let child = root.join("Alice/Pack/Child");
    std::fs::create_dir_all(&child).unwrap();
    std::fs::write(child.join("marker.ini"), "owned child payload").unwrap();
    let child_id = commit_fixture_toggle(&coordinator, &root, &child).await;
    let parent_id = commit_fixture_toggle(&coordinator, &root, &root.join("Alice/Pack")).await;
    let parent_target = root.join("Alice/DISABLED Pack");
    let original_parent = temp.path().join("Original Pack");
    std::fs::rename(&parent_target, &original_parent).unwrap();
    std::fs::create_dir(&parent_target).unwrap();
    std::fs::write(
        parent_target.join("replacement.ini"),
        "external parent payload",
    )
    .unwrap();
    assert!(!parent_target.join("DISABLED Child").exists());
    let pending = coordinator.pending_disk_commits().unwrap();
    let ProjectionRenameProof::Replaced(affected) =
        projection_rename_proof(&pending, &root).unwrap()
    else {
        panic!("a replaced parent must invalidate its missing owned descendant");
    };
    assert_eq!(affected, [child_id.clone(), parent_id.clone()]);
    for id in &affected {
        coordinator
            .isolate_projection_for_repair(id, "Projection parent was replaced")
            .unwrap();
    }
    let remaining = coordinator.pending_disk_commits().unwrap();
    assert_eq!(
        remaining
            .iter()
            .map(|operation| &operation.id)
            .collect::<Vec<_>>(),
        [&bob]
    );
    assert!(matches!(
        projection_rename_proof(&remaining, &root).unwrap(),
        ProjectionRenameProof::Proven(_)
    ));
    assert_eq!(
        std::fs::read_to_string(original_parent.join("DISABLED Child/marker.ini")).unwrap(),
        "owned child payload"
    );
    assert_eq!(
        std::fs::read_to_string(parent_target.join("replacement.ini")).unwrap(),
        "external parent payload"
    );
    let epoch = crate::platform::fs::file_utils::filesystem_identity(&root).unwrap();
    assert!(coordinator
        .earliest_toggle_projection_repair("game-1", &epoch)
        .unwrap()
        .is_some());
    let entries = journal.entries();
    assert!(entries
        .iter()
        .all(|operation| operation.status == OperationStatus::DiskCommitted));
    for id in [child_id, parent_id] {
        let operation = entries.iter().find(|operation| operation.id == id).unwrap();
        assert_eq!(
            operation.database_projection_status,
            DatabaseProjectionStatus::NeedsRepair
        );
    }
}

#[tokio::test]
async fn same_epoch_repair_hole_prevents_acknowledging_a_later_completed_toggle() {
    use crate::modules::mutation::journal::{OperationJournal, OperationPlan, PlannedStep};
    use crate::platform::fs::operation_lock::OperationLock;
    use std::sync::Arc;
    let ctx = crate::test_utils::init_test_db().await;
    let temp = tempfile::tempdir().unwrap();
    let journal = Arc::new(OperationJournal::open(temp.path().join("journal.json"), 50).unwrap());
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let record = |epoch: &str| {
        let id = journal
            .plan_operation(
                OperationPlan::new(
                    "workspace-switch",
                    "game",
                    vec![PlannedStep::rename(
                        0,
                        temp.path().join("DISABLED A"),
                        temp.path().join("A"),
                    )],
                )
                .with_source_epoch(epoch.to_string()),
            )
            .unwrap();
        journal.mark_applying(&id).unwrap();
        journal.mark_step_applied(&id, 0).unwrap();
        let revision = journal.mark_disk_committed(&id).unwrap();
        (id, revision)
    };
    let (foreign, _) = record("foreign-root");
    coordinator
        .isolate_projection_for_repair(&foreign, "foreign repair")
        .unwrap();
    let (repair, hole) = record("current-root");
    coordinator
        .isolate_projection_for_repair(&repair, "owned row needs repair")
        .unwrap();
    let (later, later_revision) = record("current-root");
    coordinator.complete_disk_projection(&[later]).unwrap();
    assert!(later_revision > hole);
    assert_eq!(
        coordinator
            .earliest_toggle_projection_repair("game", "current-root")
            .unwrap(),
        Some((hole, "owned row needs repair".to_string()))
    );
    let acknowledged = projection_revision_before_repairs(
        &coordinator,
        "game",
        "current-root",
        Some(later_revision),
    )
    .unwrap()
    .unwrap();
    persist_projection_checkpoint(&ctx.pool, "game", "current-root", acknowledged)
        .await
        .unwrap();
    assert!(
        read_projection_checkpoint(&ctx.pool, "game", "current-root")
            .await
            .unwrap()
            < hole
    );
    assert_eq!(
        projection_revision_before_repairs(&coordinator, "game", "new-root", Some(later_revision))
            .unwrap(),
        Some(later_revision)
    );
}
