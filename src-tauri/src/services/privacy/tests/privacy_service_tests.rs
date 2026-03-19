use super::*;
use crate::database::corridor_state_repo;
use crate::database::game_repo::{upsert_game, GameRow};
use crate::services::collections::types::CollectionStateKind;
use crate::services::corridor_runtime::{get_corridor_runtime_snapshot, preview_corridor_switch};
use crate::services::corridor_types::CorridorPreviewStateKind;
use crate::test_utils::{
    insert_test_collection, insert_test_collection_item, insert_test_collection_object_state,
    insert_test_mod as insert_mod_fixture, insert_test_nested_collection_item, insert_test_object,
    TestCollectionFixture, TestCollectionItemFixture, TestCollectionObjectStateFixture,
    TestModFixture, TestNestedCollectionItemFixture, TestObjectFixture,
};
use std::path::Path;

async fn insert_test_game(pool: &sqlx::SqlitePool, mod_path: String) {
    let game = GameRow {
        id: "g1".into(),
        name: "Game 1".into(),
        game_type: "GIMI".into(),
        path: "C:\\Game1".into(),
        mod_path: Some(mod_path),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(pool, &game).await.unwrap();
}

async fn insert_test_mod(
    pool: &sqlx::SqlitePool,
    id: &str,
    name: &str,
    path: &str,
    status: &str,
    is_safe: bool,
) {
    insert_mod_fixture(
        pool,
        &TestModFixture {
            id,
            game_id: "g1",
            object_id: None,
            actual_name: name,
            folder_path: path,
            status,
            is_safe,
            object_type: Some("Other"),
            mods_path: None,
        },
    )
    .await
    .unwrap();
}

async fn insert_collection(
    pool: &sqlx::SqlitePool,
    id: &str,
    name: &str,
    is_safe: bool,
    is_last_unsaved: bool,
    items: &[(&str, &str)],
) {
    insert_test_collection(
        pool,
        &TestCollectionFixture {
            id,
            name,
            game_id: "g1",
            is_safe_context: is_safe,
            is_last_unsaved,
        },
    )
    .await
    .unwrap();

    for (mod_id, mod_path) in items {
        insert_test_collection_item(
            pool,
            &TestCollectionItemFixture {
                collection_id: id,
                mod_id,
                mod_path,
                mods_path: None,
            },
        )
        .await
        .unwrap();
    }
}

async fn insert_collection_object_state(
    pool: &sqlx::SqlitePool,
    collection_id: &str,
    object_id: &str,
    is_enabled: bool,
) {
    insert_test_collection_object_state(
        pool,
        &TestCollectionObjectStateFixture {
            collection_id,
            object_id,
            is_enabled,
        },
    )
    .await
    .unwrap();
}

async fn insert_nested_collection_item(
    pool: &sqlx::SqlitePool,
    collection_id: &str,
    mod_path: &str,
) {
    insert_test_nested_collection_item(
        pool,
        &TestNestedCollectionItemFixture {
            collection_id,
            mod_path,
            mods_path: None,
        },
    )
    .await
    .unwrap();
}

async fn insert_object(
    pool: &sqlx::SqlitePool,
    id: &str,
    name: &str,
    folder_path: &str,
    object_type: &str,
) {
    insert_test_object(
        pool,
        &TestObjectFixture {
            id,
            game_id: "g1",
            name,
            folder_path: Some(folder_path),
            object_type,
        },
    )
    .await
    .unwrap();
}

async fn link_mod_to_object(pool: &sqlx::SqlitePool, mod_id: &str, object_id: &str) {
    sqlx::query("UPDATE mods SET object_id = ? WHERE id = ?")
        .bind(object_id)
        .bind(mod_id)
        .execute(pool)
        .await
        .unwrap();
}

fn write_info_json(path: &Path, is_safe: bool) {
    let file = if is_safe {
        "{\n  \"actual_name\": \"Test Mod\",\n  \"is_safe\": true\n}\n"
    } else {
        "{\n  \"actual_name\": \"Test Mod\",\n  \"is_safe\": false\n}\n"
    };
    std::fs::write(path.join("info.json"), file).unwrap();
}

fn create_flat_mod_root(mods_root: &Path, relative_path: &str, is_safe: bool) {
    let dir = mods_root.join(relative_path);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("mod.ini"),
        "[TextureOverrideMain]\nhash = abc123\n",
    )
    .unwrap();
    write_info_json(&dir, is_safe);
}

fn create_mod_pack_root(mods_root: &Path, relative_path: &str, is_safe: bool) {
    let dir = mods_root.join(relative_path);
    std::fs::create_dir_all(dir.join("Assets")).unwrap();
    std::fs::create_dir_all(dir.join("Variants")).unwrap();
    std::fs::write(
        dir.join("mod.ini"),
        "[TextureOverrideMain]\nhash = abc123\n[ResourceBody]\nfilename = Assets/body.dds\n",
    )
    .unwrap();
    write_info_json(&dir, is_safe);
}

fn create_variant_container(mods_root: &Path, relative_path: &str, is_safe: bool) {
    let dir = mods_root.join(relative_path);
    std::fs::create_dir_all(&dir).unwrap();
    for variant in ["VariantA", "VariantB"] {
        let child = dir.join(variant);
        std::fs::create_dir_all(&child).unwrap();
        std::fs::write(
            child.join("mod.ini"),
            "[TextureOverrideVariant]\nhash = def456\n",
        )
        .unwrap();
        write_info_json(&child, is_safe);
    }
    std::fs::write(
        dir.join("orchestrator.ini"),
        "[TextureOverrideMain]\nhash = abc123\n[ResourceA]\nfilename = VariantA/file.dds\n[ResourceB]\nfilename = VariantB/file.dds\n",
    )
    .unwrap();
    write_info_json(&dir, is_safe);
}

fn setup_mods_root() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[tokio::test]
async fn test_preview_corridor_switch_uses_named_collection_labels() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "SafeObject/SafeMod", true);
    create_flat_mod_root(temp.path(), "UnsafeObject/UnsafeMod", false);
    insert_test_mod(
        pool,
        "safe_mod",
        "Safe Mod",
        "SafeObject/SafeMod",
        "ENABLED",
        true,
    )
    .await;
    insert_test_mod(
        pool,
        "unsafe_mod",
        "Unsafe Mod",
        "UnsafeObject/UnsafeMod",
        "DISABLED",
        false,
    )
    .await;
    insert_collection(
        pool,
        "safe_collection",
        "My Save",
        true,
        false,
        &[("safe_mod", "SafeObject/SafeMod")],
    )
    .await;
    insert_collection(
        pool,
        "unsafe_collection",
        "My Unsafe",
        false,
        false,
        &[("unsafe_mod", "UnsafeObject/UnsafeMod")],
    )
    .await;

    corridor_state_repo::upsert_corridor_state(pool, "g1", true, Some("safe_collection"), None)
        .await
        .unwrap();
    corridor_state_repo::upsert_corridor_state(pool, "g1", false, Some("unsafe_collection"), None)
        .await
        .unwrap();

    let preview = preview_corridor_switch(pool, "g1", true, false)
        .await
        .unwrap();

    assert_eq!(preview.leaving_state_name, "My Save");
    assert_eq!(preview.leaving_state_kind, CorridorPreviewStateKind::Named);
    assert_eq!(preview.target_state_name.as_deref(), Some("My Unsafe"));
    assert_eq!(preview.target_state_kind, CorridorPreviewStateKind::Named);
    assert_eq!(preview.leaving_mods.len(), 1);
    assert_eq!(preview.target_mods.len(), 1);
    assert_eq!(preview.target_mods[0].actual_name, "UnsafeMod");
    assert_eq!(preview.target_mods[0].node_type, "FlatModRoot");
}

#[tokio::test]
async fn test_preview_corridor_switch_leaving_state_matches_named_runtime_snapshot() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "SafeObject/SafeMod", true);
    create_flat_mod_root(temp.path(), "UnsafeObject/UnsafeMod", false);
    insert_test_mod(
        pool,
        "safe_mod",
        "Safe Mod",
        "SafeObject/SafeMod",
        "ENABLED",
        true,
    )
    .await;
    insert_test_mod(
        pool,
        "unsafe_mod",
        "Unsafe Mod",
        "UnsafeObject/UnsafeMod",
        "DISABLED",
        false,
    )
    .await;
    insert_collection(
        pool,
        "safe_collection",
        "My New Collection",
        true,
        false,
        &[("safe_mod", "SafeObject/SafeMod")],
    )
    .await;
    insert_collection(
        pool,
        "unsafe_collection",
        "My Unsafe",
        false,
        false,
        &[("unsafe_mod", "UnsafeObject/UnsafeMod")],
    )
    .await;

    corridor_state_repo::upsert_corridor_state(pool, "g1", true, Some("safe_collection"), None)
        .await
        .unwrap();
    corridor_state_repo::upsert_corridor_state(pool, "g1", false, Some("unsafe_collection"), None)
        .await
        .unwrap();

    let runtime_snapshot = get_corridor_runtime_snapshot(pool, "g1", true)
        .await
        .unwrap();
    let preview = preview_corridor_switch(pool, "g1", true, false)
        .await
        .unwrap();

    assert_eq!(runtime_snapshot.state_kind, CollectionStateKind::Named);
    assert_eq!(runtime_snapshot.state_name.as_deref(), Some("My New Collection"));
    assert_eq!(preview.leaving_state_kind, CorridorPreviewStateKind::Named);
    assert_eq!(preview.leaving_state_name, "My New Collection");
    assert_eq!(preview.leaving_mods.len(), runtime_snapshot.roots.len());
    assert_eq!(
        preview
            .leaving_mods
            .iter()
            .map(|item| item.folder_path.as_str())
            .collect::<Vec<_>>(),
        runtime_snapshot
            .roots
            .iter()
            .map(|item| item.folder_path.as_str())
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn test_preview_corridor_switch_uses_unicode_collection_labels_without_leaking_disabled_paths(
) {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "안전/日本語모드", true);
    create_flat_mod_root(temp.path(), "위험/中文모드", false);
    create_flat_mod_root(temp.path(), "위험/DISABLED 한글숨김", false);

    insert_test_mod(
        pool,
        "safe_unicode_mod",
        "日本語모드",
        "안전/日本語모드",
        "ENABLED",
        true,
    )
    .await;
    insert_test_mod(
        pool,
        "unsafe_unicode_mod",
        "中文모드",
        "위험/中文모드",
        "DISABLED",
        false,
    )
    .await;

    insert_collection(
        pool,
        "safe_unicode_collection",
        "내 세이브",
        true,
        false,
        &[("safe_unicode_mod", "안전/日本語모드")],
    )
    .await;
    insert_collection(
        pool,
        "unsafe_unicode_collection",
        "내 언세이프",
        false,
        false,
        &[("unsafe_unicode_mod", "위험/中文모드")],
    )
    .await;
    insert_nested_collection_item(
        pool,
        "unsafe_unicode_collection",
        &temp
            .path()
            .join("위험")
            .join("DISABLED 한글숨김")
            .to_string_lossy(),
    )
    .await;

    corridor_state_repo::upsert_corridor_state(
        pool,
        "g1",
        true,
        Some("safe_unicode_collection"),
        None,
    )
    .await
    .unwrap();
    corridor_state_repo::upsert_corridor_state(
        pool,
        "g1",
        false,
        Some("unsafe_unicode_collection"),
        None,
    )
    .await
    .unwrap();

    let preview = preview_corridor_switch(pool, "g1", true, false)
        .await
        .unwrap();

    assert_eq!(preview.leaving_state_name, "내 세이브");
    assert_eq!(preview.target_state_name.as_deref(), Some("내 언세이프"));
    assert_eq!(preview.target_mods.len(), 1);
    assert_eq!(preview.target_mods[0].actual_name, "中文모드");
}

#[tokio::test]
async fn test_preview_corridor_switch_shows_target_mods_that_are_currently_disabled_on_disk() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "SafeObject/SafeMod", true);
    create_flat_mod_root(temp.path(), "UnsafeObject/DISABLED UnsafePreview", false);
    insert_test_mod(
        pool,
        "safe_mod",
        "Safe Mod",
        "SafeObject/SafeMod",
        "ENABLED",
        true,
    )
    .await;
    insert_test_mod(
        pool,
        "unsafe_disabled_mod",
        "Unsafe Preview",
        "UnsafeObject/DISABLED UnsafePreview",
        "DISABLED",
        false,
    )
    .await;
    insert_collection(
        pool,
        "safe_collection",
        "My Save",
        true,
        false,
        &[("safe_mod", "SafeObject/SafeMod")],
    )
    .await;
    insert_collection(
        pool,
        "unsafe_collection",
        "My Unsafe",
        false,
        false,
        &[("unsafe_disabled_mod", "UnsafeObject/DISABLED UnsafePreview")],
    )
    .await;

    corridor_state_repo::upsert_corridor_state(pool, "g1", true, Some("safe_collection"), None)
        .await
        .unwrap();
    corridor_state_repo::upsert_corridor_state(pool, "g1", false, Some("unsafe_collection"), None)
        .await
        .unwrap();

    let preview = preview_corridor_switch(pool, "g1", true, false)
        .await
        .unwrap();

    assert_eq!(preview.target_state_name.as_deref(), Some("My Unsafe"));
    assert_eq!(preview.target_mods.len(), 1);
    assert_eq!(preview.target_mods[0].actual_name, "UnsafePreview");
    assert_eq!(preview.target_mods[0].node_type, "FlatModRoot");
}

#[tokio::test]
async fn test_preview_corridor_switch_projects_nested_paths_to_main_root_and_dedupes() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "SafeObject/SafeMod", true);
    create_mod_pack_root(temp.path(), "UnsafeObject/MainPack", false);
    insert_test_mod(
        pool,
        "safe_mod",
        "Safe Mod",
        "SafeObject/SafeMod",
        "ENABLED",
        true,
    )
    .await;
    insert_collection(
        pool,
        "safe_collection",
        "My Save",
        true,
        false,
        &[("safe_mod", "SafeObject/SafeMod")],
    )
    .await;
    insert_collection(pool, "unsafe_collection", "My Unsafe", false, false, &[]).await;
    insert_nested_collection_item(pool, "unsafe_collection", "UnsafeObject/MainPack/Assets").await;
    insert_nested_collection_item(pool, "unsafe_collection", "UnsafeObject/MainPack/Variants")
        .await;

    corridor_state_repo::upsert_corridor_state(pool, "g1", true, Some("safe_collection"), None)
        .await
        .unwrap();
    corridor_state_repo::upsert_corridor_state(pool, "g1", false, Some("unsafe_collection"), None)
        .await
        .unwrap();

    let preview = preview_corridor_switch(pool, "g1", true, false)
        .await
        .unwrap();

    assert_eq!(preview.target_mods.len(), 1);
    assert_eq!(preview.target_mods[0].actual_name, "MainPack");
    assert_eq!(preview.target_mods[0].node_type, "ModPackRoot");
    assert!(
        preview.target_mods[0]
            .folder_path
            .ends_with("UnsafeObject\\MainPack")
            || preview.target_mods[0]
                .folder_path
                .ends_with("UnsafeObject/MainPack")
    );
}

#[tokio::test]
async fn test_preview_corridor_switch_projects_variant_children_to_variant_container_root() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "SafeObject/SafeMod", true);
    create_variant_container(temp.path(), "UnsafeObject/VariantPack", false);
    insert_test_mod(
        pool,
        "safe_mod",
        "Safe Mod",
        "SafeObject/SafeMod",
        "ENABLED",
        true,
    )
    .await;
    insert_collection(
        pool,
        "safe_collection",
        "My Save",
        true,
        false,
        &[("safe_mod", "SafeObject/SafeMod")],
    )
    .await;
    insert_collection(pool, "unsafe_collection", "My Unsafe", false, false, &[]).await;
    insert_nested_collection_item(
        pool,
        "unsafe_collection",
        "UnsafeObject/VariantPack/VariantA",
    )
    .await;

    corridor_state_repo::upsert_corridor_state(pool, "g1", true, Some("safe_collection"), None)
        .await
        .unwrap();
    corridor_state_repo::upsert_corridor_state(pool, "g1", false, Some("unsafe_collection"), None)
        .await
        .unwrap();

    let preview = preview_corridor_switch(pool, "g1", true, false)
        .await
        .unwrap();

    assert_eq!(preview.target_mods.len(), 1);
    assert_eq!(preview.target_mods[0].actual_name, "VariantPack");
    assert_eq!(preview.target_mods[0].node_type, "VariantContainer");
}

#[tokio::test]
async fn test_preview_corridor_switch_uses_unsaved_target_snapshot() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "SafeObject/SafeMod", true);
    create_flat_mod_root(temp.path(), "UnsafeObject/UnsafeMod", false);
    insert_test_mod(
        pool,
        "safe_mod",
        "Safe Mod",
        "SafeObject/SafeMod",
        "ENABLED",
        true,
    )
    .await;
    insert_test_mod(
        pool,
        "unsafe_mod",
        "Unsafe Mod",
        "UnsafeObject/UnsafeMod",
        "DISABLED",
        false,
    )
    .await;
    insert_collection(
        pool,
        "safe_collection",
        "My Save",
        true,
        false,
        &[("safe_mod", "SafeObject/SafeMod")],
    )
    .await;
    insert_collection(
        pool,
        "unsafe_snapshot",
        "Unsaved",
        false,
        true,
        &[("unsafe_mod", "UnsafeObject/UnsafeMod")],
    )
    .await;

    corridor_state_repo::upsert_corridor_state(pool, "g1", true, Some("safe_collection"), None)
        .await
        .unwrap();
    corridor_state_repo::upsert_corridor_state(pool, "g1", false, None, Some("unsafe_snapshot"))
        .await
        .unwrap();

    let preview = preview_corridor_switch(pool, "g1", true, false)
        .await
        .unwrap();

    assert_eq!(preview.target_state_name.as_deref(), Some("Unsaved Preset"));
    assert_eq!(preview.target_state_kind, CorridorPreviewStateKind::Unsaved);
    assert_eq!(preview.target_mods.len(), 1);
    assert_eq!(preview.target_mods[0].actual_name, "UnsafeMod");
}

#[tokio::test]
async fn test_preview_corridor_switch_filters_disabled_object_members_from_target_collection() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "SafeObject/SafeMod", true);
    create_flat_mod_root(temp.path(), "UnsafeObject/EnabledUnsafe", false);
    create_flat_mod_root(temp.path(), "UnsafeObject/DisabledUnsafe", false);
    insert_test_mod(
        pool,
        "safe_mod",
        "Safe Mod",
        "SafeObject/SafeMod",
        "ENABLED",
        true,
    )
    .await;
    insert_test_mod(
        pool,
        "unsafe_enabled_mod",
        "Unsafe Enabled Mod",
        "UnsafeObject/EnabledUnsafe",
        "DISABLED",
        false,
    )
    .await;
    insert_test_mod(
        pool,
        "unsafe_disabled_mod",
        "Unsafe Disabled Mod",
        "UnsafeObject/DisabledUnsafe",
        "DISABLED",
        false,
    )
    .await;
    insert_object(pool, "obj-safe", "SafeObject", "SafeObject", "Character").await;
    insert_object(
        pool,
        "obj-unsafe-on",
        "UnsafeObject",
        "UnsafeObject",
        "Character",
    )
    .await;
    insert_object(
        pool,
        "obj-unsafe-off",
        "DisabledUnsafeObject",
        "DisabledUnsafeObject",
        "Character",
    )
    .await;
    link_mod_to_object(pool, "safe_mod", "obj-safe").await;
    link_mod_to_object(pool, "unsafe_enabled_mod", "obj-unsafe-on").await;
    link_mod_to_object(pool, "unsafe_disabled_mod", "obj-unsafe-off").await;
    insert_collection(
        pool,
        "safe_collection",
        "My Save",
        true,
        false,
        &[("safe_mod", "SafeObject/SafeMod")],
    )
    .await;
    insert_collection(
        pool,
        "unsafe_collection",
        "My Unsafe",
        false,
        false,
        &[
            ("unsafe_enabled_mod", "UnsafeObject/EnabledUnsafe"),
            ("unsafe_disabled_mod", "UnsafeObject/DisabledUnsafe"),
        ],
    )
    .await;
    insert_collection_object_state(pool, "unsafe_collection", "obj-unsafe-on", true).await;
    insert_collection_object_state(pool, "unsafe_collection", "obj-unsafe-off", false).await;

    corridor_state_repo::upsert_corridor_state(pool, "g1", true, Some("safe_collection"), None)
        .await
        .unwrap();
    corridor_state_repo::upsert_corridor_state(pool, "g1", false, Some("unsafe_collection"), None)
        .await
        .unwrap();

    let preview = preview_corridor_switch(pool, "g1", true, false)
        .await
        .unwrap();

    assert_eq!(preview.target_state_name.as_deref(), Some("My Unsafe"));
    assert_eq!(preview.target_mods.len(), 1);
    assert_eq!(preview.target_mods[0].actual_name, "EnabledUnsafe");
}

#[tokio::test]
async fn test_preview_corridor_switch_filters_disabled_nested_paths_from_target_collection() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "SafeObject/SafeMod", true);
    create_flat_mod_root(temp.path(), "UnsafeObject/VisibleUnsafe", false);
    create_flat_mod_root(temp.path(), "UnsafeObject/DISABLED HiddenUnsafe", false);
    insert_test_mod(
        pool,
        "safe_mod",
        "Safe Mod",
        "SafeObject/SafeMod",
        "ENABLED",
        true,
    )
    .await;
    insert_collection(
        pool,
        "safe_collection",
        "My Save",
        true,
        false,
        &[("safe_mod", "SafeObject/SafeMod")],
    )
    .await;
    insert_collection(pool, "unsafe_collection", "My Unsafe", false, false, &[]).await;
    insert_nested_collection_item(pool, "unsafe_collection", "UnsafeObject/VisibleUnsafe").await;
    insert_nested_collection_item(
        pool,
        "unsafe_collection",
        "UnsafeObject/DISABLED HiddenUnsafe",
    )
    .await;

    corridor_state_repo::upsert_corridor_state(pool, "g1", true, Some("safe_collection"), None)
        .await
        .unwrap();
    corridor_state_repo::upsert_corridor_state(pool, "g1", false, Some("unsafe_collection"), None)
        .await
        .unwrap();

    let preview = preview_corridor_switch(pool, "g1", true, false)
        .await
        .unwrap();

    assert_eq!(preview.target_state_name.as_deref(), Some("My Unsafe"));
    assert_eq!(preview.target_mods.len(), 1);
    assert_eq!(preview.target_mods[0].actual_name, "VisibleUnsafe");
}

#[tokio::test]
async fn test_preview_corridor_switch_keeps_named_state_when_all_members_are_filtered_out() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "SafeObject/SafeMod", true);
    create_flat_mod_root(temp.path(), "UnsafeObject/DisabledUnsafe", false);
    insert_test_mod(
        pool,
        "safe_mod",
        "Safe Mod",
        "SafeObject/SafeMod",
        "ENABLED",
        true,
    )
    .await;
    insert_test_mod(
        pool,
        "unsafe_disabled_mod",
        "Unsafe Disabled Mod",
        "UnsafeObject/DisabledUnsafe",
        "DISABLED",
        false,
    )
    .await;
    insert_object(
        pool,
        "obj-unsafe-off",
        "DisabledUnsafeObject",
        "DisabledUnsafeObject",
        "Character",
    )
    .await;
    link_mod_to_object(pool, "unsafe_disabled_mod", "obj-unsafe-off").await;
    insert_collection(
        pool,
        "safe_collection",
        "My Save",
        true,
        false,
        &[("safe_mod", "SafeObject/SafeMod")],
    )
    .await;
    insert_collection(
        pool,
        "unsafe_collection",
        "My Unsafe",
        false,
        false,
        &[("unsafe_disabled_mod", "UnsafeObject/DisabledUnsafe")],
    )
    .await;
    insert_collection_object_state(pool, "unsafe_collection", "obj-unsafe-off", false).await;

    corridor_state_repo::upsert_corridor_state(pool, "g1", true, Some("safe_collection"), None)
        .await
        .unwrap();
    corridor_state_repo::upsert_corridor_state(pool, "g1", false, Some("unsafe_collection"), None)
        .await
        .unwrap();

    let preview = preview_corridor_switch(pool, "g1", true, false)
        .await
        .unwrap();

    assert_eq!(preview.target_state_name.as_deref(), Some("My Unsafe"));
    assert_eq!(preview.target_state_kind, CorridorPreviewStateKind::Named);
    assert!(preview.target_mods.is_empty());
    assert_eq!(preview.target_description, "My Unsafe (All Disabled)");
}

#[tokio::test]
async fn test_preview_corridor_switch_reports_true_empty_target_state() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "UnsafeObject/UnsafeMod", false);
    insert_test_mod(
        pool,
        "unsafe_mod",
        "Unsafe Mod",
        "UnsafeObject/UnsafeMod",
        "ENABLED",
        false,
    )
    .await;

    let preview = preview_corridor_switch(pool, "g1", false, true)
        .await
        .unwrap();

    assert_eq!(preview.leaving_state_name, "Unsaved Preset");
    assert_eq!(
        preview.leaving_state_kind,
        CorridorPreviewStateKind::Unsaved
    );
    assert_eq!(preview.target_state_name, None);
    assert_eq!(preview.target_state_kind, CorridorPreviewStateKind::None);
    assert!(preview.target_mods.is_empty());
    assert_eq!(preview.target_description, "All Disabled");
}

#[tokio::test]
async fn test_preview_corridor_switch_leaving_state_matches_unsaved_runtime_snapshot() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "UnsafeObject/UnsafeMod", false);
    insert_test_mod(
        pool,
        "unsafe_mod",
        "Unsafe Mod",
        "UnsafeObject/UnsafeMod",
        "ENABLED",
        false,
    )
    .await;

    let runtime_snapshot = get_corridor_runtime_snapshot(pool, "g1", false)
        .await
        .unwrap();
    let preview = preview_corridor_switch(pool, "g1", false, true)
        .await
        .unwrap();

    assert_eq!(runtime_snapshot.state_kind, CollectionStateKind::Unsaved);
    assert_eq!(preview.leaving_state_kind, CorridorPreviewStateKind::Unsaved);
    assert_eq!(
        preview.leaving_state_name,
        runtime_snapshot
            .state_name
            .unwrap_or_else(|| "Unsaved Preset".to_string())
    );
    assert_eq!(preview.leaving_mods.len(), runtime_snapshot.roots.len());
}

#[tokio::test]
async fn test_preview_corridor_switch_preserves_empty_named_collection_label() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    create_flat_mod_root(temp.path(), "UnsafeObject/UnsafeMod", false);
    insert_test_mod(
        pool,
        "unsafe_mod",
        "Unsafe Mod",
        "UnsafeObject/UnsafeMod",
        "ENABLED",
        false,
    )
    .await;
    insert_collection(pool, "safe_empty", "My Save", true, false, &[]).await;

    corridor_state_repo::upsert_corridor_state(pool, "g1", true, Some("safe_empty"), None)
        .await
        .unwrap();

    let preview = preview_corridor_switch(pool, "g1", false, true)
        .await
        .unwrap();

    assert_eq!(preview.target_state_name.as_deref(), Some("My Save"));
    assert_eq!(preview.target_state_kind, CorridorPreviewStateKind::Named);
    assert!(preview.target_mods.is_empty());
    assert_eq!(preview.target_description, "My Save (All Disabled)");
}

#[tokio::test]
async fn test_switch_mode_preserves_depth_1() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    insert_test_mod(pool, "obj1", "Object 1", "Obj1", "ENABLED", true).await;
    insert_test_mod(pool, "mod1", "Mod 1", "Obj1/Mod1", "ENABLED", true).await;
    std::fs::create_dir_all(temp.path().join("Obj1")).unwrap();
    std::fs::create_dir_all(temp.path().join("Obj1/Mod1")).unwrap();

    let watcher = WatcherState::default();
    switch_mode(Mode::NSFW, pool, &watcher, "g1").await.unwrap();

    let obj_status: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = 'obj1'")
        .fetch_one(pool)
        .await
        .unwrap();
    let mod_status: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = 'mod1'")
        .fetch_one(pool)
        .await
        .unwrap();

    assert_eq!(obj_status, "ENABLED");
    assert_eq!(mod_status, "DISABLED");
}

#[tokio::test]
async fn test_switch_mode_only_disables_leaving_corridor() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    insert_test_mod(
        pool,
        "safe_mod",
        "Safe Mod",
        "SafeObject/SafeMod",
        "ENABLED",
        true,
    )
    .await;
    insert_test_mod(
        pool,
        "unsafe_mod",
        "Unsafe Mod",
        "UnsafeObject/UnsafeMod",
        "ENABLED",
        false,
    )
    .await;
    std::fs::create_dir_all(temp.path().join("SafeObject/SafeMod")).unwrap();
    std::fs::create_dir_all(temp.path().join("UnsafeObject/UnsafeMod")).unwrap();

    let watcher = WatcherState::default();
    let result = switch_mode(Mode::NSFW, pool, &watcher, "g1").await.unwrap();

    let safe_status: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = 'safe_mod'")
        .fetch_one(pool)
        .await
        .unwrap();
    let unsafe_status: String =
        sqlx::query_scalar("SELECT status FROM mods WHERE id = 'unsafe_mod'")
            .fetch_one(pool)
            .await
            .unwrap();

    assert_eq!(result.disabled_count, 1);
    assert_eq!(result.restored_count, 0);
    assert_eq!(safe_status, "DISABLED");
    assert_eq!(unsafe_status, "ENABLED");
}

#[tokio::test]
async fn test_switch_mode_preserves_last_selected_collection_per_corridor() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;
    let temp = setup_mods_root();

    insert_test_game(pool, temp.path().to_string_lossy().to_string()).await;
    insert_test_mod(
        pool,
        "safe_mod",
        "Safe Mod",
        "SafeObject/SafeMod",
        "ENABLED",
        true,
    )
    .await;
    insert_test_mod(
        pool,
        "unsafe_mod",
        "Unsafe Mod",
        "UnsafeObject/UnsafeMod",
        "ENABLED",
        false,
    )
    .await;
    std::fs::create_dir_all(temp.path().join("SafeObject/SafeMod")).unwrap();
    std::fs::create_dir_all(temp.path().join("UnsafeObject/UnsafeMod")).unwrap();
    insert_collection(
        pool,
        "safe_collection",
        "My Save",
        true,
        false,
        &[("safe_mod", "SafeObject/SafeMod")],
    )
    .await;
    insert_collection(
        pool,
        "unsafe_collection",
        "My Unsafe",
        false,
        false,
        &[("unsafe_mod", "UnsafeObject/UnsafeMod")],
    )
    .await;

    corridor_state_repo::upsert_corridor_state(pool, "g1", true, Some("safe_collection"), None)
        .await
        .unwrap();

    let watcher = WatcherState::default();
    switch_mode(Mode::NSFW, pool, &watcher, "g1").await.unwrap();
    corridor_state_repo::upsert_corridor_state(pool, "g1", false, Some("unsafe_collection"), None)
        .await
        .unwrap();
    switch_mode(Mode::SFW, pool, &watcher, "g1").await.unwrap();

    let safe_state = corridor_state_repo::get_corridor_state(pool, "g1", true)
        .await
        .unwrap();
    let unsafe_state = corridor_state_repo::get_corridor_state(pool, "g1", false)
        .await
        .unwrap();

    assert_eq!(
        safe_state.active_collection_id.as_deref(),
        Some("safe_collection")
    );
    assert_eq!(
        unsafe_state.active_collection_id.as_deref(),
        Some("unsafe_collection")
    );
}
