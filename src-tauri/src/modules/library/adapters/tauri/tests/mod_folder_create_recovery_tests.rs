use super::*;
use crate::modules::mutation::api::recovery_testing::{
    OperationJournal, OperationStatus, RecoveryRoots, RecoveryRunner,
};
use std::collections::HashMap;
use std::fs;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

const GAME_ID: &str = "folder-create-crash-test";
const ROOT_ENV: &str = "EMMM_FOLDER_CREATE_TEST_ROOT";
const BOUNDARY_ENV: &str = "EMMM_FOLDER_CREATE_TEST_BOUNDARY";
const EXIT_CODE: i32 = 86;
const HISTORY_LIMIT: usize = 16;

#[test]
#[ignore = "subprocess fixture; invoked by folder_create_process_exit_recovers_each_boundary"]
fn folder_create_exit_worker() {
    let fixture = PathBuf::from(std::env::var_os(ROOT_ENV).expect("owned fixture root"));
    let boundary = std::env::var(BOUNDARY_ENV).expect("crash boundary");
    let exit_at = |point: &str| {
        if boundary == point {
            std::process::exit(EXIT_CODE);
        }
    };
    let root = fixture.join("Mods").canonicalize().unwrap();
    let mut prepared = prepare_folder_create(&root.join("Object"), "Variants").unwrap();
    prepared.namespace_proof = Some(
        crate::platform::fs::file_utils::FilesystemNamespaceProof::capture(
            &root,
            &[prepared.stage.clone(), prepared.target.clone()],
        )
        .unwrap(),
    );
    prepared.prepare().unwrap();
    let journal = OperationJournal::open(fixture.join("journal.json"), HISTORY_LIMIT).unwrap();
    let id = journal
        .plan_operation(
            crate::modules::mutation::api::OperationPlan::new(
                FOLDER_CREATE_OPERATION_KIND,
                GAME_ID,
                vec![prepared.journal_step().unwrap()],
            )
            .with_source_epoch(
                crate::platform::fs::file_utils::filesystem_identity(&root).unwrap(),
            ),
        )
        .unwrap();
    exit_at("planned");
    journal.mark_applying(&id).unwrap();
    exit_at("applying");
    prepared.promote().unwrap();
    exit_at("promoted");
    journal.mark_step_applied(&id, 0).unwrap();
    exit_at("settled");
    journal.mark_db_committed(&id).unwrap();
    exit_at("db-committed");
    journal.complete(&id).unwrap();
    exit_at("completed");
    panic!("Unknown boundary: {boundary}");
}

fn crash_folder_create(root: &Path, boundary: &str) -> Arc<OperationJournal> {
    fs::create_dir_all(root.join("Mods/Object")).unwrap();
    fs::create_dir(root.join("staging")).unwrap();
    fs::write(
        root.join("Mods/Object/existing.txt"),
        b"preserve existing asset",
    )
    .unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "modules::library::adapters::tauri::mod_core_cmds::folder_create_recovery_tests::folder_create_exit_worker",
            "--ignored",
            "--nocapture",
        ])
        .env(ROOT_ENV, root)
        .env(BOUNDARY_ENV, boundary)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!(
                "Creation child timed out at {boundary}: {:?}",
                child.wait_with_output()
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(EXIT_CODE),
        "{boundary}: {output:?}"
    );
    Arc::new(OperationJournal::open(root.join("journal.json"), HISTORY_LIMIT).unwrap())
}

fn recovery(root: &Path, journal: Arc<OperationJournal>) -> RecoveryRunner {
    RecoveryRunner::new(
        journal,
        RecoveryRoots::new(
            HashMap::from([(GAME_ID.to_string(), root.join("Mods"))]),
            root.join("staging"),
        ),
    )
}

#[tokio::test]
async fn folder_create_process_exit_recovers_each_boundary() {
    for boundary in [
        "planned",
        "applying",
        "promoted",
        "settled",
        "db-committed",
        "completed",
    ] {
        let fixture = tempfile::tempdir().unwrap();
        let journal = crash_folder_create(fixture.path(), boundary);
        let runner = recovery(fixture.path(), Arc::clone(&journal));
        runner.run_recovery().await.unwrap();
        let committed = matches!(boundary, "db-committed" | "completed");
        let mut entries = fs::read_dir(fixture.path().join("Mods/Object"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        entries.sort();
        assert_eq!(
            entries,
            if committed {
                vec!["Variants", "existing.txt"]
            } else {
                vec!["existing.txt"]
            },
            "No orphan staging directory should remain after recovery at {boundary}"
        );
        assert_eq!(
            fs::read(fixture.path().join("Mods/Object/existing.txt")).unwrap(),
            b"preserve existing asset"
        );
        let operations = journal.entries();
        assert_eq!(operations.len(), 1);
        assert_eq!(
            operations[0].status,
            if committed {
                OperationStatus::Completed
            } else {
                OperationStatus::RolledBack
            }
        );
        runner.run_recovery().await.unwrap();
        assert_eq!(journal.entries(), operations);
    }
}

#[tokio::test]
async fn folder_create_recovery_preserves_a_replacement_target() {
    let fixture = tempfile::tempdir().unwrap();
    let journal = crash_folder_create(fixture.path(), "promoted");
    let target = fixture.path().join("Mods/Object/Variants");
    fs::rename(&target, fixture.path().join("Mods/Object/OwnedElsewhere")).unwrap();
    fs::create_dir(&target).unwrap();
    fs::write(target.join("foreign.txt"), b"foreign asset").unwrap();
    recovery(fixture.path(), Arc::clone(&journal))
        .run_recovery()
        .await
        .unwrap();
    assert_eq!(
        fs::read(target.join("foreign.txt")).unwrap(),
        b"foreign asset"
    );
    assert!(fixture.path().join("Mods/Object/OwnedElsewhere").is_dir());
    assert_eq!(
        journal.entries()[0].status,
        OperationStatus::FailedNeedsRepair
    );
}

#[tokio::test]
async fn folder_create_recovery_preserves_added_assets_at_the_owned_target() {
    let fixture = tempfile::tempdir().unwrap();
    let journal = crash_folder_create(fixture.path(), "promoted");
    let target = fixture.path().join("Mods/Object/Variants");
    fs::write(target.join("added.txt"), b"added after crash").unwrap();
    recovery(fixture.path(), Arc::clone(&journal))
        .run_recovery()
        .await
        .unwrap();
    assert_eq!(
        fs::read(target.join("added.txt")).unwrap(),
        b"added after crash"
    );
    assert_eq!(
        journal.entries()[0].status,
        OperationStatus::FailedNeedsRepair
    );
}

#[tokio::test]
async fn folder_create_recovery_settles_interrupted_empty_stage_cleanup() {
    let fixture = tempfile::tempdir().unwrap();
    let journal = crash_folder_create(fixture.path(), "planned");
    let operation = journal.entries().remove(0);
    journal.begin_rollback(&operation.id).unwrap();
    fs::remove_dir(operation.steps[0].old_path.as_deref().unwrap()).unwrap();
    let runner = recovery(fixture.path(), Arc::clone(&journal));
    runner.run_recovery().await.unwrap();
    assert_eq!(journal.entries()[0].status, OperationStatus::RolledBack);
    runner.run_recovery().await.unwrap();
    assert_eq!(
        fs::read(fixture.path().join("Mods/Object/existing.txt")).unwrap(),
        b"preserve existing asset"
    );
}

#[tokio::test]
async fn folder_create_recovery_rejects_non_staging_source_names() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("Mods");
    let source = root.join("UserFolder");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir(fixture.path().join("staging")).unwrap();
    let journal = Arc::new(
        OperationJournal::open(fixture.path().join("journal.json"), HISTORY_LIMIT).unwrap(),
    );
    journal
        .plan_operation(
            crate::modules::mutation::api::OperationPlan::new(
                FOLDER_CREATE_OPERATION_KIND,
                GAME_ID,
                vec![crate::modules::mutation::api::PlannedStep::rename(
                    0,
                    source.clone(),
                    root.join("Target"),
                )
                .with_expected_identity(
                    crate::platform::fs::file_utils::filesystem_identity(&source),
                )],
            )
            .with_source_epoch(
                crate::platform::fs::file_utils::filesystem_identity(&root).unwrap(),
            ),
        )
        .unwrap();
    recovery(fixture.path(), Arc::clone(&journal))
        .run_recovery()
        .await
        .unwrap();
    assert!(source.is_dir());
    assert_eq!(
        journal.entries()[0].status,
        OperationStatus::FailedNeedsRepair
    );
}
