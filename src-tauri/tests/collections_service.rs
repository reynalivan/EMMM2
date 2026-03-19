use emmm2_lib::services::collections::{
    apply_collection, auto_disable_auto_tagged_outside_corridor, create_collection,
    get_collection_runtime_preview, list_collections, CreateCollectionInput,
};
use emmm2_lib::services::corridor_runtime::get_corridor_runtime_snapshot;
use emmm2_lib::services::fs_utils::operation_lock::OperationLock;
use emmm2_lib::services::mods::core_ops::toggle_mod_inner_service;
use emmm2_lib::services::scanner::watcher::WatcherState;
use std::fs;
mod common;
use tempfile::TempDir;

async fn setup_pool() -> sqlx::SqlitePool {
    let ctx = common::init_test_db().await;
    ctx.pool
}

async fn seed_game_and_mods(
    pool: &sqlx::SqlitePool,
    mods_dir: &str,
) -> (String, String, String, String, String) {
    let game_id = "game-gimi".to_string();
    let mod_a_id = "mod-a".to_string();
    let mod_b_id = "mod-b".to_string();
    let object_id = "obj-raiden".to_string();

    sqlx::query("INSERT INTO games (id, name, game_type, path, mod_path) VALUES (?, ?, ?, ?, ?)")
        .bind(&game_id)
        .bind("Genshin")
        .bind("GIMI")
        .bind(mods_dir)
        .bind(mods_dir)
        .execute(pool)
        .await
        .expect("insert game");

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, sort_order, created_at) VALUES (?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(&object_id)
    .bind(&game_id)
    .bind("Raiden Shogun")
    .bind("Character")
    .bind(0)
    .execute(pool)
    .await
    .expect("insert object");

    let mod_a_path = format!("{mods_dir}/DISABLED RaidenA");
    let mod_b_path = format!("{mods_dir}/RaidenB");

    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&mod_a_id)
    .bind(&game_id)
    .bind("Raiden A")
    .bind(&mod_a_path)
    .bind("DISABLED")
    .bind(&object_id)
    .execute(pool)
    .await
    .expect("insert mod a");

    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&mod_b_id)
    .bind(&game_id)
    .bind("Raiden B")
    .bind(&mod_b_path)
    .bind("ENABLED")
    .bind(&object_id)
    .execute(pool)
    .await
    .expect("insert mod b");

    common::refresh_unicode_keys(pool).await;

    (game_id, mod_a_id, mod_b_id, mod_a_path, mod_b_path)
}

#[tokio::test]
async fn collections_create_and_list() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let (game_id, mod_a_id, _, _, _) = seed_game_and_mods(&pool, &mods_dir).await;

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "Abyss Team".to_string(),
            game_id: game_id.clone(),
            is_safe_context: true,
            mod_ids: vec![mod_a_id],
            auto_snapshot: None,
            object_states: None,
        },
    )
    .await
    .expect("create collection");

    let listed = list_collections(&pool, &game_id, true)
        .await
        .expect("list collections");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.collection.id);
    assert_eq!(listed[0].name, "Abyss Team");
}

#[tokio::test]
async fn collections_apply_then_undo_restores_state() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let (game_id, mod_a_id, mod_b_id, mod_a_path, mod_b_path) =
        seed_game_and_mods(&pool, &mods_dir).await;

    fs::create_dir_all(&mod_a_path).expect("create disabled mod a folder");
    fs::create_dir_all(&mod_b_path).expect("create enabled mod b folder");

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "Abyss Team".to_string(),
            game_id: game_id.clone(),
            is_safe_context: true,
            mod_ids: vec![mod_a_id.clone()],
            auto_snapshot: None,
            object_states: None,
        },
    )
    .await
    .expect("create collection");

    let watcher_state = WatcherState::new();

    let applied = apply_collection(
        &pool,
        &watcher_state,
        &created.collection.id,
        &game_id,
        true, // Matches the is_safe_context: true above
    )
    .await
    .expect("apply collection");

    assert_eq!(applied.changed_count, 2);

    let mod_a_status: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = ?")
        .bind(&mod_a_id)
        .fetch_one(&pool)
        .await
        .expect("mod a status after apply");
    let mod_b_status: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = ?")
        .bind(&mod_b_id)
        .fetch_one(&pool)
        .await
        .expect("mod b status after apply");

    assert_eq!(mod_a_status, "ENABLED");
    assert_eq!(mod_b_status, "DISABLED");

    let snapshot_id: String =
        sqlx::query_scalar("SELECT id FROM collections WHERE game_id = ? AND is_last_unsaved = 1")
            .bind(&game_id)
            .fetch_one(&pool)
            .await
            .expect("snapshot collection");

    let undo = apply_collection(&pool, &watcher_state, &snapshot_id, &game_id, true) // Matches the safe mode context above
        .await
        .expect("undo apply via snapshot");

    assert_eq!(undo.changed_count, 2);

    let mod_a_status_after: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = ?")
        .bind(&mod_a_id)
        .fetch_one(&pool)
        .await
        .expect("mod a status after undo");
    let mod_b_status_after: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = ?")
        .bind(&mod_b_id)
        .fetch_one(&pool)
        .await
        .expect("mod b status after undo");

    assert_eq!(mod_a_status_after, "DISABLED");
    assert_eq!(mod_b_status_after, "ENABLED");
}

#[tokio::test]
async fn collections_apply_keeps_named_active_state_on_follow_up_reads() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let (game_id, mod_a_id, _, mod_a_path, mod_b_path) = seed_game_and_mods(&pool, &mods_dir).await;

    fs::create_dir_all(&mod_a_path).expect("create disabled mod a folder");
    fs::create_dir_all(&mod_b_path).expect("create enabled mod b folder");

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "Strict Named".to_string(),
            game_id: game_id.clone(),
            is_safe_context: true,
            mod_ids: vec![mod_a_id],
            auto_snapshot: None,
            object_states: None,
        },
    )
    .await
    .expect("create named collection");

    apply_collection(
        &pool,
        &WatcherState::new(),
        &created.collection.id,
        &game_id,
        true,
    )
    .await
    .expect("apply named collection");

    let first_read = get_corridor_runtime_snapshot(&pool, &game_id, true)
        .await
        .expect("resolve strict active state after apply");
    let second_read = get_corridor_runtime_snapshot(&pool, &game_id, true)
        .await
        .expect("resolve strict active state on follow-up read");

    assert_eq!(
        first_read.active_collection_id.as_deref(),
        Some(created.collection.id.as_str())
    );
    assert_eq!(first_read.state_name.as_deref(), Some("Strict Named"));
    assert_eq!(
        first_read.state_kind,
        emmm2_lib::services::collections::types::CollectionStateKind::Named
    );

    assert_eq!(
        second_read.active_collection_id.as_deref(),
        Some(created.collection.id.as_str())
    );
    assert_eq!(second_read.state_name.as_deref(), Some("Strict Named"));
    assert_eq!(
        second_read.state_kind,
        emmm2_lib::services::collections::types::CollectionStateKind::Named
    );

    let cached_signature: String = sqlx::query_scalar(
        "SELECT signature FROM corridor_runtime_cache WHERE game_id = ? AND is_safe = 1",
    )
    .bind(&game_id)
    .fetch_one(&pool)
    .await
    .expect("load runtime cache signature");
    assert_eq!(cached_signature, second_read.signature);
}

#[tokio::test]
async fn corridor_runtime_snapshot_backfills_missing_collection_materialization() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let (game_id, _, mod_b_id, _, _) = seed_game_and_mods(&pool, &mods_dir).await;

    let object_root = tmp.path().join("Raiden Shogun");
    let enabled_root = object_root.join("RaidenB");
    fs::create_dir_all(&enabled_root).expect("create enabled mod folder");
    fs::write(
        enabled_root.join("mod.ini"),
        "[TextureOverrideMain]\nhash = deadbeef\n",
    )
    .expect("write enabled mod ini");
    sqlx::query("UPDATE mods SET folder_path = ? WHERE id = ?")
        .bind(enabled_root.to_string_lossy().to_string())
        .bind(&mod_b_id)
        .execute(&pool)
        .await
        .expect("update enabled mod path");
    common::refresh_unicode_keys(&pool).await;

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "Backfill Named".to_string(),
            game_id: game_id.clone(),
            is_safe_context: true,
            mod_ids: vec![mod_b_id.clone()],
            auto_snapshot: None,
            object_states: None,
        },
    )
    .await
    .expect("create named collection");

    sqlx::query("UPDATE collections SET snapshot_json = NULL, signature = NULL, root_count = 0 WHERE id = ?")
        .bind(&created.collection.id)
        .execute(&pool)
        .await
        .expect("clear collection snapshot columns");
    sqlx::query(
        "INSERT INTO collection_items (collection_id, mod_id, mod_path, mod_path_key)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&created.collection.id)
    .bind(&mod_b_id)
    .bind(enabled_root.to_string_lossy().to_string())
    .bind("raiden shogun/raidenb")
    .execute(&pool)
    .await
    .expect("insert legacy collection item");
    sqlx::query(
        "INSERT INTO corridor_state (game_id, is_safe, active_collection_id)
         VALUES (?, 1, ?)
         ON CONFLICT(game_id, is_safe) DO UPDATE SET active_collection_id = excluded.active_collection_id",
    )
    .bind(&game_id)
    .bind(&created.collection.id)
    .execute(&pool)
    .await
    .expect("set active collection pointer");

    let snapshot = get_corridor_runtime_snapshot(&pool, &game_id, true)
        .await
        .expect("resolve current runtime snapshot");

    let rooted_count: i64 =
        sqlx::query_scalar("SELECT COALESCE(root_count, 0) FROM collections WHERE id = ?")
            .bind(&created.collection.id)
            .fetch_one(&pool)
            .await
            .expect("count collection roots");
    let signature_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM collections WHERE id = ? AND signature IS NOT NULL",
    )
    .bind(&created.collection.id)
    .fetch_one(&pool)
    .await
    .expect("count collection signatures");

    assert_eq!(
        snapshot.active_collection_id.as_deref(),
        Some(created.collection.id.as_str())
    );
    assert_eq!(snapshot.state_name.as_deref(), Some("Backfill Named"));
    assert_eq!(
        snapshot.state_kind,
        emmm2_lib::services::collections::types::CollectionStateKind::Named
    );
    assert!(rooted_count > 0);
    assert_eq!(signature_count, 1);
}

#[tokio::test]
async fn collections_create_stores_full_object_state_universe() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let (game_id, mod_a_id, _, _, _) = seed_game_and_mods(&pool, &mods_dir).await;

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, sort_order, created_at) VALUES (?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind("obj-extra")
    .bind(&game_id)
    .bind("Extra Object")
    .bind("Character")
    .bind(1)
    .execute(&pool)
    .await
    .expect("insert extra object");
    common::refresh_unicode_keys(&pool).await;

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "Full Snapshot".to_string(),
            game_id: game_id.clone(),
            is_safe_context: true,
            mod_ids: vec![mod_a_id],
            auto_snapshot: None,
            object_states: None,
        },
    )
    .await
    .expect("create collection");

    let saved_states = created
        .object_states
        .iter()
        .map(|state| (state.object_id.as_str(), state.is_enabled))
        .collect::<Vec<_>>();

    assert_eq!(saved_states.len(), 2);
    assert!(saved_states.contains(&("obj-raiden", true)));
    assert!(saved_states.contains(&("obj-extra", true)));
}

#[tokio::test]
async fn collections_active_state_matches_unicode_named_collection() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let game_id = "game-unicode-active".to_string();
    let object_id = "obj-unicode".to_string();
    let mod_id = "mod-unicode".to_string();

    sqlx::query("INSERT INTO games (id, name, game_type, path, mod_path) VALUES (?, ?, ?, ?, ?)")
        .bind(&game_id)
        .bind("Genshin")
        .bind("GIMI")
        .bind(&mods_dir)
        .bind(&mods_dir)
        .execute(&pool)
        .await
        .expect("insert unicode game");

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, folder_path, sort_order, created_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(&object_id)
    .bind(&game_id)
    .bind("한국 캐릭터")
    .bind("Character")
    .bind("한국 캐릭터")
    .bind(0)
    .execute(&pool)
    .await
    .expect("insert unicode object");

    let mod_relative_path = "한국 캐릭터/日本語모드";
    let mod_dir = tmp.path().join("한국 캐릭터").join("日本語모드");
    fs::create_dir_all(&mod_dir).expect("create unicode mod folder");
    fs::write(
        mod_dir.join("mod.ini"),
        "[TextureOverrideMain]\nhash = abc123\n",
    )
    .expect("write unicode mod.ini");

    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&mod_id)
    .bind(&game_id)
    .bind("日本語모드")
    .bind(mod_relative_path)
    .bind("ENABLED")
    .bind(&object_id)
    .execute(&pool)
    .await
    .expect("insert unicode mod");
    common::refresh_unicode_keys(&pool).await;

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "한글 프리셋".to_string(),
            game_id: game_id.clone(),
            is_safe_context: true,
            mod_ids: vec![mod_id.clone()],
            auto_snapshot: None,
            object_states: None,
        },
    )
    .await
    .expect("create unicode collection");

    let active_state = get_corridor_runtime_snapshot(&pool, &game_id, true)
        .await
        .expect("resolve active unicode collection");

    assert_eq!(
        active_state.active_collection_id.as_deref(),
        Some(created.collection.id.as_str())
    );
    assert_eq!(active_state.state_name.as_deref(), Some("한글 프리셋"));
    assert_eq!(active_state.roots.len(), 1);
    assert_eq!(active_state.roots[0].actual_name, "日本語모드");
}

#[tokio::test]
async fn collections_apply_disabled_object_state_keeps_named_active_state() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let game_id = "game-object-state".to_string();
    let object_id = "obj-aino".to_string();
    let mod_id = "mod-aino".to_string();

    sqlx::query("INSERT INTO games (id, name, game_type, path, mod_path) VALUES (?, ?, ?, ?, ?)")
        .bind(&game_id)
        .bind("Genshin")
        .bind("GIMI")
        .bind(&mods_dir)
        .bind(&mods_dir)
        .execute(&pool)
        .await
        .expect("insert game");

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, folder_path, sort_order, created_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(&object_id)
    .bind(&game_id)
    .bind("Ainoz")
    .bind("Character")
    .bind("Ainoz")
    .bind(0)
    .execute(&pool)
    .await
    .expect("insert object");

    let mod_path = "Ainoz/Look1".to_string();
    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&mod_id)
    .bind(&game_id)
    .bind("Look 1")
    .bind(&mod_path)
    .bind("ENABLED")
    .bind(&object_id)
    .execute(&pool)
    .await
    .expect("insert mod");

    fs::create_dir_all(tmp.path().join("Ainoz").join("Look1")).expect("create object folder");
    common::refresh_unicode_keys(&pool).await;

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "Object Disabled".to_string(),
            game_id: game_id.clone(),
            is_safe_context: true,
            mod_ids: Vec::new(),
            auto_snapshot: None,
            object_states: Some(vec![
                emmm2_lib::services::collections::types::CollectionObjectState {
                    object_id: object_id.clone(),
                    is_enabled: false,
                },
            ]),
        },
    )
    .await
    .expect("create collection");

    apply_collection(
        &pool,
        &WatcherState::new(),
        &created.collection.id,
        &game_id,
        true,
    )
    .await
    .expect("apply collection");

    let object_folder_path: String =
        sqlx::query_scalar("SELECT folder_path FROM objects WHERE id = ?")
            .bind(&object_id)
            .fetch_one(&pool)
            .await
            .expect("load object folder path");
    let mod_folder_path: String = sqlx::query_scalar("SELECT folder_path FROM mods WHERE id = ?")
        .bind(&mod_id)
        .fetch_one(&pool)
        .await
        .expect("load mod folder path");

    assert_eq!(object_folder_path, "DISABLED Ainoz");
    assert!(mod_folder_path.starts_with("DISABLED Ainoz/"));

    let overview = get_corridor_runtime_snapshot(&pool, &game_id, true)
        .await
        .expect("resolve strict active state");

    assert_eq!(
        overview.active_collection_id.as_deref(),
        Some(created.collection.id.as_str())
    );
    assert_eq!(overview.state_name.as_deref(), Some("Object Disabled"));
    assert_eq!(
        overview.state_kind,
        emmm2_lib::services::collections::types::CollectionStateKind::Named
    );
}

#[tokio::test]
async fn collections_manual_mod_toggle_marks_runtime_unsaved_until_match_restored() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let game_id = "game-manual-drift".to_string();
    let object_id = "obj-raiden-drift".to_string();
    let mod_b_id = "mod-b-drift".to_string();
    let mod_b_path = format!("{mods_dir}/Raiden/LookB");

    sqlx::query("INSERT INTO games (id, name, game_type, path, mod_path) VALUES (?, ?, ?, ?, ?)")
        .bind(&game_id)
        .bind("Genshin")
        .bind("GIMI")
        .bind(&mods_dir)
        .bind(&mods_dir)
        .execute(&pool)
        .await
        .expect("insert drift game");

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, folder_path, sort_order, created_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(&object_id)
    .bind(&game_id)
    .bind("Raiden Shogun")
    .bind("Character")
    .bind("Raiden")
    .bind(0)
    .execute(&pool)
    .await
    .expect("insert drift object");

    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&mod_b_id)
    .bind(&game_id)
    .bind("Look B")
    .bind("Raiden/LookB")
    .bind("ENABLED")
    .bind(&object_id)
    .execute(&pool)
    .await
    .expect("insert enabled drift mod");

    fs::create_dir_all(tmp.path().join("Raiden").join("LookB"))
        .expect("create enabled drift mod folder");
    fs::write(
        tmp.path().join("Raiden").join("LookB").join("mod.ini"),
        "[TextureOverrideMain]\nhash = abc123\n",
    )
    .expect("write drift mod ini");
    common::refresh_unicode_keys(&pool).await;

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "Runtime Drift".to_string(),
            game_id: game_id.clone(),
            is_safe_context: true,
            mod_ids: Vec::new(),
            auto_snapshot: Some(true),
            object_states: None,
        },
    )
    .await
    .expect("save current state as collection");

    let active_named = get_corridor_runtime_snapshot(&pool, &game_id, true)
        .await
        .expect("named runtime snapshot");
    assert_eq!(active_named.active_collection_id.as_deref(), Some(created.collection.id.as_str()));
    assert_eq!(
        active_named.state_kind,
        emmm2_lib::services::collections::types::CollectionStateKind::Named
    );

    let op_lock = OperationLock::new();
    let watcher = WatcherState::new();
    let toggled_disabled_path = toggle_mod_inner_service(
        &pool,
        &watcher,
        &op_lock,
        mod_b_path.clone(),
        false,
        &game_id,
    )
    .await
    .expect("disable active mod");
    assert!(toggled_disabled_path.contains("DISABLED"));

    let drifted = get_corridor_runtime_snapshot(&pool, &game_id, true)
        .await
        .expect("drifted runtime snapshot");
    assert_eq!(drifted.active_collection_id, None);
    assert_eq!(drifted.state_name.as_deref(), Some("Unsaved Preset"));
    assert_eq!(
        drifted.state_kind,
        emmm2_lib::services::collections::types::CollectionStateKind::Unsaved
    );

    let restored_path = toggle_mod_inner_service(
        &pool,
        &watcher,
        &op_lock,
        toggled_disabled_path,
        true,
        &game_id,
    )
    .await
    .expect("re-enable active mod");
    assert_eq!(restored_path.replace('\\', "/"), mod_b_path.replace('\\', "/"));

    let restored = get_corridor_runtime_snapshot(&pool, &game_id, true)
        .await
        .expect("restored runtime snapshot");
    assert_eq!(restored.active_collection_id.as_deref(), Some(created.collection.id.as_str()));
    assert_eq!(
        restored.state_kind,
        emmm2_lib::services::collections::types::CollectionStateKind::Named
    );

}

#[tokio::test]
async fn collections_preview_filters_disabled_unicode_nested_path() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let game_id = "game-unicode-preview".to_string();
    let object_id = "obj-preview".to_string();
    let mod_id = "mod-preview".to_string();
    let collection_id = "collection-preview-unicode";

    sqlx::query("INSERT INTO games (id, name, game_type, path, mod_path) VALUES (?, ?, ?, ?, ?)")
        .bind(&game_id)
        .bind("Genshin")
        .bind("GIMI")
        .bind(&mods_dir)
        .bind(&mods_dir)
        .execute(&pool)
        .await
        .expect("insert preview game");

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, folder_path, sort_order, created_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(&object_id)
    .bind(&game_id)
    .bind("한국 컨테이너")
    .bind("Character")
    .bind("한국 컨테이너")
    .bind(0)
    .execute(&pool)
    .await
    .expect("insert preview object");

    let enabled_mod_dir = tmp.path().join("한국 컨테이너").join("日本語루트");
    let disabled_nested_dir = tmp
        .path()
        .join("한국 컨테이너")
        .join("중첩")
        .join("DISABLED 中文모드");
    fs::create_dir_all(&enabled_mod_dir).expect("create enabled unicode root");
    fs::create_dir_all(&disabled_nested_dir).expect("create disabled unicode nested root");
    fs::write(
        enabled_mod_dir.join("mod.ini"),
        "[TextureOverrideMain]\nhash = abc123\n",
    )
    .expect("write enabled unicode mod.ini");
    fs::write(
        disabled_nested_dir.join("mod.ini"),
        "[TextureOverrideMain]\nhash = def456\n",
    )
    .expect("write disabled unicode mod.ini");

    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&mod_id)
    .bind(&game_id)
    .bind("日本語루트")
    .bind("한국 컨테이너/日本語루트")
    .bind("ENABLED")
    .bind(&object_id)
    .execute(&pool)
    .await
    .expect("insert preview mod");

    sqlx::query(
        "INSERT INTO collections (id, name, game_id, is_safe_context, is_last_unsaved) VALUES (?, ?, ?, 1, 0)",
    )
    .bind(collection_id)
    .bind("유니코드 미리보기")
    .bind(&game_id)
    .execute(&pool)
    .await
    .expect("insert preview collection");

    sqlx::query("INSERT INTO collection_items (collection_id, mod_id, mod_path) VALUES (?, ?, ?)")
        .bind(collection_id)
        .bind(&mod_id)
        .bind("한국 컨테이너/日本語루트")
        .execute(&pool)
        .await
        .expect("insert collection item");

    sqlx::query("INSERT INTO collection_nested_items (collection_id, mod_path) VALUES (?, ?)")
        .bind(collection_id)
        .bind(disabled_nested_dir.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .expect("insert disabled unicode nested item");

    sqlx::query(
        "INSERT INTO collection_object_states (collection_id, object_id, is_enabled) VALUES (?, ?, 1)",
    )
    .bind(collection_id)
    .bind(&object_id)
    .execute(&pool)
    .await
    .expect("insert collection object state");
    common::refresh_unicode_keys(&pool).await;

    let preview = get_collection_runtime_preview(&pool, collection_id, &game_id)
        .await
        .expect("get unicode collection preview");

    assert_eq!(preview.roots.len(), 1);
    assert_eq!(preview.roots[0].actual_name, "日本語루트");
}

#[tokio::test]
async fn collections_auto_snapshot_ignores_disabled_unicode_ancestor_and_keeps_enabled_unicode_path(
) {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let game_id = "game-unicode-snapshot".to_string();

    sqlx::query("INSERT INTO games (id, name, game_type, path, mod_path) VALUES (?, ?, ?, ?, ?)")
        .bind(&game_id)
        .bind("Genshin")
        .bind("GIMI")
        .bind(&mods_dir)
        .bind(&mods_dir)
        .execute(&pool)
        .await
        .expect("insert snapshot game");

    let disabled_nested_dir = tmp
        .path()
        .join("Ainoz")
        .join("DISABLED 아이농")
        .join("아이농 누드");
    let enabled_nested_dir = tmp
        .path()
        .join("Albedo")
        .join("ALBEDO 알베도 폰타인 복장 (TS)")
        .join("Albedo_Fountain_TS");

    fs::create_dir_all(&disabled_nested_dir).expect("create disabled unicode nested path");
    fs::create_dir_all(&enabled_nested_dir).expect("create enabled unicode nested path");
    fs::write(
        disabled_nested_dir.join("mod.ini"),
        "[TextureOverrideMain]\nhash = abc123\n",
    )
    .expect("write disabled nested mod.ini");
    fs::write(
        enabled_nested_dir.join("mod.ini"),
        "[TextureOverrideMain]\nhash = def456\n",
    )
    .expect("write enabled nested mod.ini");

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "wae".to_string(),
            game_id: game_id.clone(),
            is_safe_context: true,
            mod_ids: Vec::new(),
            auto_snapshot: Some(true),
            object_states: None,
        },
    )
    .await
    .expect("create auto snapshot collection");

    let disabled_saved_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM collection_nested_items WHERE collection_id = ? AND mod_path LIKE ?",
    )
    .bind(&created.collection.id)
    .bind("%아이농%")
    .fetch_one(&pool)
    .await
    .expect("count disabled unicode nested rows");
    assert_eq!(disabled_saved_count, 0);

    let preview = get_collection_runtime_preview(&pool, &created.collection.id, &game_id)
        .await
        .expect("preview auto snapshot collection");
    assert_eq!(preview.roots.len(), 1);
    assert_eq!(preview.roots[0].actual_name, "Albedo_Fountain_TS");
    assert_eq!(preview.roots[0].node_type.as_deref(), Some("FlatModRoot"));
    assert!(!preview.roots[0].folder_path.contains("아이농"));
    assert!(preview.roots[0]
        .folder_path
        .contains("ALBEDO 알베도 폰타인 복장 (TS)"));

    let active_state = get_corridor_runtime_snapshot(&pool, &game_id, true)
        .await
        .expect("resolve active collection after auto snapshot");
    assert_eq!(
        active_state.active_collection_id.as_deref(),
        Some(created.collection.id.as_str())
    );
    assert_eq!(active_state.roots.len(), 1);
    assert_eq!(active_state.roots[0].actual_name, "Albedo_Fountain_TS");
    assert_eq!(
        active_state.roots[0].node_type.as_deref(),
        Some("FlatModRoot")
    );
    assert!(!active_state.roots[0].folder_path.contains("아이농"));
    assert!(active_state.roots[0]
        .folder_path
        .contains("ALBEDO 알베도 폰타인 복장 (TS)"));
}

#[tokio::test]
async fn collections_auto_disables_auto_tagged_mod_outside_current_corridor() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let game_id = "game-corridor-cleanup".to_string();
    let object_id = "obj-ainoz".to_string();
    let mod_id = "mod-ainoz-nsfw".to_string();
    let rel_path = "Ainoz/NSFW Aino Bikini Nude";
    let mod_dir = tmp.path().join("Ainoz").join("NSFW Aino Bikini Nude");

    fs::create_dir_all(&mod_dir).expect("create auto-tagged mod dir");
    fs::write(
        mod_dir.join("mod.ini"),
        "[TextureOverrideMain]\nhash = abc123\n",
    )
    .expect("write auto-tagged mod ini");

    sqlx::query("INSERT INTO games (id, name, game_type, path, mod_path) VALUES (?, ?, ?, ?, ?)")
        .bind(&game_id)
        .bind("Genshin")
        .bind("GIMI")
        .bind(&mods_dir)
        .bind(&mods_dir)
        .execute(&pool)
        .await
        .expect("insert cleanup game");

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, folder_path, folder_path_key, sort_order, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(&object_id)
    .bind(&game_id)
    .bind("Ainoz")
    .bind("Other")
    .bind("Ainoz")
    .bind("ainoz")
    .bind(0)
    .execute(&pool)
    .await
    .expect("insert cleanup object");

    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, folder_path_key, status, object_id, is_safe, corridor_source) VALUES (?, ?, ?, ?, ?, 'ENABLED', ?, 0, 'auto_tagged')",
    )
    .bind(&mod_id)
    .bind(&game_id)
    .bind("NSFW Aino Bikini Nude")
    .bind(rel_path)
    .bind("ainoz/nsfw aino bikini nude")
    .bind(&object_id)
    .execute(&pool)
    .await
    .expect("insert auto-tagged enabled mod");

    let changed =
        auto_disable_auto_tagged_outside_corridor(&pool, &WatcherState::new(), &game_id, true)
            .await
            .expect("cleanup auto-tagged mismatch");

    assert_eq!(changed, 1);

    let status: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = ?")
        .bind(&mod_id)
        .fetch_one(&pool)
        .await
        .expect("cleanup status");
    let folder_path: String = sqlx::query_scalar("SELECT folder_path FROM mods WHERE id = ?")
        .bind(&mod_id)
        .fetch_one(&pool)
        .await
        .expect("cleanup folder path");

    assert_eq!(status, "DISABLED");
    assert!(
        folder_path.ends_with("Ainoz/DISABLED NSFW Aino Bikini Nude")
            || folder_path.ends_with("Ainoz\\DISABLED NSFW Aino Bikini Nude")
    );
}
