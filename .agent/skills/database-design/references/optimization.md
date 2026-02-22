# SQLite Optimization

## 1. Pragma Tuning (Startup)
Execute these on every app launch:
```rust
sqlx::query("PRAGMA journal_mode = WAL;").execute(&pool).await?;
sqlx::query("PRAGMA synchronous = NORMAL;").execute(&pool).await?;
sqlx::query("PRAGMA foreign_keys = ON;").execute(&pool).await?;
sqlx::query("PRAGMA busy_timeout = 5000;").execute(&pool).await?;
```

## 2. Indexing Strategy
-   **Covering Index**: Include extra columns to avoid table lookup.
    ```sql
    CREATE INDEX idx_mods_list ON mods(game_id, status) INCLUDE (actual_name);
    ```
-   **Partial Index**: Index only active items.
    ```sql
    CREATE INDEX idx_active_mods ON mods(id) WHERE status = 'ENABLED';
    ```

## 3. Query Optimization
-   **Avoid `OFFSET`**: Use Keyset Pagination (Cursor).
    -   *Bad*: `LIMIT 10 OFFSET 1000`
    -   *Good*: `WHERE created_at < last_seen_date LIMIT 10`
-   **No `N+1`**: Use `WHERE id IN (...)` for batch fetching.
