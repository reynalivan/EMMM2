use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Game configuration row stored in the `games` table.
/// Uses the extended columns from migration 012.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameRow {
    pub id: String,
    pub name: String,
    pub game_type: String,
    pub path: String,
    pub mod_path: Option<String>,
    pub game_exe: Option<String>,
    pub launcher_path: Option<String>,
    pub loader_exe: Option<String>,
    pub launch_args: Option<String>,
}

// ── KV Settings ─────────────────────────────────────────────

/// Get a single setting value by key.
pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

/// Upsert a single setting (INSERT OR REPLACE).
pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR REPLACE INTO app_settings (key, value) VALUES (?, ?)")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}

/// Fetch all settings as a HashMap.
pub async fn get_all_settings(pool: &SqlitePool) -> Result<HashMap<String, String>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM app_settings")
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().collect())
}

// ── Games CRUD ──────────────────────────────────────────────

/// Get all configured games.
pub async fn get_all_games(pool: &SqlitePool) -> Result<Vec<GameRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, GameRow>(
        "SELECT id, name, game_type, path, mod_path, game_exe, launcher_path, loader_exe, launch_args FROM games ORDER BY name"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Upsert a game row. Uses INSERT OR REPLACE.
pub async fn upsert_game(pool: &SqlitePool, game: &GameRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO games (id, name, game_type, path, mod_path, game_exe, launcher_path, loader_exe, launch_args, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
    )
    .bind(&game.id)
    .bind(&game.name)
    .bind(&game.game_type)
    .bind(&game.path)
    .bind(&game.mod_path)
    .bind(&game.game_exe)
    .bind(&game.launcher_path)
    .bind(&game.loader_exe)
    .bind(&game.launch_args)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete a game by its ID.
pub async fn delete_game(pool: &SqlitePool, game_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM games WHERE id = ?")
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Count total games (used for check_config_status).
pub async fn count_games(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM games")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

// ── Reset ───────────────────────────────────────────────────

/// Delete all user data from every table, restoring the app to fresh-install state.
/// Tables are cleared in FK-safe order within a single transaction.
pub async fn reset_all_data(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Child tables first (FK dependencies)
    sqlx::query("DELETE FROM collection_items")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM dedup_group_members")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM dedup_groups")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM dedup_jobs")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM duplicate_whitelist")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM scan_results")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM mods").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM objects").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM collections")
        .execute(&mut *tx)
        .await?;

    // Root tables
    sqlx::query("DELETE FROM games").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM app_settings")
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/settings_repo_tests.rs"]
mod tests;
