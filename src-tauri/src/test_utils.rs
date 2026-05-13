use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::sync::Once;

use crate::domain::collection::ProjectedCollectionState;
use crate::repo::game_repo::{upsert_game, GameRow};
use crate::services::path_key::{collection_name_key, folder_path_key, object_name_key};

static INIT: Once = Once::new();

pub struct TestContext {
    pub pool: Pool<Sqlite>,
}

pub struct TestGameFixture<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub game_type: crate::database::models::GameType,
    pub path: &'a str,
    pub mods_path: Option<&'a str>,
}

pub struct TestObjectFixture<'a> {
    pub id: &'a str,
    pub game_id: &'a str,
    pub name: &'a str,
    pub folder_path: &'a str,
    pub object_type: &'a str,
}

pub struct TestModFixture<'a> {
    pub id: &'a str,
    pub game_id: &'a str,
    pub object_id: Option<&'a str>,
    pub actual_name: &'a str,
    pub folder_path: &'a str,
    pub status: crate::database::models::ItemStatus,
    pub is_safe: bool,
    pub object_type: Option<&'a str>,
    pub mods_path: Option<&'a str>,
}

pub struct TestCollectionFixture<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub game_id: &'a str,
    pub is_safe: bool,
    pub is_last_unsaved: bool,
}

pub struct TestCollectionObjectStateFixture<'a> {
    pub collection_id: &'a str,
    pub object_id: &'a str,
    pub is_enabled: bool,
}

pub async fn init_test_db() -> TestContext {
    INIT.call_once(|| {
        // Initialize logger only once
        let _ = env_logger::builder().is_test(true).try_init();
    });

    // Create an in-memory database for each test
    // Shared cache allows multiple connections to the same in-memory DB
    let pool = SqlitePoolOptions::new()
        .max_connections(1) // Single connection to avoid locking issues in tests
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    // Run migrations (force cache bust)
    let m = sqlx::migrate!("./migrations");
    m.run(&pool).await.expect("Failed to run migrations");
    crate::repo::unicode_keys::ensure_unicode_keys(&pool)
        .await
        .expect("Failed to backfill unicode keys");

    TestContext { pool }
}

pub async fn insert_test_game(
    pool: &Pool<Sqlite>,
    fixture: &TestGameFixture<'_>,
) -> Result<(), sqlx::Error> {
    let game = GameRow {
        id: fixture.id.to_string(),
        name: fixture.name.to_string(),
        game_type: fixture.game_type,
        path: fixture.path.to_string(),
        mods_path: fixture.mods_path.map(ToString::to_string),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };

    upsert_game(pool, &game).await
}

pub async fn insert_test_object(
    pool: &Pool<Sqlite>,
    fixture: &TestObjectFixture<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, name_key, folder_path, folder_path_key, object_type, tags, metadata)
         VALUES (?, ?, ?, ?, ?, ?, ?, '[]', '{}')",
    )
    .bind(fixture.id)
    .bind(fixture.game_id)
    .bind(fixture.name)
    .bind(object_name_key(fixture.name))
    .bind(fixture.folder_path)
    .bind(folder_path_key(fixture.folder_path, None))
    .bind(fixture.object_type)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_test_mod(
    pool: &Pool<Sqlite>,
    fixture: &TestModFixture<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO mods (id, game_id, object_id, actual_name, folder_path, folder_path_key, status, object_type, is_favorite, is_safe, corridor_source, size_bytes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'unknown', 0)",
    )
    .bind(fixture.id)
    .bind(fixture.game_id)
    .bind(fixture.object_id)
    .bind(fixture.actual_name)
    .bind(fixture.folder_path)
    .bind(folder_path_key(fixture.folder_path, fixture.mods_path))
    .bind(fixture.status as i64)
    .bind(fixture.object_type.unwrap_or("Other"))
    .bind(0) // is_favorite
    .bind(fixture.is_safe)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_test_collection_snapshot(
    pool: &Pool<Sqlite>,
    collection_id: &str,
    state: &ProjectedCollectionState,
) -> Result<(), sqlx::Error> {
    let snapshot_json = crate::services::projected_state_service::serialize_snapshot_json(state)
        .unwrap_or_else(|| {
            "{\"object_states\":[],\"active_roots\":[],\"summary\":{\"object_count\":0,\"enabled_object_count\":0,\"active_root_count\":0,\"missing_root_count\":0}}".to_string()
        });
    let signature = crate::services::projected_state_service::signature_for_projected_state(state);
    let active_root_count = state.summary.active_root_count as i32;

    sqlx::query(
        "UPDATE collections SET snapshot_json = ?, signature = ?, root_count = ?, display_mod_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(snapshot_json)
    .bind(signature)
    .bind(active_root_count)
    .bind(active_root_count)
    .bind(collection_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_test_corridor_pointers_unchecked(
    pool: &Pool<Sqlite>,
    game_id: &str,
    is_safe: bool,
    active_collection_id: Option<&str>,
    undo_collection_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(pool)
        .await?;

    let result = sqlx::query(
        r#"
        INSERT INTO corridor_state (game_id, is_safe, active_collection_id, undo_collection_id)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(game_id, is_safe) DO UPDATE SET
            active_collection_id = excluded.active_collection_id,
            undo_collection_id = excluded.undo_collection_id
        "#,
    )
    .bind(game_id)
    .bind(is_safe_i32)
    .bind(active_collection_id)
    .bind(undo_collection_id)
    .execute(pool)
    .await;

    let restore_result = sqlx::query("PRAGMA foreign_keys = ON").execute(pool).await;
    result?;
    restore_result?;

    Ok(())
}

pub async fn update_test_mod_path_and_status(
    pool: &Pool<Sqlite>,
    mod_id: &str,
    folder_path: &str,
    mods_path: Option<&str>,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET folder_path = ?, folder_path_key = ?, status = ? WHERE id = ?")
        .bind(folder_path)
        .bind(folder_path_key(folder_path, mods_path))
        .bind(
            status
                .parse::<crate::database::models::ItemStatus>()
                .unwrap() as i64,
        )
        .bind(mod_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn insert_test_collection(
    pool: &Pool<Sqlite>,
    fixture: &TestCollectionFixture<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO collections (id, name, name_key, game_id, is_safe, is_last_unsaved)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(fixture.id)
    .bind(fixture.name)
    .bind(collection_name_key(fixture.name))
    .bind(fixture.game_id)
    .bind(fixture.is_safe)
    .bind(fixture.is_last_unsaved)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_test_collection_object_state(
    pool: &Pool<Sqlite>,
    fixture: &TestCollectionObjectStateFixture<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO collection_objects (collection_id, object_id, is_enabled)
         VALUES (?, ?, ?)",
    )
    .bind(fixture.collection_id)
    .bind(fixture.object_id)
    .bind(fixture.is_enabled)
    .execute(pool)
    .await?;

    Ok(())
}
