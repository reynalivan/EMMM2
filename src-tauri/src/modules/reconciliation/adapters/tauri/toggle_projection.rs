use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, State};

use crate::modules::mutation::api::MutationCoordinator;
use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcileResult, DiskReconcileStatus,
};
use crate::modules::settings::api::config::ConfigService;
use crate::modules::workspace::api::scanner::watcher::WatcherState;
use crate::shared::errors::AppError;

static RUNNING_SWITCH_PROJECTIONS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn is_toggle_projection_operation(
    operation: &crate::modules::mutation::journal::Operation,
    game_id: &str,
) -> bool {
    operation.game_id == game_id
        && matches!(operation.kind.as_str(), "workspace-switch" | "bulk-toggle")
}

#[cfg(test)]
mod tests {
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
                || async {
                    panic!("an unavailable source or confirmation is not a full-scan request")
                },
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
                game_type: crate::modules::games::domain::models::GameType::GIMI,
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
                .with_source_epoch(
                    crate::platform::fs::file_utils::filesystem_identity(root).unwrap(),
                ),
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
                game_type: GameType::GIMI,
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
        let journal = std::sync::Arc::new(
            OperationJournal::open(temp.path().join("journal.json"), 16).unwrap(),
        );
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
        let affected =
            rename_confirmation_operation_ids(&pending, &root, &result.rename_confirmations);
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
        let journal = std::sync::Arc::new(
            OperationJournal::open(temp.path().join("journal.json"), 16).unwrap(),
        );
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
        let journal =
            Arc::new(OperationJournal::open(temp.path().join("journal.json"), 50).unwrap());
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
            projection_revision_before_repairs(
                &coordinator,
                "game",
                "new-root",
                Some(later_revision)
            )
            .unwrap(),
            Some(later_revision)
        );
    }
}

use crate::modules::reconciliation::application::toggle_projection::{
    legacy_projection_matches_root, projection_rename_proof, rename_confirmation_operation_ids,
    settle_repaired_projection_operations, trusted_projection_paths,
    unprojected_toggle_operation_ids, ProjectionRenameProof,
};

#[cfg(test)]
use crate::modules::reconciliation::application::toggle_projection::proven_projection_events;

enum ToggleProjectionAttempt {
    Reconciled(Box<DiskReconcileResult>, Vec<String>),
    Replaced(Vec<String>),
}

#[derive(Clone, serde::Serialize)]
struct WorkspaceSwitchProjected {
    game_id: String,
    source_epoch: String,
    disk_revision: u64,
}

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct WorkspaceSwitchSnapshot {
    game_id: String,
    source_epoch: String,
    disk_revision: u64,
    projected_revision: u64,
    projection_repair_reason: Option<String>,
}

pub(crate) fn projection_source_epoch(
    config: &ConfigService,
    game_id: &str,
) -> Result<String, AppError> {
    let root = config
        .mods_root_for(game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(
        &root,
    )
    .ok_or_else(|| AppError::Io(format!("Mods root is unavailable: {}", root.display())))
}

pub(crate) fn ensure_projection_epoch(
    config: &ConfigService,
    game_id: &str,
    expected: &str,
) -> Result<(), AppError> {
    if projection_source_epoch(config, game_id)? != expected {
        return Err(AppError::Io(
            "Mods root identity changed during the operation; refresh this game before retrying"
                .to_string(),
        ));
    }
    Ok(())
}

async fn read_projection_checkpoint(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    source_epoch: &str,
) -> Result<u64, AppError> {
    let revision = sqlx::query_scalar::<_, i64>(
        "SELECT projected_revision FROM workspace_projection_checkpoints WHERE game_id = ? AND source_epoch = ?",
    )
    .bind(game_id)
    .bind(source_epoch)
    .fetch_optional(pool)
    .await?;
    Ok(revision
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(0))
}

async fn persist_projection_checkpoint(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    source_epoch: &str,
    revision: u64,
) -> Result<(), AppError> {
    let revision = i64::try_from(revision)
        .map_err(|_| AppError::Validation("Disk revision exceeds SQLite range".to_string()))?;
    sqlx::query(
        "INSERT INTO workspace_projection_checkpoints (game_id, source_epoch, projected_revision) VALUES (?, ?, ?) \
         ON CONFLICT(game_id, source_epoch) DO UPDATE SET projected_revision = MAX(projected_revision, excluded.projected_revision)",
    )
    .bind(game_id)
    .bind(source_epoch)
    .bind(revision)
    .execute(pool)
    .await?;
    Ok(())
}

async fn checkpoint_current_projection(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    expected_epoch: &str,
    revision: Option<u64>,
) -> Result<(), AppError> {
    ensure_projection_epoch(config, game_id, expected_epoch)?;
    if let Some(revision) = revision {
        persist_projection_checkpoint(pool, game_id, expected_epoch, revision).await?;
        ensure_projection_epoch(config, game_id, expected_epoch)?;
    }
    Ok(())
}

pub(crate) async fn complete_reconciled_toggle_projection(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    coordinator: &MutationCoordinator,
    game_id: &str,
    expected_epoch: &str,
    pending_ids: &[String],
    revision: Option<u64>,
) -> Result<(), AppError> {
    let pending = coordinator.pending_disk_commits()?;
    let current = pending
        .iter()
        .filter(|operation| pending_ids.contains(&operation.id))
        .cloned()
        .collect::<Vec<_>>();
    let root = app
        .state::<ConfigService>()
        .mods_root_for(game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    if current.iter().any(|operation| {
        operation
            .source_epoch
            .as_deref()
            .is_some_and(|epoch| epoch != expected_epoch)
    }) || (current
        .iter()
        .any(|operation| operation.source_epoch.is_none())
        && !legacy_projection_matches_root(&current, &root))
    {
        return Err(AppError::Io(
            "Pending disk commits belong to an unverified root epoch; recovery is required"
                .to_string(),
        ));
    }
    let current = settle_repaired_projection_operations(
        pool,
        coordinator,
        game_id,
        expected_epoch,
        &root,
        &current,
    )
    .await?;
    if let Some((hole, reason)) =
        coordinator.earliest_toggle_projection_repair(game_id, expected_epoch)?
    {
        return Err(AppError::Io(format!(
            "Disk projection requires repair at revision {hole}: {reason}"
        )));
    }
    let revision = revision
        .into_iter()
        .chain(
            current
                .iter()
                .filter_map(|operation| operation.disk_revision),
        )
        .max();
    let completed_revision = coordinator
        .toggle_projection_lineage_in_epoch(game_id, expected_epoch)?
        .into_iter()
        .filter(|operation| {
            operation.status == crate::modules::mutation::journal::OperationStatus::Completed
        })
        .filter_map(|operation| operation.disk_revision)
        .max()
        .unwrap_or(0);
    let revision = Some(revision.unwrap_or(0).max(completed_revision));
    checkpoint_current_projection(
        &app.state::<ConfigService>(),
        pool,
        game_id,
        expected_epoch,
        revision,
    )
    .await?;
    coordinator.complete_disk_projection(pending_ids)?;
    publish_completed_toggle_projection(app, pool, game_id, expected_epoch, revision);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_switch_snapshot(
    game_id: String,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    coordinator: State<'_, MutationCoordinator>,
) -> Result<WorkspaceSwitchSnapshot, AppError> {
    let source_epoch = projection_source_epoch(config.inner(), &game_id)?;
    let mut projected_revision =
        read_projection_checkpoint(pool.inner(), &game_id, &source_epoch).await?;
    let repair = coordinator.earliest_toggle_projection_repair(&game_id, &source_epoch)?;
    if let Some((revision, _)) = &repair {
        projected_revision = projected_revision.min(revision.saturating_sub(1));
    }
    let disk_revision =
        coordinator.latest_toggle_disk_revision_in_epoch(&game_id, Some(&source_epoch))?;
    ensure_projection_epoch(config.inner(), &game_id, &source_epoch)?;
    Ok(WorkspaceSwitchSnapshot {
        game_id,
        source_epoch,
        disk_revision,
        projected_revision,
        projection_repair_reason: repair.map(|(_, reason)| reason),
    })
}

fn running_switch_projections() -> &'static Mutex<HashSet<String>> {
    RUNNING_SWITCH_PROJECTIONS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn projection_revision_before_repairs(
    coordinator: &MutationCoordinator,
    game_id: &str,
    epoch: &str,
    revision: Option<u64>,
) -> Result<Option<u64>, AppError> {
    Ok(
        match coordinator.earliest_toggle_projection_repair(game_id, epoch)? {
            Some((hole, _)) => revision.map(|revision| revision.min(hole.saturating_sub(1))),
            None => revision,
        },
    )
}

pub(crate) fn publish_completed_toggle_projection(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    source_epoch: &str,
    disk_revision: Option<u64>,
) {
    if disk_revision.is_none() {
        return;
    }
    if let Err(error) =
        ensure_projection_epoch(&app.state::<ConfigService>(), game_id, source_epoch)
    {
        log::warn!("Discarding stale-root projection publication for '{game_id}': {error}");
        return;
    }
    crate::modules::reconciliation::api::enqueue_runtime_sync_scoped(
        app,
        pool,
        game_id,
        crate::modules::reconciliation::api::RuntimeSyncCause::EffectiveModsChanged,
        crate::modules::reconciliation::api::RuntimeSyncRequest::Full,
    );
    if let Some(disk_revision) = disk_revision {
        if let Err(error) = app.emit(
            "workspace_switch:projected",
            WorkspaceSwitchProjected {
                game_id: game_id.to_string(),
                source_epoch: source_epoch.to_string(),
                disk_revision,
            },
        ) {
            log::warn!("Could not emit workspace projection revision for '{game_id}': {error}");
        }
    }
}

pub(crate) fn queue_toggle_projection(
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    game_id: String,
) {
    app.state::<crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState>()
        .projection_wakeup(&game_id).notify_one();
    let mut running = crate::shared::sync::lock(running_switch_projections());
    if !running.insert(game_id.clone()) {
        return;
    }
    drop(running);
    tauri::async_runtime::spawn(async move {
        run_workspace_switch_projection(&app, &pool, &game_id).await;
    });
}

async fn project_toggle_scope<S, SF, F, FF>(
    paths: Option<Vec<String>>,
    scoped: S,
    full: F,
) -> Result<DiskReconcileResult, AppError>
where
    S: FnOnce(Vec<String>) -> SF,
    SF: std::future::Future<Output = Result<DiskReconcileResult, AppError>>,
    F: FnOnce() -> FF,
    FF: std::future::Future<Output = Result<DiskReconcileResult, AppError>>,
{
    match paths {
        Some(paths) => scoped(paths).await,
        None => full().await,
    }
}

async fn run_workspace_switch_projection(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
) {
    let mut retry_delay = Duration::from_millis(75);
    let mut failures = 0u32;
    let state = app.state::<crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState>();
    let wakeup = state.projection_wakeup(game_id);
    loop {
        tokio::select! {
            _ = tokio::time::sleep(retry_delay) => {},
            _ = wakeup.notified() => {},
        }
        let notified = wakeup.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        let coordinator = app.state::<MutationCoordinator>();
        let state = app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>();
        let lease_started = Instant::now();
        let game_guard = state.game_lock(game_id).lock_owned().await;
        let Some(operation_guard) = coordinator.inner_lock().try_acquire_for_reconcile() else {
            drop(game_guard);
            retry_delay = Duration::from_millis(75);
            continue;
        };
        let lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_reconcile_guard(game_guard, operation_guard);
        let lease_wait_ms = lease_started.elapsed().as_millis();
        let mut pending = match coordinator.pending_disk_commits() {
            Ok(pending) => pending
                .into_iter()
                .filter(|operation| is_toggle_projection_operation(operation, game_id))
                .collect::<Vec<_>>(),
            Err(error) => {
                log::error!(
                    "Could not inspect pending workspace projection for '{game_id}': {error}"
                );
                drop(lease);
                notified.await;
                retry_delay = Duration::from_millis(75);
                continue;
            }
        };
        if pending.is_empty() {
            drop(lease);
            let mut running = crate::shared::sync::lock(running_switch_projections());
            match coordinator.pending_disk_commits() {
                Ok(remaining)
                    if !remaining
                        .iter()
                        .any(|operation| is_toggle_projection_operation(operation, game_id)) =>
                {
                    running.remove(game_id);
                    return;
                }
                Ok(_) => continue,
                Err(error) => {
                    log::error!(
                        "Could not settle workspace projection worker for '{game_id}': {error}"
                    );
                    retry_delay = Duration::from_secs(1);
                    continue;
                }
            }
        }
        let projection_epoch = match projection_source_epoch(&app.state::<ConfigService>(), game_id)
        {
            Ok(epoch) => epoch,
            Err(error) => {
                log::warn!("Workspace projection source is unavailable for '{game_id}': {error}");
                drop(lease);
                notified.await;
                retry_delay = Duration::from_millis(75);
                continue;
            }
        };
        let Some(root) = app.state::<ConfigService>().mods_root_for(game_id) else {
            drop(lease);
            notified.await;
            continue;
        };
        let candidates = pending
            .iter()
            .filter(|operation| {
                operation
                    .source_epoch
                    .as_deref()
                    .is_none_or(|epoch| epoch == projection_epoch)
            })
            .cloned()
            .collect::<Vec<_>>();
        let legacy_valid = legacy_projection_matches_root(&candidates, &root);
        let mut isolation_failed = false;
        for operation in &pending {
            let belongs = operation
                .source_epoch
                .as_deref()
                .map_or(legacy_valid, |epoch| epoch == projection_epoch);
            if !belongs {
                if let Err(error) = coordinator.isolate_projection_for_repair(&operation.id, "Root epoch mismatch or unproven legacy identity; disk commit preserved for manual repair") {
                    log::error!("Could not isolate stale-root projection: {error}");
                    isolation_failed = true;
                }
            }
        }
        pending.retain(|operation| {
            operation
                .source_epoch
                .as_deref()
                .map_or(legacy_valid, |epoch| epoch == projection_epoch)
        });
        if pending.is_empty() {
            drop(lease);
            if isolation_failed {
                notified.await;
            }
            continue;
        }
        let pending_ids = pending
            .iter()
            .map(|operation| operation.id.clone())
            .collect::<Vec<_>>();
        let projected_revision = pending
            .iter()
            .filter_map(|operation| operation.disk_revision)
            .max();
        let authority_marker = app
            .try_state::<ConfigService>()
            .and_then(|config| config.mods_root_for(game_id))
            .and_then(|mods_root| {
                app.try_state::<WatcherState>()
                    .and_then(|watcher| watcher.current_session_for_root(&mods_root))
                    .and_then(|session| {
                        state
                            .authority_event_generation(game_id, session.generation())
                            .map(|generation| (mods_root, session.generation(), generation))
                    })
            });
        if authority_marker.is_none() {
            if let Err(error) = coordinator.note_disk_projection_failure(
                &pending_ids,
                "Projection parked: watcher coverage is unavailable",
            ) {
                log::error!("Could not record parked projection: {error}");
            }
            drop(lease);
            notified.await;
            retry_delay = Duration::from_millis(75);
            continue;
        }
        let watcher = app.state::<WatcherState>();
        let echo_watermark = watcher.suppressor.expected_echo_watermark();
        let trusted_authority =
            authority_marker
                .as_ref()
                .and_then(|(mods_root, watcher_session, _)| {
                    state.trusted_internal_mutation_evidence(game_id, mods_root, *watcher_session)
                });
        let changed_paths = pending
            .iter()
            .flat_map(|operation| &operation.steps)
            .flat_map(|step| [&step.old_path, &step.new_path])
            .flatten()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let scoped_paths = trusted_projection_paths(&pending);
        let scope_mode = if scoped_paths.is_some() {
            "scoped"
        } else {
            "full_unproven_scope"
        };
        let projection_started = Instant::now();
        log::debug!("WorkspaceProjection game_id={game_id} epoch={projection_epoch} disk_revision={projected_revision:?} operations={} mode={scope_mode} lease_wait_ms={lease_wait_ms}", pending_ids.len());
        let projection = tokio::select! {
            biased;
            _ = coordinator.inner_lock().wait_for_foreground_intent() => None,
            result = async {
                let proof_pending = pending.clone();
                let proof_root = root.clone();
                let proof = tokio::task::spawn_blocking(move || projection_rename_proof(&proof_pending, &proof_root)).await??;
                let rename_events = match proof {
                    ProjectionRenameProof::Proven(events) => events,
                    ProjectionRenameProof::Replaced(ids) => return Ok(ToggleProjectionAttempt::Replaced(ids)),
                };
                let result = project_toggle_scope(
                    scoped_paths,
                    |paths| crate::modules::reconciliation::application::disk_reconcile::emit::run_trusted_internal_disk_reconcile_under_lease(app, pool, game_id, paths, rename_events, &lease),
                    || crate::modules::reconciliation::application::disk_reconcile::emit::run_deferred_full_internal_disk_reconcile_under_lease(app, pool, game_id, &lease),
                ).await?;
                let invalid = if result.status.applied() {
                    unprojected_toggle_operation_ids(pool, game_id, &pending, &root).await?
                } else {
                    Vec::new()
                };
                Ok::<_, AppError>(ToggleProjectionAttempt::Reconciled(Box::new(result), invalid))
            } => Some(result),
        };
        let Some(projection) = projection else {
            log::debug!("WorkspaceProjection game_id={game_id} epoch={projection_epoch} disk_revision={projected_revision:?} outcome=yielded elapsed_ms={}", projection_started.elapsed().as_millis());
            drop(lease);
            retry_delay = Duration::from_millis(75);
            continue;
        };
        match projection {
            Ok(ToggleProjectionAttempt::Replaced(ids)) => {
                let message = "Owned projection target was replaced; verified disk commit preserved for repair";
                for id in ids {
                    if let Err(error) = coordinator.isolate_projection_for_repair(&id, message) {
                        log::error!(
                            "Could not retain replaced projection repair evidence: {error}"
                        );
                    }
                }
                drop(lease);
                retry_delay = Duration::from_millis(75);
            }
            Ok(ToggleProjectionAttempt::Reconciled(mut result, invalid)) => {
                log::debug!("WorkspaceProjection game_id={game_id} epoch={projection_epoch} disk_revision={projected_revision:?} outcome={:?} mode={scope_mode} elapsed_ms={}", result.status, projection_started.elapsed().as_millis());
                if result.status == DiskReconcileStatus::SourceUnavailable {
                    let message = result.error_message.as_deref().unwrap_or(
                        "Projection source is unavailable; verified disk changes remain committed",
                    );
                    if let Err(error) =
                        coordinator.note_disk_projection_failure(&pending_ids, message)
                    {
                        log::error!("Could not record unavailable projection source: {error}");
                    }
                    drop(lease);
                    retry_delay = next_projection_retry_delay(&mut failures, retry_delay);
                    continue;
                }
                if result.status == DiskReconcileStatus::NeedsRenameConfirmation {
                    let message = "Folder changes are committed on disk; resolve ambiguous rename ownership before synchronization can finish";
                    let affected = rename_confirmation_operation_ids(
                        &pending,
                        &root,
                        &result.rename_confirmations,
                    );
                    for id in &affected {
                        if let Err(error) = coordinator.isolate_projection_for_repair(id, message) {
                            log::error!(
                                "Could not retain rename-confirmation repair evidence: {error}"
                            );
                        }
                    }
                    drop(lease);
                    retry_delay = if affected.is_empty() {
                        next_projection_retry_delay(&mut failures, retry_delay)
                    } else {
                        Duration::from_millis(75)
                    };
                    continue;
                }
                if !invalid.is_empty() {
                    let message = "Owned disk rename is not indexed; resolve the affected folder conflict before capturing a collection";
                    log::warn!("Preserving disk commits with incomplete owned projection for '{game_id}': {message}");
                    for id in invalid {
                        if let Err(journal_error) =
                            coordinator.isolate_projection_for_repair(&id, message)
                        {
                            log::error!(
                                "Could not isolate incomplete owned projection: {journal_error}"
                            );
                        }
                    }
                    drop(lease);
                    retry_delay = Duration::from_millis(75);
                    continue;
                }
                let authority_accepted = if let Some(authority) = &trusted_authority {
                    state.mark_trusted_internal_mutation_reconciled(
                        authority,
                        &result,
                        &changed_paths,
                    )
                } else if let Some((mods_root, watcher_session, observed_generation)) =
                    &authority_marker
                {
                    state.mark_authority_reconciled(
                        game_id,
                        mods_root,
                        *watcher_session,
                        *observed_generation,
                        &result,
                        &changed_paths,
                    )
                } else {
                    false
                };
                if !authority_accepted {
                    log::debug!(
                        "Workspace disk projection for '{game_id}' awaits current watcher authority"
                    );
                    drop(lease);
                    retry_delay = next_projection_retry_delay(&mut failures, retry_delay);
                    continue;
                }
                result.warnings.retain(|warning| {
                    warning.kind
                        != crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileWarningKind::AuthorityPending
                });
                let acknowledged_revision = match projection_revision_before_repairs(
                    &coordinator,
                    game_id,
                    &projection_epoch,
                    projected_revision,
                ) {
                    Ok(revision) => revision,
                    Err(error) => {
                        log::error!(
                            "Could not inspect workspace projection repair boundary: {error}"
                        );
                        drop(lease);
                        retry_delay = next_projection_retry_delay(&mut failures, retry_delay);
                        continue;
                    }
                };
                let checkpoint = checkpoint_current_projection(
                    &app.state::<ConfigService>(),
                    pool,
                    game_id,
                    &projection_epoch,
                    acknowledged_revision,
                )
                .await;
                #[cfg(debug_assertions)]
                let settlement_started = Instant::now();
                let settlement =
                    checkpoint.and_then(|()| coordinator.complete_disk_projection(&pending_ids));
                #[cfg(debug_assertions)]
                log::info!("toggle projection settlement game_id={} operations={} success={} journal_snapshot_ms={}",
                    game_id, pending_ids.len(), settlement.is_ok(), settlement_started.elapsed().as_millis());
                if let Err(error) = settlement {
                    log::error!(
                        "Could not complete workspace disk projection for '{game_id}': {error}"
                    );
                    drop(lease);
                    retry_delay = next_projection_retry_delay(&mut failures, retry_delay);
                    continue;
                }
                if let Some(session) = watcher.current_session_for_root(&root) {
                    watcher
                        .suppressor
                        .mark_rename_echoes_reconciled_through(&session, echo_watermark);
                }
                match ensure_projection_epoch(
                    &app.state::<ConfigService>(),
                    game_id,
                    &projection_epoch,
                ) {
                    Ok(()) => {
                        if let Err(error) = app.emit("disk_reconcile:result", &result) {
                            log::warn!("Could not emit completed workspace projection for '{game_id}': {error}");
                        }
                    }
                    Err(error) => log::warn!(
                        "Discarding stale-root reconcile publication for '{game_id}': {error}"
                    ),
                }
                drop(lease);
                publish_completed_toggle_projection(
                    app,
                    pool,
                    game_id,
                    &projection_epoch,
                    acknowledged_revision,
                );
                retry_delay = Duration::from_millis(75);
                failures = 0;
            }
            Err(error) => {
                log::debug!("WorkspaceProjection game_id={game_id} epoch={projection_epoch} disk_revision={projected_revision:?} outcome=retry mode={scope_mode} elapsed_ms={}", projection_started.elapsed().as_millis());
                log::error!("Workspace disk projection remains pending for '{game_id}': {error}");
                if let Err(journal_error) =
                    coordinator.note_disk_projection_failure(&pending_ids, &error.to_string())
                {
                    log::error!("Could not record pending projection failure for '{game_id}': {journal_error}");
                }
                drop(lease);
                retry_delay = next_projection_retry_delay(&mut failures, retry_delay);
            }
        }
    }
}

fn next_projection_retry_delay(failures: &mut u32, previous: Duration) -> Duration {
    *failures = failures.saturating_add(1);
    if *failures >= 6 {
        Duration::from_secs(30)
    } else {
        (previous * 2).min(Duration::from_secs(5))
    }
}
