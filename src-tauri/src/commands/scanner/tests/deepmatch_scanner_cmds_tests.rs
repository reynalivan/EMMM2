use super::*;
use crate::database::models::{GameType, ItemStatus};
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_mod, insert_test_object, TestGameFixture,
    TestModFixture, TestObjectFixture,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

async fn setup_object_preview_fixture(
    mods_root: &Path,
    rows: &[(&str, &str, &str, &Path)],
) -> sqlx::SqlitePool {
    let pool = init_test_db().await.pool;
    let mods_root_string = mods_root.to_string_lossy().to_string();

    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_preview_objects",
            name: "Genshin",
            game_type: GameType::GIMI,
            path: mods_root_string.as_str(),
            mods_path: Some(mods_root_string.as_str()),
        },
    )
    .await
    .expect("insert game");

    for (object_id, object_name) in [("obj1", "Hu Tao"), ("obj2", "Kazuha")] {
        insert_test_object(
            &pool,
            &TestObjectFixture {
                id: object_id,
                game_id: "g_preview_objects",
                name: object_name,
                folder_path: object_name,
                object_type: "Character",
            },
        )
        .await
        .expect("insert object");
    }

    for (mod_id, object_id, actual_name, folder_path) in rows {
        let folder_path_string = folder_path.to_string_lossy().to_string();
        insert_test_mod(
            &pool,
            &TestModFixture {
                id: mod_id,
                game_id: "g_preview_objects",
                object_id: Some(object_id),
                actual_name,
                folder_path: folder_path_string.as_str(),
                status: ItemStatus::Disabled,
                is_safe: true,
                object_type: Some("Character"),
                mods_path: Some(mods_root_string.as_str()),
            },
        )
        .await
        .expect("insert mod");
    }

    pool
}

fn paths_to_strings(paths: Vec<PathBuf>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

#[tokio::test]
async fn score_candidates_batch_cmd_returns_scores_for_candidates() {
    let dir = TempDir::new().expect("tempdir");
    let mod_dir = dir.path().join("Kazuha");
    fs::create_dir(&mod_dir).expect("create mod dir");
    fs::write(
        mod_dir.join("mod.ini"),
        "[TextureOverrideKazuha]\nhash = 12345678\n",
    )
    .expect("write mod ini");

    let db_json = json!([
        {
            "name": "Kazuha",
            "tags": [],
            "object_type": "Character",
            "custom_skins": [],
            "thumbnail_path": null,
            "metadata": null,
            "hash_db": {}
        },
        {
            "name": "Diluc",
            "tags": [],
            "object_type": "Character",
            "custom_skins": [],
            "thumbnail_path": null,
            "metadata": null,
            "hash_db": {}
        }
    ])
    .to_string();

    let scores = score_candidates_batch_cmd(
        mod_dir.to_string_lossy().to_string(),
        vec!["Kazuha".to_string(), "Diluc".to_string()],
        db_json,
    )
    .await
    .expect("scores");

    assert!(scores.contains_key("Kazuha"));
    assert!(scores.contains_key("Diluc"));
}

#[tokio::test]
async fn resolve_object_preview_paths_returns_existing_paths_for_object_ids() {
    let temp = TempDir::new().expect("tempdir");
    let mods_root = temp.path().join("Mods");
    let hu_tao = mods_root.join("Hu Tao").join("Skin1");
    let kazuha = mods_root.join("Kazuha").join("Skin1");
    fs::create_dir_all(&hu_tao).expect("create hu tao mod");
    fs::create_dir_all(&kazuha).expect("create kazuha mod");

    let pool = setup_object_preview_fixture(
        &mods_root,
        &[
            ("m1", "obj1", "Hu Tao Skin", hu_tao.as_path()),
            ("m2", "obj2", "Kazuha Skin", kazuha.as_path()),
        ],
    )
    .await;

    let resolved = resolve_object_preview_paths(
        &pool,
        "g_preview_objects",
        &mods_root,
        &["obj1".to_string(), "obj2".to_string()],
    )
    .await
    .expect("resolve paths");

    assert_eq!(
        paths_to_strings(resolved),
        vec![
            hu_tao
                .canonicalize()
                .expect("canonical hu tao")
                .to_string_lossy()
                .to_string(),
            kazuha
                .canonicalize()
                .expect("canonical kazuha")
                .to_string_lossy()
                .to_string(),
        ]
    );
}

#[tokio::test]
async fn resolve_object_preview_paths_returns_empty_for_empty_object_ids() {
    let temp = TempDir::new().expect("tempdir");
    let mods_root = temp.path().join("Mods");
    fs::create_dir_all(&mods_root).expect("create mods root");
    let pool = setup_object_preview_fixture(&mods_root, &[]).await;

    let resolved = resolve_object_preview_paths(&pool, "g_preview_objects", &mods_root, &[])
        .await
        .expect("resolve paths");

    assert!(resolved.is_empty());
}

#[tokio::test]
async fn resolve_object_preview_paths_skips_missing_database_paths() {
    let temp = TempDir::new().expect("tempdir");
    let mods_root = temp.path().join("Mods");
    let existing = mods_root.join("Hu Tao").join("Skin1");
    let missing = mods_root.join("Hu Tao").join("Missing");
    fs::create_dir_all(&existing).expect("create existing mod");

    let pool = setup_object_preview_fixture(
        &mods_root,
        &[
            ("m1", "obj1", "Existing Skin", existing.as_path()),
            ("m2", "obj1", "Missing Skin", missing.as_path()),
        ],
    )
    .await;

    let resolved = resolve_object_preview_paths(
        &pool,
        "g_preview_objects",
        &mods_root,
        &["obj1".to_string()],
    )
    .await
    .expect("resolve paths");

    assert_eq!(
        paths_to_strings(resolved),
        vec![existing
            .canonicalize()
            .expect("canonical existing")
            .to_string_lossy()
            .to_string()]
    );
}

#[tokio::test]
async fn resolve_object_preview_paths_skips_paths_outside_mods_root() {
    let temp = TempDir::new().expect("tempdir");
    let mods_root = temp.path().join("Mods");
    let outside_root = temp.path().join("Outside");
    let outside_mod = outside_root.join("Escaped");
    fs::create_dir_all(&mods_root).expect("create mods root");
    fs::create_dir_all(&outside_mod).expect("create outside mod");

    let pool = setup_object_preview_fixture(
        &mods_root,
        &[("m1", "obj1", "Escaped", outside_mod.as_path())],
    )
    .await;

    let resolved = resolve_object_preview_paths(
        &pool,
        "g_preview_objects",
        &mods_root,
        &["obj1".to_string()],
    )
    .await
    .expect("resolve paths");

    assert!(resolved.is_empty());
}
