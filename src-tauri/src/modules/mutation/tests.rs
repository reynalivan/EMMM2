use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tempfile::tempdir;

use super::coordinator::MutationCoordinator;
use super::journal::{
    DatabaseProjectionStatus, MutationStepKind, OperationJournal, OperationPlan, OperationStatus,
    PlannedStep, StepSettlement, StepStatus,
};
use super::recovery::{RecoveryRoots, RecoveryRunner};
use crate::platform::fs::operation_lock::OperationLock;

const TEST_HISTORY_LIMIT: usize = 16;
const GAME_ID: &str = "game-1";
const BENCHMARK_SAMPLES: usize = 7;

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

fn batch_rename_plan(root: &Path, step_count: usize) -> OperationPlan {
    OperationPlan::new(
        "bulk-rename",
        GAME_ID,
        (0..step_count)
            .map(|sequence| {
                PlannedStep::rename(
                    sequence as u32,
                    root.join(format!("Old-{sequence}")),
                    root.join(format!("New-{sequence}")),
                )
            })
            .collect(),
    )
}

#[test]
fn batch_step_settlement_persists_once_and_reopens_atomically() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let journal = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    let id = journal
        .plan_operation(batch_rename_plan(temp.path(), 3))
        .unwrap();
    journal.mark_applying(&id).unwrap();
    journal.reset_persistence_metrics();

    journal
        .settle_steps(
            &id,
            &[
                (0, StepSettlement::Applied),
                (1, StepSettlement::Skipped),
                (2, StepSettlement::Applied),
            ],
        )
        .unwrap();

    assert_eq!(journal.persistence_metrics().writes, 1);
    let operation = &journal.entries()[0];
    assert_eq!(operation.status, OperationStatus::Applied);
    assert_eq!(operation.steps[0].status, StepStatus::Applied);
    assert_eq!(operation.steps[1].status, StepStatus::Skipped);
    assert_eq!(operation.steps[2].status, StepStatus::Applied);
    drop(journal);

    let reopened = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    let operation = &reopened.entries()[0];
    assert_eq!(operation.status, OperationStatus::Applied);
    assert_eq!(operation.steps[0].status, StepStatus::Applied);
    assert_eq!(operation.steps[1].status, StepStatus::Skipped);
    assert_eq!(operation.steps[2].status, StepStatus::Applied);
}

#[test]
fn invalid_batch_step_settlement_is_atomic() {
    let temp = tempdir().unwrap();
    let journal =
        OperationJournal::open(temp.path().join("journal.json"), TEST_HISTORY_LIMIT).unwrap();
    let id = journal
        .plan_operation(batch_rename_plan(temp.path(), 2))
        .unwrap();
    journal.mark_applying(&id).unwrap();
    journal.reset_persistence_metrics();

    assert!(journal
        .settle_steps(
            &id,
            &[(0, StepSettlement::Applied), (0, StepSettlement::Skipped)],
        )
        .is_err());
    assert!(journal
        .settle_steps(&id, &[(9, StepSettlement::Applied)])
        .is_err());

    assert_eq!(journal.persistence_metrics().writes, 0);
    let operation = &journal.entries()[0];
    assert_eq!(operation.status, OperationStatus::Applying);
    assert!(operation
        .steps
        .iter()
        .all(|step| step.status == StepStatus::Planned));
}

#[test]
fn rollback_step_settlement_persists_once() {
    let temp = tempdir().unwrap();
    let journal =
        OperationJournal::open(temp.path().join("journal.json"), TEST_HISTORY_LIMIT).unwrap();
    let id = journal
        .plan_operation(batch_rename_plan(temp.path(), 2))
        .unwrap();
    journal.mark_applying(&id).unwrap();
    journal
        .settle_steps(
            &id,
            &[(0, StepSettlement::Applied), (1, StepSettlement::Applied)],
        )
        .unwrap();
    journal.begin_rollback(&id).unwrap();
    journal.reset_persistence_metrics();

    journal
        .settle_steps(
            &id,
            &[
                (1, StepSettlement::RolledBack),
                (0, StepSettlement::RolledBack),
            ],
        )
        .unwrap();

    assert_eq!(journal.persistence_metrics().writes, 1);
    assert!(journal.entries()[0]
        .steps
        .iter()
        .all(|step| step.status == StepStatus::RolledBack));
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

#[derive(Clone, Copy)]
enum JournalReplaceCrashPoint {
    BeforeCanonicalMovesToRecovery,
    AfterCanonicalMovesToRecovery,
    AfterTempMovesToCanonical,
}

fn journal_artifact_path(path: &Path, label: &str, sequence: u32) -> std::path::PathBuf {
    path.with_file_name(format!(
        "{}.{}.test.{sequence}",
        path.file_name().unwrap().to_string_lossy(),
        label
    ))
}

fn arrange_atomic_replace_crash(
    crash_point: JournalReplaceCrashPoint,
    path: &Path,
    planned_snapshot: &Path,
    applying_snapshot: &Path,
) {
    let recovery = journal_artifact_path(path, "recover", 0);
    let temporary = journal_artifact_path(path, "tmp", 0);
    match crash_point {
        JournalReplaceCrashPoint::BeforeCanonicalMovesToRecovery => {
            std::fs::copy(planned_snapshot, path).unwrap();
            std::fs::copy(applying_snapshot, temporary).unwrap();
        }
        JournalReplaceCrashPoint::AfterCanonicalMovesToRecovery => {
            std::fs::remove_file(path).unwrap();
            std::fs::copy(planned_snapshot, recovery).unwrap();
            std::fs::copy(applying_snapshot, temporary).unwrap();
        }
        JournalReplaceCrashPoint::AfterTempMovesToCanonical => {
            std::fs::copy(applying_snapshot, path).unwrap();
            std::fs::copy(planned_snapshot, recovery).unwrap();
        }
    }
}

fn active_state_path(journal_path: &Path, operation_id: &str) -> std::path::PathBuf {
    journal_path.parent().unwrap().join(format!(
        "{}.active/{operation_id}.json",
        journal_path.file_name().unwrap().to_string_lossy()
    ))
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

#[test]
fn versioned_journal_recovers_all_atomic_replace_crash_points() {
    for crash_point in [
        JournalReplaceCrashPoint::BeforeCanonicalMovesToRecovery,
        JournalReplaceCrashPoint::AfterCanonicalMovesToRecovery,
        JournalReplaceCrashPoint::AfterTempMovesToCanonical,
    ] {
        let temp = tempdir().unwrap();
        let path = temp.path().join("journal.json");
        let journal = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
        let id = journal
            .plan_operation(rename_plan(
                &temp.path().join("Mods/Old"),
                &temp.path().join("Mods/New"),
            ))
            .unwrap();
        let planned_snapshot = temp.path().join("planned.snapshot");
        std::fs::copy(&path, &planned_snapshot).unwrap();
        journal.fail(&id, "repair required").unwrap();
        let failed_snapshot = temp.path().join("failed.snapshot");
        std::fs::copy(&path, &failed_snapshot).unwrap();
        drop(journal);

        arrange_atomic_replace_crash(crash_point, &path, &planned_snapshot, &failed_snapshot);

        let reopened = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
        let expected_status = match crash_point {
            JournalReplaceCrashPoint::BeforeCanonicalMovesToRecovery => OperationStatus::Planned,
            JournalReplaceCrashPoint::AfterCanonicalMovesToRecovery
            | JournalReplaceCrashPoint::AfterTempMovesToCanonical => {
                OperationStatus::FailedNeedsRepair
            }
        };
        assert_eq!(reopened.entries()[0].status, expected_status);
    }
}

#[test]
fn active_progress_does_not_rewrite_the_immutable_plan_snapshot() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let journal = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    let id = journal
        .plan_operation(rename_plan(
            &temp.path().join("Mods/Old"),
            &temp.path().join("Mods/New"),
        ))
        .unwrap();
    let planned_snapshot = std::fs::read(&path).unwrap();
    let progress_path = active_state_path(&path, &id);

    journal.mark_applying(&id).unwrap();
    journal.mark_step_applied(&id, 0).unwrap();

    assert_eq!(std::fs::read(&path).unwrap(), planned_snapshot);
    assert!(progress_path.exists());
    drop(journal);

    let reopened = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    let recovered = reopened.entries();
    assert_eq!(recovered[0].status, OperationStatus::Applied);
    assert_eq!(recovered[0].steps[0].status, StepStatus::Applied);

    reopened.mark_db_committed(&id).unwrap();
    reopened.complete(&id).unwrap();

    assert!(!progress_path.exists());
    assert_eq!(reopened.entries()[0].status, OperationStatus::Completed);
}

#[test]
fn active_progress_recovers_the_newest_valid_artifact() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let journal = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    let id = journal
        .plan_operation(rename_plan(
            &temp.path().join("Mods/Old"),
            &temp.path().join("Mods/New"),
        ))
        .unwrap();
    let progress_path = active_state_path(&path, &id);

    journal.mark_applying(&id).unwrap();
    let applying_state = temp.path().join("applying.progress");
    std::fs::copy(&progress_path, &applying_state).unwrap();
    journal.mark_step_applied(&id, 0).unwrap();
    let applied_state = temp.path().join("applied.progress");
    std::fs::copy(&progress_path, &applied_state).unwrap();
    drop(journal);

    std::fs::remove_file(&progress_path).unwrap();
    let recovery = journal_artifact_path(&progress_path, "recover", 0);
    let temporary = journal_artifact_path(&progress_path, "tmp", 0);
    std::fs::copy(&applying_state, recovery).unwrap();
    std::fs::copy(&applied_state, temporary).unwrap();

    let reopened = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    let recovered = reopened.entries();
    assert_eq!(recovered[0].status, OperationStatus::Applied);
    assert_eq!(recovered[0].steps[0].status, StepStatus::Applied);
    let restored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&progress_path).unwrap()).unwrap();
    assert_eq!(restored["revision"].as_u64(), Some(3));
}

#[test]
fn invalid_active_progress_artifact_requires_repair_without_deletion() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let journal = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    let id = journal
        .plan_operation(rename_plan(
            &temp.path().join("Mods/Old"),
            &temp.path().join("Mods/New"),
        ))
        .unwrap();
    let progress_path = active_state_path(&path, &id);
    journal.mark_applying(&id).unwrap();
    drop(journal);

    std::fs::remove_file(&progress_path).unwrap();
    let recovery = journal_artifact_path(&progress_path, "recover", 0);
    std::fs::write(&recovery, b"not-json").unwrap();

    let error = OperationJournal::open(&path, TEST_HISTORY_LIMIT)
        .err()
        .expect("invalid active progress must require repair");

    assert!(error.to_string().contains("requires repair"));
    assert!(!progress_path.exists());
    assert!(recovery.exists());
}

#[test]
fn ambiguous_unversioned_journal_artifacts_require_repair_without_deletion() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let journal = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    let id = journal
        .plan_operation(rename_plan(
            &temp.path().join("Mods/Old"),
            &temp.path().join("Mods/New"),
        ))
        .unwrap();
    let legacy_bytes = serde_json::to_vec(&journal.entries()).unwrap();
    drop(journal);

    std::fs::write(&path, &legacy_bytes).unwrap();
    let recovery = journal_artifact_path(&path, "recover", 0);
    std::fs::write(&recovery, legacy_bytes).unwrap();

    let error = OperationJournal::open(&path, TEST_HISTORY_LIMIT)
        .err()
        .expect("ambiguous unversioned artifacts must require repair");

    assert!(error.to_string().contains("requires repair"));
    assert!(path.exists());
    assert!(recovery.exists());
    assert!(!id.is_empty());
}

#[test]
fn unversioned_journal_migrates_to_a_versioned_snapshot() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let journal = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    let id = journal
        .plan_operation(rename_plan(
            &temp.path().join("Mods/Old"),
            &temp.path().join("Mods/New"),
        ))
        .unwrap();
    let legacy_bytes = serde_json::to_vec(&journal.entries()).unwrap();
    drop(journal);
    std::fs::write(&path, legacy_bytes).unwrap();

    let reopened = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    let snapshot: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();

    assert_eq!(reopened.entries()[0].id, id);
    assert_eq!(snapshot["format_version"].as_u64(), Some(1));
    assert_eq!(snapshot["revision"].as_u64(), Some(1));
    assert_eq!(snapshot["operation_ids"][0].as_str(), Some(id.as_str()));
    assert!(snapshot["checksum"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
}

#[test]
fn versioned_journal_rejects_checksum_tampering() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let journal = OperationJournal::open(&path, TEST_HISTORY_LIMIT).unwrap();
    journal
        .plan_operation(rename_plan(
            &temp.path().join("Mods/Old"),
            &temp.path().join("Mods/New"),
        ))
        .unwrap();
    drop(journal);

    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    snapshot["checksum"] = serde_json::Value::String("invalid".to_string());
    std::fs::write(&path, serde_json::to_vec(&snapshot).unwrap()).unwrap();

    let error = OperationJournal::open(&path, TEST_HISTORY_LIMIT)
        .err()
        .expect("tampered checksum must not open");

    assert!(error.to_string().contains("checksum"));
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
async fn crash_after_partial_batch_before_settlement_uses_disk_evidence() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let old_a = game_root.join("OldA");
    let new_a = game_root.join("NewA");
    let old_b = game_root.join("OldB");
    let new_b = game_root.join("NewB");
    std::fs::create_dir_all(&old_a).unwrap();
    std::fs::create_dir_all(&old_b).unwrap();
    let identity_a = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&old_a).unwrap();
    let identity_b = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&old_b).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let guard = coordinator
        .acquire_operation(OperationPlan::new(
            "bulk-rename",
            GAME_ID,
            vec![
                PlannedStep::rename(0, old_a.clone(), new_a.clone())
                    .with_expected_identity(Some(identity_a)),
                PlannedStep::rename(1, old_b.clone(), new_b.clone())
                    .with_expected_identity(Some(identity_b)),
            ],
        ))
        .await
        .unwrap();

    std::fs::rename(&old_a, &new_a).unwrap();
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
async fn crash_after_batch_renames_before_settlement_uses_disk_evidence() {
    let temp = tempdir().unwrap();
    let game_root = temp.path().join("Mods");
    let staging_root = temp.path().join("staging");
    let old_a = game_root.join("OldA");
    let new_a = game_root.join("NewA");
    let old_b = game_root.join("OldB");
    let new_b = game_root.join("NewB");
    std::fs::create_dir_all(&old_a).unwrap();
    std::fs::create_dir_all(&old_b).unwrap();
    let identity_a = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&old_a).unwrap();
    let identity_b = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&old_b).unwrap();
    let journal = open_journal(&temp.path().join("journal.json"));
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let guard = coordinator
        .acquire_operation(OperationPlan::new(
            "bulk-rename",
            GAME_ID,
            vec![
                PlannedStep::rename(0, old_a.clone(), new_a.clone())
                    .with_expected_identity(Some(identity_a)),
                PlannedStep::rename(1, old_b.clone(), new_b.clone())
                    .with_expected_identity(Some(identity_b)),
            ],
        ))
        .await
        .unwrap();

    std::fs::rename(&old_a, &new_a).unwrap();
    std::fs::rename(&old_b, &new_b).unwrap();
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

#[derive(Clone, Copy)]
enum JournalBenchmarkScenario {
    SuccessfulCommit,
    PartialRollback,
}

impl JournalBenchmarkScenario {
    fn label(self) -> &'static str {
        match self {
            Self::SuccessfulCommit => "successful_commit",
            Self::PartialRollback => "partial_rollback",
        }
    }

    fn expected_writes(self) -> u64 {
        match self {
            Self::SuccessfulCommit | Self::PartialRollback => 5,
        }
    }
}

struct JournalBenchmarkSample {
    elapsed: Duration,
    writes: u64,
    serialized_bytes: u64,
    final_file_bytes: u64,
}

#[test]
#[ignore = "manual journal I/O baseline; performs synchronous temporary-file writes"]
fn benchmark_bulk_journal_persistence_baseline() {
    for history_limit in [1, 64, 256] {
        for step_count in [100, 1_000, 10_000] {
            for scenario in [
                JournalBenchmarkScenario::SuccessfulCommit,
                JournalBenchmarkScenario::PartialRollback,
            ] {
                let _warmup = run_journal_benchmark_sample(history_limit, step_count, scenario);
                let mut samples = (0..BENCHMARK_SAMPLES)
                    .map(|_| run_journal_benchmark_sample(history_limit, step_count, scenario))
                    .collect::<Vec<_>>();
                samples.sort_by_key(|sample| sample.elapsed);

                let median = samples[BENCHMARK_SAMPLES / 2].elapsed;
                let p95 =
                    samples[((BENCHMARK_SAMPLES * 95).div_ceil(100)).saturating_sub(1)].elapsed;
                let median_bytes = samples[BENCHMARK_SAMPLES / 2].serialized_bytes;
                let final_file_bytes = samples[BENCHMARK_SAMPLES / 2].final_file_bytes;
                let expected_writes = scenario.expected_writes();

                assert!(samples
                    .iter()
                    .all(|sample| sample.writes == expected_writes));
                assert!(samples
                    .iter()
                    .all(|sample| sample.serialized_bytes > sample.final_file_bytes));
                println!(
                    "journal_benchmark scenario={} history_limit={} steps={} samples={} p50_ms={:.3} p95_ms={:.3} writes={} median_serialized_bytes={} median_final_file_bytes={}",
                    scenario.label(),
                    history_limit,
                    step_count,
                    BENCHMARK_SAMPLES,
                    duration_ms(median),
                    duration_ms(p95),
                    expected_writes,
                    median_bytes,
                    final_file_bytes,
                );
            }
        }
    }
}

fn run_journal_benchmark_sample(
    history_limit: usize,
    step_count: usize,
    scenario: JournalBenchmarkScenario,
) -> JournalBenchmarkSample {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let journal = OperationJournal::open(&path, history_limit).unwrap();
    journal
        .seed_terminal_history_for_benchmark(history_limit)
        .unwrap();
    journal.reset_persistence_metrics();

    let started = Instant::now();
    let id = journal
        .plan_operation(benchmark_plan(temp.path(), step_count))
        .unwrap();
    journal.mark_applying(&id).unwrap();

    match scenario {
        JournalBenchmarkScenario::SuccessfulCommit => {
            let settlements = (0..step_count as u32)
                .map(|sequence| (sequence, StepSettlement::Applied))
                .collect::<Vec<_>>();
            journal.settle_steps(&id, &settlements).unwrap();
            journal.mark_db_committed(&id).unwrap();
            journal.complete(&id).unwrap();
        }
        JournalBenchmarkScenario::PartialRollback => {
            let applied_steps = step_count / 2;
            let applied = (0..applied_steps as u32)
                .map(|sequence| (sequence, StepSettlement::Applied))
                .collect::<Vec<_>>();
            journal.settle_steps(&id, &applied).unwrap();
            journal.begin_rollback(&id).unwrap();
            let rolled_back = (0..applied_steps as u32)
                .rev()
                .map(|sequence| (sequence, StepSettlement::RolledBack))
                .collect::<Vec<_>>();
            journal.settle_steps(&id, &rolled_back).unwrap();
        }
    }

    let elapsed = started.elapsed();
    let metrics = journal.persistence_metrics();
    let final_file_bytes = std::fs::metadata(&path).unwrap().len();
    JournalBenchmarkSample {
        elapsed,
        writes: metrics.writes,
        serialized_bytes: metrics.serialized_bytes,
        final_file_bytes,
    }
}

fn benchmark_plan(root: &Path, step_count: usize) -> OperationPlan {
    let steps = (0..step_count)
        .map(|index| {
            PlannedStep::rename(
                index as u32,
                root.join(format!(
                    "Mods/Enabled/Folder-{index:04}/file-{index:04}.ini"
                )),
                root.join(format!(
                    "Mods/Disabled/Folder-{index:04}/file-{index:04}.ini"
                )),
            )
        })
        .collect();
    OperationPlan::new("benchmark-bulk-mutation", GAME_ID, steps)
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
