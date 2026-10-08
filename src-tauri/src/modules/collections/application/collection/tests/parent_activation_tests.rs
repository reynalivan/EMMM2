use super::*;

async fn apply_with_dormant_nonmember(safe_mode: bool) {
    let ctx = init_test_db().await;
    let root = tempfile::tempdir().expect("mods root");
    let mods_path = root.path().to_string_lossy().to_string();
    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
    for (id, name, enabled) in [("wanted", "Blue", false), ("extra", "Red", true)] {
        let path = format!(
            "DISABLED AINOZ/{}{name}",
            if enabled { "" } else { "DISABLED " }
        );
        create_flat_mod_folder(root.path(), &path);
        insert_test_mod(
            &ctx.pool,
            &TestModFixture {
                id,
                game_id: "game-1",
                object_id: Some("object-1"),
                actual_name: name,
                folder_path: &path,
                status: if enabled {
                    ItemStatus::Enabled
                } else {
                    ItemStatus::Disabled
                },
                is_safe: !enabled,
                object_type: Some("Character"),
                mods_path: Some(&mods_path),
            },
        )
        .await
        .expect("seed mod");
    }
    sqlx::query("UPDATE mods SET safety_source = 'manual' WHERE game_id = 'game-1'")
        .execute(&ctx.pool)
        .await
        .expect("classify mods");
    sqlx::query(
        "UPDATE objects SET folder_path = 'DISABLED AINOZ', status = 0 WHERE id = 'object-1'",
    )
    .execute(&ctx.pool)
    .await
    .expect("disable object");
    let preset = collection::create(&ctx.pool, "preset", "game-1", "Preset", true, false)
        .await
        .expect("preset");
    let mods = vec![test_collection_mod(&preset.id, "AINOZ/Blue", "Blue")];
    let objects = vec![test_collection_object(&preset.id)];
    let state = projected_state::build_projected_state(&mods, &objects, Some(&mods_path));
    persist_projected_state(&ctx.pool, &preset.id, &mods, &objects, &state)
        .await
        .expect("save preset");
    let mut settings = AppSettings::default();
    settings
        .safety
        .set_runtime_safe_mode("game-1".to_string(), safe_mode);

    apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-1",
        collection_id: &preset.id,
        capture_last_changes: true,
        mods_path: root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings,
    })
    .await
    .expect("apply");

    assert!(root.path().join("AINOZ/Blue").is_dir());
    assert!(
        root.path().join("AINOZ/DISABLED Red").is_dir(),
        "nonmember must stay disabled when its parent is activated"
    );
    assert!(!root.path().join("AINOZ/Red").exists());
}

#[tokio::test]
async fn parent_activation_disables_dormant_nonmembers() {
    apply_with_dormant_nonmember(false).await;
}

#[tokio::test]
async fn safe_collection_parent_activation_cannot_enable_dormant_unsafe_mods() {
    apply_with_dormant_nonmember(true).await;
}

#[cfg(windows)]
#[tokio::test]
async fn locked_collection_member_keeps_structured_busy_error() {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    let ctx = init_test_db().await;
    let root = tempfile::tempdir().unwrap();
    let mods_path = root.path().to_string_lossy().to_string();
    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
    create_flat_mod_folder(root.path(), "AINOZ/DISABLED Blue");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "blue",
            game_id: "game-1",
            object_id: Some("object-1"),
            actual_name: "Blue",
            folder_path: "AINOZ/DISABLED Blue",
            status: ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .unwrap();
    let preset = collection::create(&ctx.pool, "preset", "game-1", "Preset", true, false)
        .await
        .unwrap();
    let mods = vec![test_collection_mod(&preset.id, "AINOZ/Blue", "Blue")];
    let objects = vec![test_collection_object(&preset.id)];
    let state = projected_state::build_projected_state(&mods, &objects, Some(&mods_path));
    persist_projected_state(&ctx.pool, &preset.id, &mods, &objects, &state)
        .await
        .unwrap();
    let handle = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(root.path().join("AINOZ/DISABLED Blue"))
        .unwrap();

    let error = apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-1",
        collection_id: &preset.id,
        capture_last_changes: true,
        mods_path: root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    })
    .await
    .expect_err("locked folder must fail with a structured error");
    assert!(matches!(
        error,
        CollectionError::FileInUse { .. } | CollectionError::PathBusy { .. }
    ));
    assert!(root.path().join("AINOZ/DISABLED Blue").is_dir());
    assert!(!root.path().join("AINOZ/Blue").exists());
    drop(handle);
}
