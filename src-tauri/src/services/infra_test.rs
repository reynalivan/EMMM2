#[cfg(test)]
mod tests {
    use crate::test_utils::init_test_db;

    #[tokio::test]
    async fn test_db_initialization() {
        let ctx = init_test_db().await;

        // precise table check: ensure 'games' table exists
        let result =
            sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='games'")
                .fetch_optional(&ctx.pool)
                .await
                .expect("Failed to query database");

        assert!(result.is_some(), "Games table should exist in test DB");
    }
}
