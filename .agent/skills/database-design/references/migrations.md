# SQLx Migrations

## 1. Workflow

1.  **Create**: `sqlx migrate add <name>` (e.g., `add_indexes`).
2.  **Edit**: File created in `src-tauri/migrations/`.
3.  **Run**: App runs `sqlx::migrate!()` on boot.

## 2. Safe Migration Rules

- **Never Change History**: Do not edit old migration files. Create a new one to "undo" or "fix".
- **Transactions**: Migrations run in a transaction automatically suitable for SQLite.
- **Idempotency**: Use `CREATE TABLE IF NOT EXISTS` or `DROP TABLE IF EXISTS` carefully.

## 3. Zero Downtime (In-App)

Since this is a specific desktop app:

1.  Add new column as `NULL`.
2.  Backfill data in Rust background task if heavy.
3.  Set `NOT NULL` in a future migration if needed (requires table recreate in SQLite).
