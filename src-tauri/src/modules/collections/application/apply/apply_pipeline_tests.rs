use super::*;

#[cfg(windows)]
fn reattach_original_group_to_replacement_root(root: &std::path::Path) {
    let parked = root.with_file_name("ParkedMods");
    std::fs::rename(root, &parked).unwrap();
    std::fs::create_dir(root).unwrap();
    std::fs::rename(parked.join("Group"), root.join("Group")).unwrap();
}

#[cfg(windows)]
#[tokio::test]
async fn collection_runtime_retry_rejects_reattached_original_group() {
    let mut fixture = nested_collection_rename_fixture().await;
    let root = fixture.context.mods_path.clone();
    fixture.context.to_disable = vec![crate::shared::path_key::folder_path_key(
        "Group/Object/Mod",
        Some(&root.to_string_lossy()),
    )];
    super::super::steps::batch_rename::prepare(&mut fixture.context)
        .await
        .unwrap();
    let plan = fixture
        .context
        .prepared_renames
        .as_ref()
        .unwrap()
        .first_mod_plan()
        .unwrap();
    let source = root.join("Group/Object/Mod");
    plan.apply()
        .expect("original namespace must allow runtime rename");
    plan.rollback()
        .expect("original namespace must allow runtime rollback");
    assert!(source.join("mesh.buf").is_file());
    let original_root = crate::platform::fs::file_utils::filesystem_identity(&root).unwrap();
    let mut attempts = 0;
    let result = crate::platform::fs::retry::RetryBudget::default()
        .run(|| {
            attempts += 1;
            if attempts == 1 {
                reattach_original_group_to_replacement_root(&root);
                std::future::ready(Err(std::io::Error::from_raw_os_error(32)))
            } else {
                std::future::ready(plan.apply())
            }
        })
        .await;

    assert_eq!(attempts, 2);
    assert!(result.is_err(), "retry must reject a replacement Mods root");
    assert_ne!(
        crate::platform::fs::file_utils::filesystem_identity(&root).as_deref(),
        Some(original_root.as_str())
    );
    assert_eq!(
        std::fs::read(source.join("mesh.buf")).unwrap(),
        b"owned asset"
    );
    assert!(!root.join("Group/Object/DISABLED Mod").exists());
}

#[cfg(windows)]
#[tokio::test]
async fn collection_object_retry_rejects_reattached_original_group() {
    use crate::modules::collections::domain::collection::MemberKind;
    let mut fixture = nested_collection_rename_fixture().await;
    let root = fixture.context.mods_path.clone();
    let source = root.join("Group/Object");
    fixture.context.target_objects.push(CollectionObject {
        kind: MemberKind::Object,
        collection_id: "epoch-collection".into(),
        object_id: "epoch-object".into(),
        is_enabled: false,
        display_name: Some("Object".into()),
        path_key: None,
    });
    super::super::steps::batch_rename::prepare(&mut fixture.context)
        .await
        .unwrap();
    let plan = fixture
        .context
        .prepared_renames
        .as_ref()
        .unwrap()
        .first_object_plan()
        .unwrap();
    plan.apply("object folder")
        .expect("original namespace must allow object rename");
    plan.rollback("object folder")
        .expect("original namespace must allow object rollback");
    assert!(source.join("Mod/mesh.buf").is_file());
    let original_root = crate::platform::fs::file_utils::filesystem_identity(&root).unwrap();
    let mut attempts = 0;
    let result = crate::platform::fs::retry::RetryBudget::default()
        .run(|| {
            attempts += 1;
            if attempts == 1 {
                reattach_original_group_to_replacement_root(&root);
                std::future::ready(Err(std::io::Error::from_raw_os_error(32)))
            } else {
                std::future::ready(
                    plan.apply("object folder")
                        .map_err(|error| std::io::Error::other(error.to_string())),
                )
            }
        })
        .await;

    assert_eq!(attempts, 2);
    assert!(result.is_err(), "retry must reject a replacement Mods root");
    assert_ne!(
        crate::platform::fs::file_utils::filesystem_identity(&root).as_deref(),
        Some(original_root.as_str())
    );
    assert_eq!(
        std::fs::read(source.join("Mod/mesh.buf")).unwrap(),
        b"owned asset"
    );
    assert!(!root.join("Group/DISABLED Object").exists());
}

#[cfg(windows)]
#[tokio::test]
async fn collection_disables_nested_mod_then_object_without_self_blocking() {
    use crate::modules::collections::domain::collection::MemberKind;
    let mut fixture = nested_collection_rename_fixture().await;
    let root = fixture.context.mods_path.clone();
    fixture.context.to_disable = vec![crate::shared::path_key::folder_path_key(
        "Group/Object/Mod",
        Some(&root.to_string_lossy()),
    )];
    fixture.context.target_objects.push(CollectionObject {
        kind: MemberKind::Object,
        collection_id: "epoch-collection".into(),
        object_id: "epoch-object".into(),
        is_enabled: false,
        display_name: Some("Object".into()),
        path_key: None,
    });
    super::super::steps::batch_rename::prepare(&mut fixture.context)
        .await
        .unwrap();
    super::super::steps::batch_rename::rename(&mut fixture.context)
        .await
        .unwrap();

    assert!(root
        .join("Group/DISABLED Object/DISABLED Mod/mesh.buf")
        .is_file());
    assert!(!root.join("Group/Object").exists());
}

#[cfg(windows)]
#[tokio::test]
async fn collection_trusted_reconcile_keeps_nested_group_and_leaf_owned() {
    use crate::modules::collections::domain::collection::MemberKind;
    let mut fixture = nested_collection_rename_fixture().await;
    let root = fixture.context.mods_path.clone();
    fixture.context.to_disable = vec![crate::shared::path_key::folder_path_key(
        "Group/Object/Mod",
        Some(&root.to_string_lossy()),
    )];
    fixture.context.target_objects.push(CollectionObject {
        kind: MemberKind::Object,
        collection_id: "epoch-collection".into(),
        object_id: "epoch-object".into(),
        is_enabled: false,
        display_name: Some("Object".into()),
        path_key: None,
    });
    super::super::steps::batch_rename::prepare(&mut fixture.context)
        .await
        .unwrap();
    let group = root.join("Group");
    let parked_group = root.join("ParkedGroup");
    let leaf = group.join("DISABLED Object/DISABLED Mod");
    let parked_leaf = group.join("DISABLED Object/ParkedMod");
    let mut group_moved = false;
    let mut leaf_moved = false;
    super::super::steps::batch_rename::rename_with_test_hook(&mut fixture.context, || {
        group_moved = std::fs::rename(&group, &parked_group).is_ok();
        if group_moved {
            std::fs::rename(&parked_group, &group).unwrap();
        }
        leaf_moved = std::fs::rename(&leaf, &parked_leaf).is_ok();
        if leaf_moved {
            std::fs::rename(&parked_leaf, &leaf).unwrap();
        }
    })
    .await
    .unwrap();

    assert!(!group_moved, "trusted Group was movable at reconcile entry");
    assert!(
        !leaf_moved,
        "trusted destination was movable at reconcile entry"
    );
    assert_eq!(
        std::fs::read(leaf.join("mesh.buf")).unwrap(),
        b"owned asset"
    );
}

#[cfg(windows)]
#[tokio::test]
async fn collection_stale_spelling_noop_leaf_stays_owned_during_reconcile() {
    let mut fixture = nested_collection_rename_fixture().await;
    let root = fixture.context.mods_path.clone();
    let stale_path = "Group/Object/DISABLED Mod";
    let stale_key =
        crate::shared::path_key::folder_path_key(stale_path, Some(&root.to_string_lossy()));
    sqlx::query("UPDATE mods SET folder_path = ?, folder_path_key = ? WHERE id = 'epoch-mod'")
        .bind(stale_path)
        .bind(stale_key)
        .execute(&fixture.context.pool)
        .await
        .unwrap();
    fixture.context.to_enable = vec![crate::shared::path_key::folder_path_key(
        "Group/Object/Mod",
        Some(&root.to_string_lossy()),
    )];
    super::super::steps::batch_rename::prepare(&mut fixture.context)
        .await
        .unwrap();
    let plan = fixture
        .context
        .prepared_renames
        .as_ref()
        .unwrap()
        .first_mod_plan()
        .unwrap();
    assert_eq!(plan.old_path(), plan.new_path());
    assert_ne!(plan.requested_path(), plan.old_path());
    let leaf = root.join("Group/Object/Mod");
    let parked = root.join("Group/Object/ParkedMod");
    let mut leaf_moved = false;
    super::super::steps::batch_rename::rename_with_test_hook(&mut fixture.context, || {
        leaf_moved = std::fs::rename(&leaf, &parked).is_ok();
        if leaf_moved {
            std::fs::rename(&parked, &leaf).unwrap();
        }
    })
    .await
    .unwrap();

    assert!(
        !leaf_moved,
        "checked no-op leaf was movable at trusted reconcile entry"
    );
    assert_eq!(
        std::fs::read(leaf.join("mesh.buf")).unwrap(),
        b"owned asset"
    );
}

struct CollectionJournalFixture {
    _directory: tempfile::TempDir,
    context: ApplyContext,
    journal: std::sync::Arc<crate::modules::mutation::api::recovery_testing::OperationJournal>,
    coordinator: crate::modules::mutation::api::MutationCoordinator,
}

async fn collection_journal_fixture() -> CollectionJournalFixture {
    collection_journal_fixture_in(None).await
}

async fn collection_journal_fixture_in(
    parent: Option<&std::path::Path>,
) -> CollectionJournalFixture {
    use crate::modules::games::domain::models::{GameType, ItemStatus};
    use crate::test_utils::{
        insert_test_game, insert_test_mod, insert_test_object, TestGameFixture, TestModFixture,
        TestObjectFixture,
    };
    let directory = match parent {
        Some(parent) => tempfile::tempdir_in(parent).unwrap(),
        None => tempfile::tempdir().unwrap(),
    };
    let mods_path = directory.path().join("Mods");
    let mod_path = mods_path.join("Object/Mod");
    std::fs::create_dir_all(&mod_path).unwrap();
    std::fs::write(
        mod_path.join("mod.ini"),
        "[TextureOverrideTest]\nhash = 01234567\n",
    )
    .unwrap();
    std::fs::write(mod_path.join("mesh.buf"), b"owned asset").unwrap();
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(directory.path().join("test.sqlite"))
                .create_if_missing(true),
        )
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    crate::modules::system::adapters::sqlite::utils::unicode_keys::ensure_unicode_keys(&pool)
        .await
        .unwrap();
    let root_text = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "epoch-game",
            name: "Epoch",
            game_type: GameType::Gimi,
            path: &root_text,
            mods_path: Some(&root_text),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "epoch-object",
            game_id: "epoch-game",
            name: "Object",
            folder_path: "Object",
            object_type: "Other",
        },
    )
    .await
    .unwrap();
    insert_test_mod(
        &pool,
        &TestModFixture {
            id: "epoch-mod",
            game_id: "epoch-game",
            object_id: Some("epoch-object"),
            actual_name: "Mod",
            folder_path: "Object/Mod",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Other"),
            mods_path: Some(&root_text),
        },
    )
    .await
    .unwrap();
    crate::modules::collections::adapters::sqlite::create(
        &pool,
        "epoch-collection",
        "epoch-game",
        "Empty target",
        true,
        false,
    )
    .await
    .unwrap();
    let journal = std::sync::Arc::new(
        crate::modules::mutation::api::recovery_testing::OperationJournal::open(
            directory.path().join("journal.json"),
            16,
        )
        .unwrap(),
    );
    let coordinator = crate::modules::mutation::api::MutationCoordinator::unconfigured();
    coordinator.configure(journal.clone()).unwrap();
    let context = ApplyContext::new(ApplyCollectionRequest {
        pool: &pool,
        game_id: "epoch-game",
        collection_id: "epoch-collection",
        capture_last_changes: false,
        mods_path,
        suppressor: std::sync::Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    });
    CollectionJournalFixture {
        _directory: directory,
        context,
        journal,
        coordinator,
    }
}

#[cfg(windows)]
async fn nested_collection_rename_fixture() -> CollectionJournalFixture {
    let mut fixture = collection_journal_fixture().await;
    let root = &fixture.context.mods_path;
    std::fs::create_dir(root.join("Group")).unwrap();
    std::fs::rename(root.join("Object"), root.join("Group/Object")).unwrap();
    sqlx::query("UPDATE objects SET folder_path = 'Group/Object' WHERE id = 'epoch-object'")
        .execute(&fixture.context.pool)
        .await
        .unwrap();
    let path_key =
        crate::shared::path_key::folder_path_key("Group/Object/Mod", Some(&root.to_string_lossy()));
    sqlx::query("UPDATE mods SET folder_path = 'Group/Object/Mod', folder_path_key = ? WHERE id = 'epoch-mod'")
        .bind(path_key)
        .execute(&fixture.context.pool)
        .await
        .unwrap();
    fixture.context.source_root_proof =
        Some(crate::platform::fs::file_utils::FilesystemIdentityProof::capture(root).unwrap());
    fixture
}

#[tokio::test]
async fn collection_apply_journal_retains_original_root_identity() {
    let mut fixture = collection_journal_fixture().await;
    let expected =
        crate::platform::fs::file_utils::filesystem_identity(&fixture.context.mods_path).unwrap();
    let _lock = fixture.coordinator.inner_lock().acquire().await.unwrap();

    execute_inner(&mut fixture.context, Some(&fixture.coordinator))
        .await
        .unwrap();

    let operations = fixture.journal.entries();
    assert_eq!(operations.len(), 1);
    assert_eq!(
        operations[0].source_epoch.as_deref(),
        Some(expected.as_str())
    );
    assert!(fixture
        .context
        .mods_path
        .join("Object/DISABLED Mod/mesh.buf")
        .is_file());
}

#[cfg(windows)]
#[tokio::test]
async fn collection_replaced_root_cannot_finalize_completed_task() {
    let mut fixture = collection_journal_fixture().await;
    crate::modules::workspace::adapters::sqlite::task::create_claimed_task(
        &fixture.context.pool,
        "task-replaced-root",
        "epoch-game",
        crate::modules::workspace::domain::task::TASK_TYPE_APPLY_COLLECTION,
        Some("epoch-collection"),
    )
    .await
    .unwrap();
    let _lock = fixture.coordinator.inner_lock().acquire().await.unwrap();
    execute_inner(&mut fixture.context, Some(&fixture.coordinator))
        .await
        .unwrap();
    let parked = fixture._directory.path().join("ParkedMods");
    std::fs::rename(&fixture.context.mods_path, &parked).unwrap();
    std::fs::create_dir(&fixture.context.mods_path).unwrap();

    finalize_apply(&fixture.context, "task-replaced-root", TaskStatus::Running)
        .await
        .expect_err("replacement root must not be finalized");
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(
        &fixture.context.pool,
        "task-replaced-root",
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(task.status, TaskStatus::Running);
    assert!(parked.join("Object/DISABLED Mod/mesh.buf").is_file());
}

#[tokio::test]
async fn completed_collection_task_does_not_rollback_disk_after_journal_gap() {
    use crate::modules::mutation::api::recovery_testing::{
        OperationStatus, RecoveryRoots, RecoveryRunner,
    };
    let mut fixture = collection_journal_fixture().await;
    crate::modules::workspace::adapters::sqlite::task::create_claimed_task(
        &fixture.context.pool,
        "task-journal-gap",
        "epoch-game",
        crate::modules::workspace::domain::task::TASK_TYPE_APPLY_COLLECTION,
        Some("epoch-collection"),
    )
    .await
    .unwrap();
    let _lock = fixture.coordinator.inner_lock().acquire().await.unwrap();
    execute_inner(&mut fixture.context, Some(&fixture.coordinator))
        .await
        .unwrap();
    finalize_apply(&fixture.context, "task-journal-gap", TaskStatus::Running)
        .await
        .unwrap();
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(
        &fixture.context.pool,
        "task-journal-gap",
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(task.status, TaskStatus::Completed);

    let runner = RecoveryRunner::new(
        fixture.journal.clone(),
        RecoveryRoots::new(
            std::collections::HashMap::from([(
                "epoch-game".into(),
                fixture.context.mods_path.clone(),
            )]),
            fixture._directory.path().join("Staging"),
        ),
    )
    .with_projection_pool(fixture.context.pool.clone());
    runner.run_recovery().await.unwrap();

    assert!(fixture
        .context
        .mods_path
        .join("Object/DISABLED Mod/mesh.buf")
        .is_file());
    assert_eq!(
        fixture.journal.entries()[0].status,
        OperationStatus::Completed
    );
    assert!(
        crate::modules::mutation::adapters::sqlite::projection_receipts::get(
            &fixture.context.pool,
            &fixture.journal.entries()[0].id,
        )
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
#[ignore = "subprocess fixture for collection_receipt_survives_abrupt_exit_and_reopen"]
async fn collection_receipt_exit_worker() {
    let Some(parent) = std::env::var_os("EMMM_COLLECTION_RECEIPT_TEST_ROOT") else {
        return;
    };
    let parent = PathBuf::from(parent);
    let mut fixture = collection_journal_fixture_in(Some(&parent)).await;
    crate::modules::workspace::adapters::sqlite::task::create_claimed_task(
        &fixture.context.pool,
        "task-abrupt-exit",
        "epoch-game",
        crate::modules::workspace::domain::task::TASK_TYPE_APPLY_COLLECTION,
        Some("epoch-collection"),
    )
    .await
    .unwrap();
    let _lock = fixture.coordinator.inner_lock().acquire().await.unwrap();
    execute_inner(&mut fixture.context, Some(&fixture.coordinator))
        .await
        .unwrap();
    finalize_apply(&fixture.context, "task-abrupt-exit", TaskStatus::Running)
        .await
        .unwrap();
    assert!(fixture
        .context
        .mods_path
        .join("Object/DISABLED Mod/mesh.buf")
        .is_file());
    assert_ne!(
        fixture.journal.entries()[0].status,
        crate::modules::mutation::api::recovery_testing::OperationStatus::Completed
    );
    std::fs::write(
        parent.join("fixture-path.txt"),
        fixture._directory.path().to_string_lossy().as_bytes(),
    )
    .unwrap();
    std::process::exit(87);
}

#[tokio::test]
async fn collection_receipt_survives_abrupt_exit_and_reopen() {
    use crate::modules::mutation::api::recovery_testing::{
        OperationJournal, OperationStatus, RecoveryRoots, RecoveryRunner,
    };
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let parent = tempfile::tempdir().unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "modules::collections::application::apply::apply_pipeline::tests::collection_receipt_exit_worker",
            "--ignored",
            "--nocapture",
        ])
        .env("EMMM_COLLECTION_RECEIPT_TEST_ROOT", parent.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!(
                "collection receipt child timed out: {:?}",
                child.wait_with_output()
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(87), "child output: {output:?}");
    let fixture_path =
        PathBuf::from(std::fs::read_to_string(parent.path().join("fixture-path.txt")).unwrap());
    let mods_path = fixture_path.join("Mods");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new().filename(fixture_path.join("test.sqlite")),
        )
        .await
        .unwrap();
    let journal =
        std::sync::Arc::new(OperationJournal::open(fixture_path.join("journal.json"), 16).unwrap());
    assert_eq!(journal.entries().len(), 1);
    assert_ne!(journal.entries()[0].status, OperationStatus::Completed);
    RecoveryRunner::new(
        journal.clone(),
        RecoveryRoots::new(
            std::collections::HashMap::from([("epoch-game".into(), mods_path.clone())]),
            fixture_path.join("Staging"),
        ),
    )
    .with_projection_pool(pool.clone())
    .run_recovery()
    .await
    .unwrap();

    assert!(mods_path.join("Object/DISABLED Mod/mesh.buf").is_file());
    assert!(!mods_path.join("Object/Mod").exists());
    assert_eq!(journal.entries()[0].status, OperationStatus::Completed);
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(
        &pool,
        "task-abrupt-exit",
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(
        crate::modules::mutation::adapters::sqlite::projection_receipts::get(
            &pool,
            &journal.entries()[0].id,
        )
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn failed_collection_finalization_has_no_receipt_and_recovers_precommit_disk() {
    use crate::modules::mutation::api::recovery_testing::{
        OperationStatus, RecoveryRoots, RecoveryRunner,
    };
    let mut fixture = collection_journal_fixture().await;
    crate::modules::workspace::adapters::sqlite::task::create_claimed_task(
        &fixture.context.pool,
        "task-finalize-fails",
        "epoch-game",
        crate::modules::workspace::domain::task::TASK_TYPE_APPLY_COLLECTION,
        Some("epoch-collection"),
    )
    .await
    .unwrap();
    let _lock = fixture.coordinator.inner_lock().acquire().await.unwrap();
    execute_inner(&mut fixture.context, Some(&fixture.coordinator))
        .await
        .unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_collection_task_completion BEFORE UPDATE OF status ON tasks \
         WHEN NEW.id = 'task-finalize-fails' AND NEW.status = 'COMPLETED' \
         BEGIN SELECT RAISE(FAIL, 'injected finalization failure'); END",
    )
    .execute(&fixture.context.pool)
    .await
    .unwrap();
    assert!(
        finalize_apply(&fixture.context, "task-finalize-fails", TaskStatus::Running)
            .await
            .is_err()
    );
    let operation_id = fixture.journal.entries()[0].id.clone();
    let receipt = crate::modules::mutation::adapters::sqlite::projection_receipts::get(
        &fixture.context.pool,
        &operation_id,
    )
    .await
    .unwrap();
    assert!(receipt.is_none());
    let runner = RecoveryRunner::new(
        fixture.journal.clone(),
        RecoveryRoots::new(
            std::collections::HashMap::from([(
                "epoch-game".into(),
                fixture.context.mods_path.clone(),
            )]),
            fixture._directory.path().join("Staging"),
        ),
    )
    .with_projection_pool(fixture.context.pool.clone());
    runner.run_recovery().await.unwrap();
    assert!(fixture
        .context
        .mods_path
        .join("Object/Mod/mesh.buf")
        .is_file());
    assert_eq!(
        fixture.journal.entries()[0].status,
        OperationStatus::RolledBack
    );
}

#[tokio::test]
async fn mismatched_collection_receipt_preserves_disk_for_repair() {
    use crate::modules::mutation::api::recovery_testing::{
        OperationStatus, RecoveryRoots, RecoveryRunner,
    };
    let mut fixture = collection_journal_fixture().await;
    let _lock = fixture.coordinator.inner_lock().acquire().await.unwrap();
    execute_inner(&mut fixture.context, Some(&fixture.coordinator))
        .await
        .unwrap();
    let operation_id = fixture.journal.entries()[0].id.clone();
    sqlx::query(
        "INSERT INTO mutation_projection_receipts (operation_id, game_id, source_epoch) VALUES (?, ?, ?)",
    )
    .bind(&operation_id)
    .bind("wrong-game")
    .bind("wrong-epoch")
    .execute(&fixture.context.pool)
    .await
    .unwrap();
    let runner = RecoveryRunner::new(
        fixture.journal.clone(),
        RecoveryRoots::new(
            std::collections::HashMap::from([(
                "epoch-game".into(),
                fixture.context.mods_path.clone(),
            )]),
            fixture._directory.path().join("Staging"),
        ),
    )
    .with_projection_pool(fixture.context.pool.clone());
    runner.run_recovery().await.unwrap();
    assert!(fixture
        .context
        .mods_path
        .join("Object/DISABLED Mod/mesh.buf")
        .is_file());
    assert_eq!(
        fixture.journal.entries()[0].status,
        OperationStatus::FailedNeedsRepair
    );
    assert!(
        crate::modules::mutation::adapters::sqlite::projection_receipts::get(
            &fixture.context.pool,
            &operation_id,
        )
        .await
        .unwrap()
        .is_some()
    );
}

#[tokio::test]
async fn legacy_collection_with_root_proof_still_preserves_unknown_commit() {
    use crate::modules::mutation::api::recovery_testing::{
        OperationStatus, RecoveryRoots, RecoveryRunner,
    };
    use crate::modules::mutation::api::{OperationPlan, PlannedStep};
    let fixture = collection_journal_fixture().await;
    let old = fixture.context.mods_path.join("Object/Mod");
    let new = fixture.context.mods_path.join("Object/DISABLED Mod");
    let identity = crate::platform::fs::file_utils::filesystem_identity(&old).unwrap();
    let root_epoch =
        crate::platform::fs::file_utils::filesystem_identity(&fixture.context.mods_path).unwrap();
    let id = fixture
        .journal
        .plan_operation(
            OperationPlan::new(
                "collection-apply",
                "epoch-game",
                vec![PlannedStep::rename(0, old.clone(), new.clone())
                    .with_expected_identity(Some(identity))],
            )
            .with_source_epoch(root_epoch),
        )
        .unwrap();
    fixture.journal.mark_applying(&id).unwrap();
    std::fs::rename(&old, &new).unwrap();
    fixture.journal.mark_step_applied(&id, 0).unwrap();
    let runner = RecoveryRunner::new(
        fixture.journal.clone(),
        RecoveryRoots::new(
            std::collections::HashMap::from([(
                "epoch-game".into(),
                fixture.context.mods_path.clone(),
            )]),
            fixture._directory.path().join("Staging"),
        ),
    )
    .with_projection_pool(fixture.context.pool.clone());
    runner.run_recovery().await.unwrap();
    assert_eq!(
        fixture.journal.entries()[0].status,
        OperationStatus::FailedNeedsRepair
    );
    assert!(!old.exists());
    assert!(new.join("mesh.buf").is_file());
}

#[tokio::test]
async fn legacy_collection_recovery_preserves_disk_without_root_proof() {
    use crate::modules::mutation::api::recovery_testing::{
        OperationStatus, RecoveryRoots, RecoveryRunner,
    };
    use crate::modules::mutation::api::{OperationPlan, PlannedStep};
    let fixture = collection_journal_fixture().await;
    let old = fixture.context.mods_path.join("Object/Mod");
    let new = fixture.context.mods_path.join("Object/DISABLED Mod");
    let identity = crate::platform::fs::file_utils::filesystem_identity(&old).unwrap();
    let id = fixture
        .journal
        .plan_operation(OperationPlan::new(
            "collection-apply",
            "epoch-game",
            vec![PlannedStep::rename(0, old.clone(), new.clone())
                .with_expected_identity(Some(identity))],
        ))
        .unwrap();
    fixture.journal.mark_applying(&id).unwrap();
    std::fs::rename(&old, &new).unwrap();
    fixture.journal.mark_step_applied(&id, 0).unwrap();
    let runner = RecoveryRunner::new(
        fixture.journal.clone(),
        RecoveryRoots::new(
            std::collections::HashMap::from([(
                "epoch-game".into(),
                fixture.context.mods_path.clone(),
            )]),
            fixture._directory.path().join("Staging"),
        ),
    );

    runner.run_recovery().await.unwrap();

    assert_eq!(
        fixture.journal.entries()[0].status,
        OperationStatus::FailedNeedsRepair
    );
    assert!(!old.exists());
    assert_eq!(std::fs::read(new.join("mesh.buf")).unwrap(), b"owned asset");
}

#[tokio::test]
async fn successful_apply_surfaces_task_finalization_failure() {
    let test_db = crate::test_utils::init_test_db().await;
    let pool = test_db.pool.clone();
    pool.close().await;
    let mut apply_context = ApplyContext::new(ApplyCollectionRequest {
        pool: &pool,
        game_id: "game-1",
        collection_id: "collection-1",
        capture_last_changes: false,
        mods_path: PathBuf::from("E:/Mods"),
        suppressor: std::sync::Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    });
    apply_context.final_state_name = Some("Preset".to_string());

    let error = finalize_apply(&apply_context, "task-1", TaskStatus::Pending)
        .await
        .expect_err("task finalization failure must be returned");

    assert!(matches!(error, CollectionError::Db(_)));
}

#[tokio::test]
async fn finalization_failure_rolls_back_active_baseline_and_task_completion_together() {
    let test_db = crate::test_utils::init_test_db().await;
    crate::test_utils::insert_test_game(
        &test_db.pool,
        &crate::test_utils::TestGameFixture {
            id: "game-atomic-finalize",
            name: "Atomic finalize",
            game_type: crate::modules::games::domain::models::GameType::Gimi,
            path: "E:/Games/Atomic",
            mods_path: Some("E:/Mods/Atomic"),
        },
    )
    .await
    .expect("seed game");
    for (id, name) in [("baseline-before", "Before"), ("baseline-after", "After")] {
        crate::modules::collections::adapters::sqlite::create(
            &test_db.pool,
            id,
            "game-atomic-finalize",
            name,
            true,
            false,
        )
        .await
        .expect("seed collection");
    }
    crate::modules::collections::adapters::sqlite::runtime::set_active(
        &test_db.pool,
        "game-atomic-finalize",
        Some("baseline-before"),
    )
    .await
    .expect("seed active baseline");
    crate::modules::workspace::adapters::sqlite::task::create_claimed_task(
        &test_db.pool,
        "task-atomic-finalize",
        "game-atomic-finalize",
        crate::modules::workspace::domain::task::TASK_TYPE_APPLY_COLLECTION,
        Some("baseline-after"),
    )
    .await
    .expect("seed running task");
    sqlx::query(
        "CREATE TRIGGER fail_task_completion BEFORE UPDATE OF status ON tasks \
         WHEN NEW.id = 'task-atomic-finalize' AND NEW.status = 'COMPLETED' \
         BEGIN SELECT RAISE(FAIL, 'injected task completion failure'); END",
    )
    .execute(&test_db.pool)
    .await
    .expect("install failure trigger");

    let mut apply_context = ApplyContext::new(ApplyCollectionRequest {
        pool: &test_db.pool,
        game_id: "game-atomic-finalize",
        collection_id: "baseline-after",
        capture_last_changes: false,
        mods_path: PathBuf::from("E:/Mods/Atomic"),
        suppressor: std::sync::Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    });
    apply_context.finalize_active_collection = true;
    apply_context.final_active_collection_id = Some("baseline-after".to_string());

    let error = finalize_apply(&apply_context, "task-atomic-finalize", TaskStatus::Running)
        .await
        .expect_err("injected finalization failure");
    assert!(matches!(error, CollectionError::Db(_)));

    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(
        &test_db.pool,
        "game-atomic-finalize",
    )
    .await
    .expect("load runtime")
    .expect("runtime exists");
    assert_eq!(
        runtime.active_collection_id.as_deref(),
        Some("baseline-before"),
        "active baseline update must roll back with failed task completion"
    );
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(
        &test_db.pool,
        "task-atomic-finalize",
    )
    .await
    .expect("load task")
    .expect("task exists");
    assert_eq!(task.status, TaskStatus::Running);
}
