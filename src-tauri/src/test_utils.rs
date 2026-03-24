use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::sync::Once;

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
    pub folder_path: Option<&'a str>,
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
    println!("DEBUG: init_test_db START");
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
    println!("DEBUG: Running migrations...");
    m.run(&pool).await.expect("Failed to run migrations");
    println!("DEBUG: Backfilling unicode keys...");
    crate::repo::unicode_keys::ensure_unicode_keys(&pool)
        .await
        .expect("Failed to backfill unicode keys");

    println!("DEBUG: init_test_db DONE");
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
    let folder_path = fixture.folder_path.map(ToString::to_string);
    let folder_path_key = folder_path
        .as_deref()
        .map(|path| folder_path_key(path, None));

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, name_key, folder_path, folder_path_key, object_type, tags, metadata)
         VALUES (?, ?, ?, ?, ?, ?, ?, '[]', '{}')",
    )
    .bind(fixture.id)
    .bind(fixture.game_id)
    .bind(fixture.name)
    .bind(object_name_key(fixture.name))
    .bind(folder_path)
    .bind(folder_path_key)
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
