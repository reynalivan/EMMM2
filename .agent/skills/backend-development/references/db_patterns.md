# Database Patterns (SQLx & SQLite)

## 1. Schema Best Practices

- **UUIDs**: Use `TEXT` primary keys with UUID v4.
  ```sql
  id TEXT PRIMARY KEY NOT NULL
  ```
- **Timestamps**: Use `DATETIME DEFAULT CURRENT_TIMESTAMP`.
- **Booleans**: Use `INTEGER` (0/1) as SQLite has no native BOOLEAN.
- **JSON**: Use `TEXT` for complex metadata chunks (e.g., `metadata_blob`).

## 2. Performance Tuning

Run these on app startup (`main.rs`):

```rust
sqlx::query("PRAGMA journal_mode = WAL;").execute(&pool).await?;
sqlx::query("PRAGMA synchronous = NORMAL;").execute(&pool).await?;
sqlx::query("PRAGMA foreign_keys = ON;").execute(&pool).await?;
```

## 3. Query Optimization

- **Indexes**: Index columns used in `WHERE`, `ORDER BY`, and `JOIN`.
  ```sql
  CREATE INDEX idx_mods_game_id ON mods(game_id);
  ```
- **Explain**: Use `EXPLAIN QUERY PLAN` if a query feels slow.

## 4. Migrations

- Location: `src-tauri/migrations/`.
- Naming: `YYYYMMDDHHMMSS_description.sql`.
- Execution: Automatic on startup via `sqlx::migrate!()`.
