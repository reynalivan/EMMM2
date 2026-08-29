use super::*;

#[tokio::test]
async fn apply_collection_disables_a_disabled_parent_object() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-parent-state", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-parent-state").await;
    std::fs::create_dir_all(mods_root.path().join("AINOZ")).expect("create enabled object");

    let collection = collection::create(
        &ctx.pool,
        "collection-parent-disabled",
        "game-parent-state",
        "Parent disabled",
        true,
        false,
    )
    .await
    .expect("create collection");
    let target_object = CollectionObject {
        is_enabled: false,
        ..test_collection_object(&collection.id)
    };
    let projected_state = projected_state::build_projected_state(
        &[],
        std::slice::from_ref(&target_object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        &[],
        &[target_object],
        &projected_state,
    )
    .await
    .expect("persist disabled parent state");

    apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-parent-state",
        collection_id: &collection.id,
        capture_last_changes: false,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    })
    .await
    .expect("apply collection");

    assert!(
        !mods_root.path().join("AINOZ").exists(),
        "the enabled parent folder must be removed"
    );
    assert!(
        mods_root.path().join("DISABLED AINOZ").is_dir(),
        "the collection's disabled parent state must be applied to disk"
    );
    let status: i64 = sqlx::query_scalar("SELECT status FROM objects WHERE id = ?")
        .bind("object-1")
        .fetch_one(&ctx.pool)
        .await
        .expect("load parent status");
    assert_eq!(status, ItemStatus::Disabled as i64);
}

#[tokio::test]
async fn apply_collection_enables_an_enabled_parent_object() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-parent-enable", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-parent-enable").await;
    std::fs::create_dir_all(mods_root.path().join("DISABLED AINOZ"))
        .expect("create disabled object");
    sqlx::query("UPDATE objects SET folder_path = ?, folder_path_key = ?, status = ? WHERE id = ?")
        .bind("DISABLED AINOZ")
        .bind(crate::shared::path_key::folder_path_key(
            "DISABLED AINOZ",
            None,
        ))
        .bind(ItemStatus::Disabled as i64)
        .bind("object-1")
        .execute(&ctx.pool)
        .await
        .expect("seed disabled parent projection");

    let collection = collection::create(
        &ctx.pool,
        "collection-parent-enabled",
        "game-parent-enable",
        "Parent enabled",
        true,
        false,
    )
    .await
    .expect("create collection");
    let target_object = test_collection_object(&collection.id);
    let projected_state = projected_state::build_projected_state(
        &[],
        std::slice::from_ref(&target_object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        &[],
        &[target_object],
        &projected_state,
    )
    .await
    .expect("persist enabled parent state");

    apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-parent-enable",
        collection_id: &collection.id,
        capture_last_changes: false,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    })
    .await
    .expect("apply collection");

    assert!(
        !mods_root.path().join("DISABLED AINOZ").exists(),
        "the disabled parent folder must be removed"
    );
    assert!(
        mods_root.path().join("AINOZ").is_dir(),
        "the collection's enabled parent state must be applied to disk"
    );
    let status: i64 = sqlx::query_scalar("SELECT status FROM objects WHERE id = ?")
        .bind("object-1")
        .fetch_one(&ctx.pool)
        .await
        .expect("load parent status");
    assert_eq!(status, ItemStatus::Enabled as i64);
}

#[tokio::test]
async fn apply_collection_converges_mixed_parent_and_child_state_to_clean() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-mixed-parent-child", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-mixed-parent-child").await;
    create_flat_mod_folder(mods_root.path(), "AINOZ/Red");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-mixed-state",
            game_id: "game-mixed-parent-child",
            object_id: Some("object-1"),
            actual_name: "Red",
            folder_path: "AINOZ/Red",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert target child projection");

    let collection = collection::create(
        &ctx.pool,
        "collection-mixed-target",
        "game-mixed-parent-child",
        "Mixed target",
        true,
        false,
    )
    .await
    .expect("create collection");
    let target_mod = test_collection_mod(&collection.id, "AINOZ/Red", "Red");
    let target_object = test_collection_object(&collection.id);
    let projected_state = projected_state::build_projected_state(
        std::slice::from_ref(&target_mod),
        std::slice::from_ref(&target_object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        &[target_mod],
        &[target_object],
        &projected_state,
    )
    .await
    .expect("persist mixed target state");

    std::fs::rename(
        mods_root.path().join("AINOZ/Red"),
        mods_root.path().join("AINOZ/DISABLED Red"),
    )
    .expect("disable child on disk");
    std::fs::rename(
        mods_root.path().join("AINOZ"),
        mods_root.path().join("DISABLED AINOZ"),
    )
    .expect("disable parent on disk");
    sqlx::query("UPDATE objects SET folder_path = ?, folder_path_key = ?, status = ? WHERE id = ?")
        .bind("DISABLED AINOZ")
        .bind(crate::shared::path_key::folder_path_key(
            "DISABLED AINOZ",
            None,
        ))
        .bind(ItemStatus::Disabled as i64)
        .bind("object-1")
        .execute(&ctx.pool)
        .await
        .expect("seed disabled parent projection");
    sqlx::query("UPDATE mods SET folder_path = ?, folder_path_key = ?, status = ? WHERE id = ?")
        .bind("DISABLED AINOZ/DISABLED Red")
        .bind(crate::shared::path_key::folder_path_key(
            "DISABLED AINOZ/DISABLED Red",
            Some(&mods_path),
        ))
        .bind(ItemStatus::Disabled as i64)
        .bind("mod-mixed-state")
        .execute(&ctx.pool)
        .await
        .expect("seed disabled child projection");

    apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-mixed-parent-child",
        collection_id: &collection.id,
        capture_last_changes: true,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    })
    .await
    .expect("apply mixed target state");

    assert!(mods_root.path().join("AINOZ/Red").is_dir());
    assert!(!mods_root.path().join("DISABLED AINOZ").exists());
    let runtime = crate::modules::collections::application::runtime::get_collection_runtime_state(
        &ctx.pool,
        "game-mixed-parent-child",
    )
    .await
    .expect("load runtime state");
    let target = collection::get_by_id(&ctx.pool, &collection.id)
        .await
        .expect("load target collection")
        .expect("target collection exists");
    assert_eq!(runtime.current_signature, target.signature.unwrap());
    assert_eq!(
        runtime.runtime_status,
        crate::modules::workspace::domain::runtime_state::RuntimeStatus::Clean
    );
}

#[tokio::test]
async fn active_pointer_failure_keeps_the_apply_task_pending_for_recovery() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();
    seed_game(&ctx.pool, "game-finalize-gap", Some(&mods_path)).await;

    for (id, name) in [("baseline", "Baseline"), ("target", "Target")] {
        let collection =
            collection::create(&ctx.pool, id, "game-finalize-gap", name, true, false)
                .await
                .expect("create collection");
        persist_projected_state(
            &ctx.pool,
            &collection.id,
            &[],
            &[],
            &projected_state::empty_projected_state(),
        )
        .await
        .expect("persist empty collection");
    }
    crate::modules::collections::adapters::outbound::sqlite::runtime::set_active(
        &ctx.pool,
        "game-finalize-gap",
        Some("baseline"),
    )
    .await
    .expect("set initial baseline");
    sqlx::query(
        r#"CREATE TRIGGER fail_active_pointer_finalize
           BEFORE UPDATE OF active_collection_id ON collection_runtime_state
           WHEN NEW.game_id = 'game-finalize-gap'
           BEGIN
             SELECT RAISE(ABORT, 'injected active pointer failure');
           END"#,
    )
    .execute(&ctx.pool)
    .await
    .expect("install active pointer failpoint");

    apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-finalize-gap",
        collection_id: "target",
        capture_last_changes: true,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    })
    .await
    .expect_err("injected active pointer write must fail");

    let tasks = crate::modules::workspace::adapters::outbound::sqlite::task::get_all_pending_tasks_global(&ctx.pool)
        .await
        .expect("load pending recovery tasks");
    let task = tasks
        .iter()
        .find(|task| {
            task.game_id == "game-finalize-gap" && task.target_id.as_deref() == Some("target")
        })
        .expect("an apply without its active pointer must remain pending");
    assert_eq!(task.rollback_collection_id.as_deref(), Some("baseline"));
    assert_eq!(
        task.rollback_active_collection_id.as_deref(),
        Some("baseline")
    );
}

#[tokio::test]
async fn pending_apply_rejects_another_apply_before_draft_capture() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();
    seed_game(&ctx.pool, "game-open-apply", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-open-apply").await;
    create_flat_mod_folder(mods_root.path(), "AINOZ/Blue");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-open-apply",
            game_id: "game-open-apply",
            object_id: Some("object-1"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert live mod");

    for (id, name, is_unsaved) in [
        ("existing-draft", "Last changes", true),
        ("blocked-target", "Target", false),
    ] {
        collection::create(&ctx.pool, id, "game-open-apply", name, true, is_unsaved)
            .await
            .expect("create collection");
        persist_projected_state(
            &ctx.pool,
            id,
            &[],
            &[],
            &projected_state::empty_projected_state(),
        )
        .await
        .expect("persist empty state");
    }
    let mut connection = ctx
        .pool
        .acquire()
        .await
        .expect("acquire runtime connection");
    crate::modules::collections::adapters::outbound::sqlite::runtime::set_draft_tx(
        &mut connection,
        "game-open-apply",
        "existing-draft",
        None,
    )
    .await
    .expect("set existing draft");
    drop(connection);
    let original_signature = collection::get_by_id(&ctx.pool, "existing-draft")
        .await
        .expect("load draft")
        .expect("draft exists")
        .signature;
    crate::modules::workspace::adapters::outbound::sqlite::task::create_task(
        &ctx.pool,
        "existing-apply",
        "game-open-apply",
        "apply_collection",
        Some("other-target"),
    )
    .await
    .expect("create pending apply");

    apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-open-apply",
        collection_id: "blocked-target",
        capture_last_changes: true,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    })
    .await
    .expect_err("another open apply must reject before capturing Last changes");

    let current_signature = collection::get_by_id(&ctx.pool, "existing-draft")
        .await
        .expect("reload draft")
        .expect("draft exists")
        .signature;
    assert_eq!(current_signature, original_signature);
}

#[derive(Clone, Copy)]
enum ReplaceFailpoint {
    SafetySummary,
    ActivePointer,
    DraftPointerCleanup,
    DraftDelete,
}

impl ReplaceFailpoint {
    fn trigger_sql(self) -> &'static str {
        match self {
            Self::SafetySummary => {
                r#"CREATE TRIGGER fail_replace_write
                   BEFORE UPDATE OF is_safe ON collections
                   WHEN OLD.id = 'replace-target'
                   BEGIN SELECT RAISE(ABORT, 'injected safety failure'); END"#
            }
            Self::ActivePointer => {
                r#"CREATE TRIGGER fail_replace_write
                   BEFORE UPDATE OF active_collection_id ON collection_runtime_state
                   WHEN NEW.game_id = 'game-replace-atomic'
                   BEGIN SELECT RAISE(ABORT, 'injected active pointer failure'); END"#
            }
            Self::DraftPointerCleanup => {
                r#"CREATE TRIGGER fail_replace_write
                   BEFORE UPDATE OF draft_collection_id ON collection_runtime_state
                   WHEN NEW.game_id = 'game-replace-atomic'
                   BEGIN SELECT RAISE(ABORT, 'injected draft pointer failure'); END"#
            }
            Self::DraftDelete => {
                r#"CREATE TRIGGER fail_replace_write
                   BEFORE DELETE ON collections
                   WHEN OLD.id = 'replace-draft'
                   BEGIN SELECT RAISE(ABORT, 'injected draft delete failure'); END"#
            }
        }
    }
}

async fn assert_replace_failure_is_atomic(failpoint: ReplaceFailpoint) {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();
    seed_game(&ctx.pool, "game-replace-atomic", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-replace-atomic").await;
    create_flat_mod_folder(mods_root.path(), "AINOZ/Live");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "live-mod",
            game_id: "game-replace-atomic",
            object_id: Some("object-1"),
            actual_name: "Live",
            folder_path: "AINOZ/Live",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert live mod");

    for (id, name, is_safe, is_unsaved) in [
        ("replace-baseline", "Baseline", true, false),
        ("replace-target", "Target", false, false),
        ("replace-draft", "Last changes", true, true),
    ] {
        collection::create(
            &ctx.pool,
            id,
            "game-replace-atomic",
            name,
            is_safe,
            is_unsaved,
        )
        .await
        .expect("create replace fixture collection");
    }
    let stored_mod = test_collection_mod("replace-target", "AINOZ/Stored", "Stored");
    let stored_object = test_collection_object("replace-target");
    let stored_state = projected_state::build_projected_state(
        std::slice::from_ref(&stored_mod),
        std::slice::from_ref(&stored_object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        "replace-target",
        &[stored_mod],
        &[stored_object],
        &stored_state,
    )
    .await
    .expect("persist original target snapshot");
    let mut connection = ctx
        .pool
        .acquire()
        .await
        .expect("acquire runtime connection");
    crate::modules::collections::adapters::outbound::sqlite::runtime::set_active_tx(
        &mut connection,
        "game-replace-atomic",
        Some("replace-baseline"),
    )
    .await
    .expect("set initial active baseline");
    crate::modules::collections::adapters::outbound::sqlite::runtime::set_draft_tx(
        &mut connection,
        "game-replace-atomic",
        "replace-draft",
        Some("replace-baseline"),
    )
    .await
    .expect("set initial draft");
    drop(connection);

    sqlx::query(failpoint.trigger_sql())
        .execute(&ctx.pool)
        .await
        .expect("install replace failpoint");
    replace_collection_with_current_state(&ctx.pool, "game-replace-atomic", "replace-target")
        .await
        .expect_err("injected replace write must fail");

    let stored_paths: Vec<String> = collection::get_mods(&ctx.pool, "replace-target")
        .await
        .expect("reload target members")
        .into_iter()
        .map(|member| member.mod_path)
        .collect();
    let runtime = crate::modules::collections::adapters::outbound::sqlite::runtime::get(&ctx.pool, "game-replace-atomic")
        .await
        .expect("reload runtime state")
        .expect("runtime state exists");
    let draft_exists = collection::get_by_id(&ctx.pool, "replace-draft")
        .await
        .expect("reload draft")
        .is_some();
    let target_is_safe = collection::get_by_id(&ctx.pool, "replace-target")
        .await
        .expect("reload target")
        .expect("target exists")
        .is_safe;

    assert_eq!(
        (
            stored_paths,
            target_is_safe,
            runtime.active_collection_id,
            runtime.draft_collection_id,
            draft_exists,
        ),
        (
            vec!["AINOZ/Stored".to_string()],
            false,
            Some("replace-baseline".to_string()),
            Some("replace-draft".to_string()),
            true,
        ),
        "a failed replace must preserve its complete pre-operation state"
    );
}

#[tokio::test]
async fn replace_is_atomic_when_safety_summary_write_fails() {
    assert_replace_failure_is_atomic(ReplaceFailpoint::SafetySummary).await;
}

#[tokio::test]
async fn replace_is_atomic_when_active_pointer_write_fails() {
    assert_replace_failure_is_atomic(ReplaceFailpoint::ActivePointer).await;
}

#[tokio::test]
async fn replace_is_atomic_when_draft_pointer_cleanup_fails() {
    assert_replace_failure_is_atomic(ReplaceFailpoint::DraftPointerCleanup).await;
}

#[tokio::test]
async fn replace_is_atomic_when_draft_delete_fails() {
    assert_replace_failure_is_atomic(ReplaceFailpoint::DraftDelete).await;
}
