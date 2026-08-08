//! `mods.folder_path` holds one thing: a path relative to the game's mods root.
//!
//! It used to hold two, depending on which writer touched the row last, and
//! every reader had to guess. Conflict detection guessed wrong and reported no
//! conflicts for years' worth of rows without ever reading a file.

use crate::common::path_key::folder_path_key;
use crate::domain::models::GameType;
use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};
use sqlx::SqlitePool;

const ROOT: &str = "C:\\Games\\Genshin\\Mods";

async fn pool_with_game() -> SqlitePool {
    let pool = init_test_db().await.pool;
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: GameType::GIMI,
            path: "C:\\Games\\Genshin",
            mods_path: Some(ROOT),
        },
    )
    .await
    .expect("game");
    pool
}

/// Insert straight through SQL: the point is to plant rows in the *old* shape,
/// which the current code no longer produces.
async fn plant(pool: &SqlitePool, id: &str, folder_path: &str) {
    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, folder_path_key, status)
         VALUES (?, 'g1', ?, ?, ?, 1)",
    )
    .bind(id)
    .bind(id)
    .bind(folder_path)
    .bind(folder_path_key(folder_path, Some(ROOT)))
    .execute(pool)
    .await
    .expect("plant row");
}

async fn stored_path(pool: &SqlitePool, id: &str) -> String {
    sqlx::query_scalar("SELECT folder_path FROM mods WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("row")
}

/// The migration ships in `migrations/`, so a fresh test pool has already run
/// it. Re-running the statement is how a test exercises it against rows the
/// migration could not have seen.
async fn run_normalization(pool: &SqlitePool) {
    let sql = include_str!("../../../migrations/20260815000000_normalize_mod_folder_paths.sql");
    sqlx::query(sql).execute(pool).await.expect("normalize");
}

#[tokio::test]
async fn an_absolute_path_is_rewritten_relative_to_the_mods_root() {
    let pool = pool_with_game().await;
    plant(&pool, "m1", &format!("{ROOT}\\Amber\\Blue Dress")).await;

    run_normalization(&pool).await;

    assert_eq!(stored_path(&pool, "m1").await, "Amber\\Blue Dress");
}

#[tokio::test]
async fn a_path_that_is_already_relative_is_left_alone() {
    let pool = pool_with_game().await;
    plant(&pool, "m1", "Amber\\Blue Dress").await;

    run_normalization(&pool).await;

    assert_eq!(stored_path(&pool, "m1").await, "Amber\\Blue Dress");
}

#[tokio::test]
async fn the_root_prefix_is_matched_case_insensitively() {
    let pool = pool_with_game().await;
    // Windows hands back whatever case the caller used.
    plant(&pool, "m1", "c:\\games\\genshin\\mods\\Klee\\Red").await;

    run_normalization(&pool).await;

    assert_eq!(stored_path(&pool, "m1").await, "Klee\\Red");
}

#[tokio::test]
async fn a_path_outside_the_mods_root_is_not_touched() {
    let pool = pool_with_game().await;
    // A sibling directory whose name merely starts the same way must not be
    // treated as being inside the root.
    plant(&pool, "m1", "C:\\Games\\Genshin\\ModsBackup\\Amber").await;
    plant(&pool, "m2", "D:\\Elsewhere\\Amber").await;

    run_normalization(&pool).await;

    assert_eq!(
        stored_path(&pool, "m1").await,
        "C:\\Games\\Genshin\\ModsBackup\\Amber"
    );
    assert_eq!(stored_path(&pool, "m2").await, "D:\\Elsewhere\\Amber");
}

#[tokio::test]
async fn a_row_equal_to_the_root_itself_is_not_emptied() {
    let pool = pool_with_game().await;
    plant(&pool, "m1", ROOT).await;

    run_normalization(&pool).await;

    assert_eq!(stored_path(&pool, "m1").await, ROOT);
}

#[tokio::test]
async fn the_root_with_a_trailing_separator_is_not_emptied() {
    let pool = pool_with_game().await;
    // The separator check alone would accept this and strip everything after
    // it, leaving folder_path as the empty string -- a row pointing at
    // nothing. The length guard is what stops it.
    plant(&pool, "m1", &format!("{ROOT}\\")).await;

    run_normalization(&pool).await;

    assert_eq!(stored_path(&pool, "m1").await, format!("{ROOT}\\"));
}

#[tokio::test]
async fn normalization_leaves_the_key_and_the_id_alone() {
    let pool = pool_with_game().await;
    let absolute = format!("{ROOT}\\Amber\\Blue Dress");
    plant(&pool, "m1", &absolute).await;

    let before: String = sqlx::query_scalar("SELECT folder_path_key FROM mods WHERE id = 'm1'")
        .fetch_one(&pool)
        .await
        .expect("key");

    run_normalization(&pool).await;

    let after: (String, String) =
        sqlx::query_as("SELECT id, folder_path_key FROM mods WHERE folder_path = ?")
            .bind("Amber\\Blue Dress")
            .fetch_one(&pool)
            .await
            .expect("row still addressable by id");

    assert_eq!(after.0, "m1", "four tables reference mods(id)");
    assert_eq!(after.1, before, "the key is the same for either path form");
}

#[tokio::test]
async fn both_path_forms_produce_the_same_key() {
    // This is why the migration can leave folder_path_key alone. If it ever
    // stops being true, the migration is incomplete.
    let absolute = format!("{ROOT}\\Amber\\Blue Dress");
    assert_eq!(
        folder_path_key(&absolute, Some(ROOT)),
        folder_path_key("Amber\\Blue Dress", Some(ROOT)),
    );
}
