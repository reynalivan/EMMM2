use sqlx::SqlitePool;

use super::ConfigService;

impl ConfigService {
    /// Create tables and apply ad-hoc schema patches if they don't exist (idempotent).
    pub(super) async fn ensure_tables(pool: &SqlitePool) {
        // Games table (matches 001_init.sql + 012 ALTER extensions)
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS games (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                game_type TEXT NOT NULL,
                path TEXT NOT NULL,
                launcher_path TEXT,
                launch_args TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                mod_path TEXT,
                game_exe TEXT,
                loader_exe TEXT
            )",
        )
        .execute(pool)
        .await;

        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await;

        // Apply fallback schema patches.
        // If `tauri_plugin_sql` migrations failed due to duplicate column errors
        // (e.g. users migrating from older SQLx tracked versions to new Tauri plugin versions),
        // we explicitly add required columns here and safely ignore "duplicate column" errors.
        let patches = [
            "ALTER TABLE games ADD COLUMN mod_path TEXT;",
            "ALTER TABLE games ADD COLUMN game_exe TEXT;",
            "ALTER TABLE games ADD COLUMN loader_exe TEXT;",
            "ALTER TABLE collections ADD COLUMN is_safe BOOLEAN DEFAULT 0;",
            "ALTER TABLE collections ADD COLUMN is_favorite BOOLEAN DEFAULT 0;",
            "ALTER TABLE objects ADD COLUMN is_pinned BOOLEAN DEFAULT 0;",
            "ALTER TABLE objects ADD COLUMN is_auto_sync BOOLEAN NOT NULL DEFAULT 0;",
            "ALTER TABLE mods ADD COLUMN last_status_sfw BOOLEAN;",
            "ALTER TABLE mods ADD COLUMN last_status_nsfw BOOLEAN;",
        ];

        for patch in patches {
            let _ = sqlx::query(patch).execute(pool).await;
        }
    }
}
