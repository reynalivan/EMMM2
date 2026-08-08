//! Identity resolution and merge policy for `ensure_object_exists`.
//!
//! Every scan and every disk reconcile funnels through this function to decide
//! whether an object on disk is one the database already knows. Getting the
//! resolution order or an overwrite rule wrong does not fail loudly — it
//! silently merges two objects, or splits one into two. These pin each arm
//! before the policy moves out of the repo.

use crate::domain::models::GameType;
use crate::repo::object_repo::{ensure_object_exists, EnsureObjectInput, MatchSource};
use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};
use sqlx::SqlitePool;

const GAME: &str = "g1";

async fn setup() -> SqlitePool {
    let pool = init_test_db().await.pool;
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: GAME,
            name: "Game",
            game_type: GameType::GIMI,
            path: "C:\\Game",
            mods_path: Some("C:\\Game\\Mods"),
        },
    )
    .await
    .expect("game fixture");
    pool
}

/// Only the four fields the identity rules read vary between cases; the JSON
/// columns are spelled once here so a test that is not about them says so.
struct Ensure<'a> {
    folder_path: &'a str,
    obj_name: &'a str,
    obj_type: &'a str,
    source: MatchSource,
}

impl Ensure<'_> {
    fn masterdb(folder_path: &'static str, obj_name: &'static str, obj_type: &'static str) -> Self {
        Self {
            folder_path,
            obj_name,
            obj_type,
            source: MatchSource::MasterDb,
        }
    }

    fn disk(folder_path: &'static str, obj_name: &'static str, obj_type: &'static str) -> Self {
        Self {
            folder_path,
            obj_name,
            obj_type,
            source: MatchSource::Disk,
        }
    }
}

async fn ensure(pool: &SqlitePool, case: Ensure<'_>) -> (String, usize) {
    let mut conn = pool.acquire().await.expect("conn");
    let mut new_objects = 0;
    let id = ensure_object_exists(
        &mut conn,
        EnsureObjectInput {
            game_id: GAME,
            folder_path: case.folder_path,
            obj_name: case.obj_name,
            obj_type: case.obj_type,
            db_thumbnail: None,
            db_tags_json: "[]",
            db_metadata_json: "{}",
            db_hash_db_json: None,
            db_custom_skins_json: None,
            source: case.source,
        },
        &mut new_objects,
    )
    .await
    .expect("ensure_object_exists");
    (id, new_objects)
}

/// `(name, folder_path, object_type)` as stored.
async fn row(pool: &SqlitePool, id: &str) -> (String, String, String) {
    sqlx::query_as("SELECT name, folder_path, object_type FROM objects WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("object row")
}

async fn object_count(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind(GAME)
        .fetch_one(pool)
        .await
        .expect("count")
}

#[tokio::test]
async fn an_unknown_object_is_inserted_and_counted() {
    let pool = setup().await;

    let (id, new_objects) = ensure(&pool, Ensure::masterdb("Mods/Hook", "Hook", "Character")).await;

    assert_eq!(new_objects, 1);
    assert_eq!(
        row(&pool, &id).await,
        ("Hook".into(), "Mods/Hook".into(), "Character".into())
    );
}

#[tokio::test]
async fn a_name_match_moves_the_object_and_is_not_a_new_object() {
    let pool = setup().await;
    let (first, _) = ensure(&pool, Ensure::masterdb("Mods/Hook", "Hook", "Character")).await;

    // Same object, renamed folder on disk. Matching by name key is what keeps
    // this from being filed as a second object.
    let (second, new_objects) = ensure(
        &pool,
        Ensure::masterdb("Mods/Hook Moved", "hook", "Character"),
    )
    .await;

    assert_eq!(second, first);
    assert_eq!(new_objects, 0);
    assert_eq!(object_count(&pool).await, 1);
    let (name, folder, _) = row(&pool, &first).await;
    assert_eq!(folder, "Mods/Hook Moved");
    // The incoming spelling wins: MasterDB casing is the canonical one.
    assert_eq!(name, "hook");
}

#[tokio::test]
async fn a_name_match_does_not_steal_a_folder_another_object_holds() {
    let pool = setup().await;
    let (hook, _) = ensure(&pool, Ensure::masterdb("Mods/Hook", "Hook", "Character")).await;
    let (klee, _) = ensure(&pool, Ensure::masterdb("Mods/Klee", "Klee", "Character")).await;

    // "Hook" now claims Klee's folder. Two rows cannot hold one folder, so the
    // move is refused rather than silently pointing both at the same place.
    let (matched, _) = ensure(&pool, Ensure::masterdb("Mods/Klee", "Hook", "Character")).await;

    assert_eq!(matched, hook);
    assert_eq!(row(&pool, &hook).await.1, "Mods/Hook");
    assert_eq!(row(&pool, &klee).await.1, "Mods/Klee");
}

#[tokio::test]
async fn a_folder_match_renames_the_object() {
    let pool = setup().await;
    let (first, _) = ensure(&pool, Ensure::masterdb("Mods/Hook", "Hook", "Character")).await;

    // No name match: the folder is the only link. The object is the same
    // physical thing, so it takes the new name rather than spawning a row.
    let (second, new_objects) =
        ensure(&pool, Ensure::masterdb("Mods/Hook", "Kirara", "Character")).await;

    assert_eq!(second, first);
    assert_eq!(new_objects, 0);
    assert_eq!(object_count(&pool).await, 1);
    assert_eq!(row(&pool, &first).await.0, "Kirara");
}

#[tokio::test]
async fn a_name_match_outranks_a_folder_match() {
    let pool = setup().await;
    let (hook, _) = ensure(&pool, Ensure::masterdb("Mods/Hook", "Hook", "Character")).await;
    let (klee, _) = ensure(&pool, Ensure::masterdb("Mods/Klee", "Klee", "Character")).await;

    // "Hook" at Klee's folder matches one row by name and a different row by
    // folder. Name wins -- identity follows the object, not the directory.
    let (matched, _) = ensure(&pool, Ensure::masterdb("Mods/Klee", "Hook", "Character")).await;

    assert_eq!(matched, hook);
    assert_ne!(matched, klee);
    assert_eq!(row(&pool, &klee).await.0, "Klee");
}

#[tokio::test]
async fn only_masterdb_may_overwrite_the_object_type() {
    let pool = setup().await;
    let (id, _) = ensure(&pool, Ensure::masterdb("Mods/Hook", "Hook", "Character")).await;

    // A disk walk knows the folder name and nothing else, so its guess at the
    // type must not overwrite what MasterDB established.
    ensure(&pool, Ensure::disk("Mods/Hook", "Hook", "Other")).await;
    assert_eq!(row(&pool, &id).await.2, "Character");

    ensure(&pool, Ensure::masterdb("Mods/Hook", "Hook", "Weapon")).await;
    assert_eq!(row(&pool, &id).await.2, "Weapon");
}

#[tokio::test]
async fn a_folder_match_respects_the_same_type_authority() {
    let pool = setup().await;
    let (id, _) = ensure(&pool, Ensure::masterdb("Mods/Hook", "Hook", "Character")).await;

    // Reached through the folder arm (the name changes), so the authority rule
    // has to be enforced there too and not only on the name arm.
    ensure(&pool, Ensure::disk("Mods/Hook", "Kirara", "Other")).await;
    assert_eq!(row(&pool, &id).await.2, "Character");
}

#[tokio::test]
async fn a_match_fills_empty_columns_without_touching_what_the_user_set() {
    let pool = setup().await;
    let (id, _) = ensure(&pool, Ensure::disk("Mods/Hook", "Hook", "Other")).await;

    sqlx::query("UPDATE objects SET tags = ?, thumbnail_path = ? WHERE id = ?")
        .bind(r#"["mine"]"#)
        .bind("C:\\mine.png")
        .bind(&id)
        .execute(&pool)
        .await
        .expect("user edit");

    let mut conn = pool.acquire().await.expect("conn");
    let mut new_objects = 0;
    ensure_object_exists(
        &mut conn,
        EnsureObjectInput {
            game_id: GAME,
            folder_path: "Mods/Hook",
            obj_name: "Hook",
            obj_type: "Character",
            db_thumbnail: Some("C:\\masterdb.png"),
            db_tags_json: r#"["canonical"]"#,
            db_metadata_json: r#"{"origin":"masterdb"}"#,
            db_hash_db_json: Some(r#"{"h":1}"#),
            db_custom_skins_json: Some("[]"),
            source: MatchSource::MasterDb,
        },
        &mut new_objects,
    )
    .await
    .expect("ensure");
    // The test pool is single-connection; hold it and the read below deadlocks.
    drop(conn);

    let (tags, thumbnail, metadata, hash_db): (String, String, String, Option<String>) =
        sqlx::query_as("SELECT tags, thumbnail_path, metadata, hash_db FROM objects WHERE id = ?")
            .bind(&id)
            .fetch_one(&pool)
            .await
            .expect("row");

    // What the user set stays; what was still empty gets filled.
    assert_eq!(tags, r#"["mine"]"#);
    assert_eq!(thumbnail, "C:\\mine.png");
    assert_eq!(metadata, r#"{"origin":"masterdb"}"#);
    assert_eq!(hash_db.as_deref(), Some(r#"{"h":1}"#));
}

#[tokio::test]
async fn a_real_value_survives_a_later_sync_that_carries_nothing() {
    let pool = setup().await;
    let (id, _) = ensure(&pool, Ensure::disk("Mods/Hook", "Hook", "Other")).await;

    sqlx::query("UPDATE objects SET metadata = ? WHERE id = ?")
        .bind(r#"{"note":"keep"}"#)
        .bind(&id)
        .execute(&pool)
        .await
        .expect("seed metadata");

    // `[]` and `{}` are how the schema spells "nothing set yet", so an incoming
    // one must not overwrite a real value.
    ensure(&pool, Ensure::masterdb("Mods/Hook", "Hook", "Character")).await;

    let metadata: String = sqlx::query_scalar("SELECT metadata FROM objects WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .expect("metadata");
    assert_eq!(metadata, r#"{"note":"keep"}"#);
}

#[tokio::test]
async fn objects_in_different_games_never_merge() {
    let pool = setup().await;
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g2",
            name: "Other Game",
            game_type: GameType::SRMI,
            path: "C:\\Other",
            mods_path: Some("C:\\Other\\Mods"),
        },
    )
    .await
    .expect("second game");

    let (first, _) = ensure(&pool, Ensure::masterdb("Mods/Hook", "Hook", "Character")).await;

    let mut conn = pool.acquire().await.expect("conn");
    let mut new_objects = 0;
    // Same name and same relative folder in a second game. Both lookups are
    // scoped by game_id, so this is a different object.
    let second = ensure_object_exists(
        &mut conn,
        EnsureObjectInput {
            game_id: "g2",
            folder_path: "Mods/Hook",
            obj_name: "Hook",
            obj_type: "Character",
            db_thumbnail: None,
            db_tags_json: "[]",
            db_metadata_json: "{}",
            db_hash_db_json: None,
            db_custom_skins_json: None,
            source: MatchSource::MasterDb,
        },
        &mut new_objects,
    )
    .await
    .expect("ensure");

    assert_ne!(second, first);
    assert_eq!(new_objects, 1);
}
