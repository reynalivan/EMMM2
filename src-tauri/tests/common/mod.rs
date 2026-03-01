use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::sync::Once;

static INIT: Once = Once::new();

pub struct TestContext {
    pub pool: Pool<Sqlite>,
}

pub async fn init_test_db() -> TestContext {
    INIT.call_once(|| {
        let _ = env_logger::builder().is_test(true).try_init();
    });

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    TestContext { pool }
}
