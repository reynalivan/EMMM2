use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tempfile::tempdir;

use super::coordinator::MutationCoordinator;
use super::journal::{
    DatabaseProjectionStatus, MutationStepKind, OperationJournal, OperationPlan, OperationStatus,
    PlannedStep, StepStatus,
};
use super::recovery::{RecoveryRoots, RecoveryRunner};
use crate::platform::fs::operation_lock::OperationLock;

const TEST_HISTORY_LIMIT: usize = 16;
const GAME_ID: &str = "game-1";

fn open_journal(path: &Path) -> Arc<OperationJournal> {
    Arc::new(OperationJournal::open(path, TEST_HISTORY_LIMIT).unwrap())
}

#[test]
fn legacy_rename_step_kind_deserializes_to_typed_variant() {
    let step: super::journal::OperationStep = serde_json::from_value(serde_json::json!({
        "sequence": 0,
        "kind": "rename",
        "old_path": "C:/Mods/Old",
        "new_path": "C:/Mods/New",
        "stage_path": null,
        "status": "Planned"
    }))
    .unwrap();

    assert_eq!(step.kind, MutationStepKind::Rename);
}

fn rename_plan(old_path: &Path, new_path: &Path) -> OperationPlan {
    OperationPlan::new(
        "rename-mod",
        GAME_ID,
        vec![PlannedStep::rename(
            0,
            old_path.to_path_buf(),
            new_path.to_path_buf(),
        )],
    )
}

fn recovery_runner(
    journal: Arc<OperationJournal>,
    game_root: &Path,
    staging_root: &Path,
) -> RecoveryRunner {
    std::fs::create_dir_all(staging_root).unwrap();
    RecoveryRunner::new(
        journal,
        RecoveryRoots::new(
            HashMap::from([(GAME_ID.to_string(), game_root.to_path_buf())]),
            staging_root.to_path_buf(),
        ),
    )
}

#[tokio::test]
async fn direct_and_coordinator_consumers_contend_on_shared_lock() {
    let temp = tempdir().unwrap();
    let lock = OperationLock::new();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(lock.clone(), journal);
    let _direct_guard = lock.acquire().await.unwrap();

    let result = coordinator
        .acquire_exempt(super::coordinator::MutationExemption::Reconciliation)
        .await;

    assert!(result.is_err(), "both consumers must use the same mutex");
}

#[tokio::test]
async fn journal_survives_reopen_with_explicit_commit() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let old_path = game_root.join("Old");
    let new_path = game_root.join("New");
    std::fs::create_dir_all(&old_path).unwrap();
    let path = temp.path().join("journal.json");
    let journal = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    let id = journal
        .plan_operation(rename_plan(&old_path, &new_path))
        .unwrap();
    journal.mark_applying(&id).unwrap();
    std::fs::rename(&old_path, &new_path).unwrap();
    journal.mark_step_applied(&id, 0).unwrap();
    journal.mark_db_committed(&id).unwrap();
    journal.complete(&id).unwrap();
    drop(journal);

    let reopened = OperationJournal::open(path, TEST_HISTORY_LIMIT).unwrap();
    let entries = reopened.entries();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, id);
    assert_eq!(entries[0].kind, "rename-mod");
    assert_eq!(entries[0].status, OperationStatus::Completed);
    assert_eq!(
        entries[0].database_projection_status,
        DatabaseProjectionStatus::Committed
    );
    assert_eq!(entries[0].steps[0].status, StepStatus::Applied);
}

#[tokio::test]
async fn skipped_collision_can_commit_without_changing_either_path() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let source = game_root.join("Source");
    let target = game_root.join("Target");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&target).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let guard = coordinator
        .acquire_operation(rename_plan(&source, &target))
        .await
        .unwrap();

    guard.mark_step_skipped(0).unwrap();
    guard.mark_db_committed().unwrap();
    drop(guard);

    recovery_runner(journal.clone(), &game_root, &staging_root)
        .run_recovery()
        .await
        .unwrap();

    assert!(source.exists());
    assert!(target.exists());
    assert_eq!(journal.entries()[0].status, OperationStatus::Completed);
    assert_eq!(journal.entries()[0].steps[0].status, StepStatus::Skipped);
}

#[tokio::test]
async fn skipped_collision_rolls_back_without_path_classification() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let source = game_root.join("Source");
    let target = game_root.join("Target");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&target).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let guard = coordinator
        .acquire_operation(rename_plan(&source, &target))
        .await
        .unwrap();

    guard.mark_step_skipped(0).unwrap();
    drop(guard);

    recovery_runner(journal.clone(), &game_root, &staging_root)
        .run_recovery()
        .await
        .unwrap();

    assert!(source.exists());
    assert!(target.exists());
    assert_eq!(journal.entries()[0].status, OperationStatus::RolledBack);
    assert_eq!(journal.entries()[0].steps[0].status, StepStatus::Skipped);
}

#[tokio::test]
async fn dropping_guard_without_terminal_action_leaves_operation_recoverable() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let old_path = game_root.join("Old");
    let new_path = game_root.join("New");
    std::fs::create_dir_all(&old_path).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());

    let guard = coordinator
        .acquire_operation(rename_plan(&old_path, &new_path))
        .await
        .unwrap();
    let operation_id = guard.operation_id().unwrap().to_string();
    assert!(coordinator.task_registry().contains(&operation_id).unwrap());

    drop(guard);

    assert!(!coordinator.task_registry().contains(&operation_id).unwrap());
    assert_eq!(journal.entries()[0].status, OperationStatus::Applying);
}

#[tokio::test]
async fn explicit_failure_never_records_completed() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let old_path = game_root.join("Old");
    let new_path = game_root.join("New");
    std::fs::create_dir_all(&old_path).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let guard = coordinator
        .acquire_operation(rename_plan(&old_path, &new_path))
        .await
        .unwrap();

    guard.fail("rename failed").unwrap();

    let operation = &journal.entries()[0];
    assert_eq!(operation.status, OperationStatus::FailedNeedsRepair);
    assert_eq!(operation.last_error.as_deref(), Some("rename failed"));
}

#[tokio::test]
async fn crash_before_first_rename_rolls_back_without_disk_change() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let old_path = game_root.join("Old");
    let new_path = game_root.join("New");
    std::fs::create_dir_all(&old_path).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    drop(
        coordinator
            .acquire_operation(rename_plan(&old_path, &new_path))
            .await
            .unwrap(),
    );

    recovery_runner(journal.clone(), &game_root, &staging_root)
        .run_recovery()
        .await
        .unwrap();

    assert!(old_path.exists());
    assert!(!new_path.exists());
    assert_eq!(journal.entries()[0].status, OperationStatus::RolledBack);
}

#[tokio::test]
async fn crash_after_one_rename_rolls_back_applied_steps_in_reverse() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let old_a = game_root.join("OldA");
    let new_a = game_root.join("NewA");
    let old_b = game_root.join("OldB");
    let new_b = game_root.join("NewB");
    std::fs::create_dir_all(&old_a).unwrap();
    std::fs::create_dir_all(&old_b).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let guard = coordinator
        .acquire_operation(OperationPlan::new(
            "bulk-rename",
            GAME_ID,
            vec![
                PlannedStep::rename(0, old_a.clone(), new_a.clone()),
                PlannedStep::rename(1, old_b.clone(), new_b.clone()),
            ],
        ))
        .await
        .unwrap();
    std::fs::rename(&old_a, &new_a).unwrap();
    guard.mark_step_applied(0).unwrap();
    drop(guard);

    recovery_runner(journal.clone(), &game_root, &staging_root)
        .run_recovery()
        .await
        .unwrap();

    assert!(old_a.exists());
    assert!(old_b.exists());
    assert!(!new_a.exists());
    assert!(!new_b.exists());
    assert_eq!(journal.entries()[0].status, OperationStatus::RolledBack);
}

#[tokio::test]
async fn crash_after_all_renames_rolls_back_when_projection_is_not_committed() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let old_path = game_root.join("Old");
    let new_path = game_root.join("New");
    std::fs::create_dir_all(&old_path).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let guard = coordinator
        .acquire_operation(rename_plan(&old_path, &new_path))
        .await
        .unwrap();
    std::fs::rename(&old_path, &new_path).unwrap();
    guard.mark_step_applied(0).unwrap();
    drop(guard);

    recovery_runner(journal.clone(), &game_root, &staging_root)
        .run_recovery()
        .await
        .unwrap();

    assert!(old_path.exists());
    assert!(!new_path.exists());
    assert_eq!(journal.entries()[0].status, OperationStatus::RolledBack);
}

#[tokio::test]
async fn crash_after_db_commit_finalizes_without_rolling_back_disk() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let old_path = game_root.join("Old");
    let new_path = game_root.join("New");
    std::fs::create_dir_all(&old_path).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let guard = coordinator
        .acquire_operation(rename_plan(&old_path, &new_path))
        .await
        .unwrap();
    std::fs::rename(&old_path, &new_path).unwrap();
    guard.mark_step_applied(0).unwrap();
    guard.mark_db_committed().unwrap();
    drop(guard);

    recovery_runner(journal.clone(), &game_root, &staging_root)
        .run_recovery()
        .await
        .unwrap();

    assert!(!old_path.exists());
    assert!(new_path.exists());
    assert_eq!(journal.entries()[0].status, OperationStatus::Completed);
}

#[tokio::test]
async fn crash_after_hardlink_creation_restores_backup_when_db_is_uncommitted() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let keeper = game_root.join("Keeper/file.ini");
    let target = game_root.join("Target/file.ini");
    let backup = game_root.join("Target/.file.ini.emmm-hardlink-backup");
    std::fs::create_dir_all(keeper.parent().unwrap()).unwrap();
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&keeper, b"same").unwrap();
    std::fs::write(&target, b"same").unwrap();
    let identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&keeper).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let guard = coordinator
        .acquire_operation(OperationPlan::new(
            "hardlink",
            GAME_ID,
            vec![PlannedStep::hardlink_replace(
                0,
                keeper.clone(),
                target.clone(),
                backup.clone(),
                identity,
            )],
        ))
        .await
        .unwrap();
    std::fs::rename(&target, &backup).unwrap();
    std::fs::hard_link(&keeper, &target).unwrap();
    guard.mark_step_applied(0).unwrap();
    drop(guard);

    recovery_runner(journal.clone(), &game_root, &staging_root)
        .run_recovery()
        .await
        .unwrap();

    assert_eq!(std::fs::read(&target).unwrap(), b"same");
    assert!(!backup.exists());
    assert_ne!(
        crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&keeper),
        crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&target)
    );
    assert_eq!(journal.entries()[0].status, OperationStatus::RolledBack);
}

#[tokio::test]
async fn crash_after_hardlink_db_commit_keeps_link_and_removes_backup() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let keeper = game_root.join("Keeper/file.ini");
    let target = game_root.join("Target/file.ini");
    let backup = game_root.join("Target/.file.ini.emmm-hardlink-backup");
    std::fs::create_dir_all(keeper.parent().unwrap()).unwrap();
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&keeper, b"same").unwrap();
    std::fs::write(&target, b"same").unwrap();
    let identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&keeper).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let guard = coordinator
        .acquire_operation(OperationPlan::new(
            "hardlink",
            GAME_ID,
            vec![PlannedStep::hardlink_replace(
                0,
                keeper.clone(),
                target.clone(),
                backup.clone(),
                identity.clone(),
            )],
        ))
        .await
        .unwrap();
    std::fs::rename(&target, &backup).unwrap();
    std::fs::hard_link(&keeper, &target).unwrap();
    guard.mark_step_applied(0).unwrap();
    guard.mark_db_committed().unwrap();
    drop(guard);

    recovery_runner(journal.clone(), &game_root, &staging_root)
        .run_recovery()
        .await
        .unwrap();

    assert_eq!(
        crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&target).as_deref(),
        Some(identity.as_str())
    );
    assert!(!backup.exists());
    assert_eq!(journal.entries()[0].status, OperationStatus::Completed);
}

#[tokio::test]
async fn crash_after_quarantine_rename_restores_the_source() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let source = game_root.join("Object");
    let quarantine = game_root.join(".emmm-object-delete-test");
    std::fs::create_dir_all(&source).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let guard = coordinator
        .acquire_operation(OperationPlan::new(
            "object-delete",
            GAME_ID,
            vec![PlannedStep::quarantine(
                0,
                source.clone(),
                quarantine.clone(),
            )],
        ))
        .await
        .unwrap();
    std::fs::rename(&source, &quarantine).unwrap();
    guard.mark_step_applied(0).unwrap();
    drop(guard);

    recovery_runner(journal.clone(), &game_root, &staging_root)
        .run_recovery()
        .await
        .unwrap();

    assert!(source.exists());
    assert!(!quarantine.exists());
    assert_eq!(journal.entries()[0].status, OperationStatus::RolledBack);
}

#[tokio::test]
async fn ambiguous_disk_state_isolated_without_guessing() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let old_path = game_root.join("Old");
    let new_path = game_root.join("New");
    std::fs::create_dir_all(&old_path).unwrap();
    std::fs::create_dir_all(&new_path).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    drop(
        coordinator
            .acquire_operation(rename_plan(&old_path, &new_path))
            .await
            .unwrap(),
    );

    recovery_runner(journal.clone(), &game_root, &staging_root)
        .run_recovery()
        .await
        .unwrap();

    assert!(old_path.exists());
    assert!(new_path.exists());
    assert_eq!(
        journal.entries()[0].status,
        OperationStatus::FailedNeedsRepair
    );
}

#[tokio::test]
async fn partial_rollback_failure_records_repair_details() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    std::fs::create_dir_all(&game_root).unwrap();
    let blocked_parent = game_root.join("blocked");
    std::fs::write(&blocked_parent, b"not a directory").unwrap();
    let old_a = blocked_parent.join("OldA");
    let new_a = game_root.join("NewA");
    let old_b = game_root.join("OldB");
    let new_b = game_root.join("NewB");
    std::fs::create_dir_all(&new_a).unwrap();
    std::fs::create_dir_all(&new_b).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    drop(
        coordinator
            .acquire_operation(OperationPlan::new(
                "bulk-rename",
                GAME_ID,
                vec![
                    PlannedStep::rename(0, old_a, new_a.clone()),
                    PlannedStep::rename(1, old_b.clone(), new_b.clone()),
                ],
            ))
            .await
            .unwrap(),
    );

    recovery_runner(journal.clone(), &game_root, &staging_root)
        .run_recovery()
        .await
        .unwrap();

    let operation = &journal.entries()[0];
    assert_eq!(operation.status, OperationStatus::FailedNeedsRepair);
    assert!(operation
        .last_error
        .as_deref()
        .unwrap()
        .contains("Rollback failed at step 0"));
    assert!(old_b.exists(), "later step must already be compensated");
    assert!(!new_b.exists());
    assert!(
        new_a.exists(),
        "failed step must remain available for repair"
    );
}
