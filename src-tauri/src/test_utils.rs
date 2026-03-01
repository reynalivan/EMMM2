use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::sync::Once;

static INIT: Once = Once::new();

pub struct TestContext {
    pub pool: Pool<Sqlite>,
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

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    TestContext { pool }
}
