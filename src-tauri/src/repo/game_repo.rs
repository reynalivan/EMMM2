use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Game configuration row stored in the `games` table.
/// Uses the extended columns from migration 012.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameRow {
    pub id: String,
    pub name: String,
    pub game_type: crate::domain::models::GameType,
    pub path: String,
    pub mods_path: Option<String>,
    pub game_exe: Option<String>,
    pub launcher_path: Option<String>,
    pub loader_exe: Option<String>,
    pub launch_args: Option<String>,
}

// ── Games CRUD ──────────────────────────────────────────────

/// Get all configured games.
pub async fn get_all_games(pool: &SqlitePool) -> Result<Vec<GameRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, GameRow>(
        "SELECT id, name, game_type, path, mods_path, game_exe, launcher_path, loader_exe, launch_args FROM games ORDER BY name"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Upsert a game row.
///
/// # Safety invariant
/// Uses `INSERT ... ON CONFLICT(id) DO UPDATE SET` (true UPSERT).
/// **NEVER** use `INSERT OR REPLACE` here — SQLite implements that as
/// DELETE + INSERT, which triggers `ON DELETE CASCADE` on `objects` and
/// `mods` tables, permanently wiping all child rows.
pub async fn upsert_game(pool: &SqlitePool, game: &GameRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO games (id, name, game_type, path, mods_path, game_exe, launcher_path, loader_exe, launch_args, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(id) DO UPDATE SET
           name = excluded.name,
           game_type = excluded.game_type,
           path = excluded.path,
           mods_path = excluded.mods_path,
           game_exe = excluded.game_exe,
           launcher_path = excluded.launcher_path,
           loader_exe = excluded.loader_exe,
           launch_args = excluded.launch_args",
    )
    .bind(&game.id)
    .bind(&game.name)
    .bind(game.game_type)
    .bind(&game.path)
    .bind(&game.mods_path)
    .bind(&game.game_exe)
    .bind(&game.launcher_path)
    .bind(&game.loader_exe)
    .bind(&game.launch_args)
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

/// Get the mod path for a specific game by ID.
/// Raw configured `mods_path` (may be None; no fallback — `get_mod_path`
/// falls back to the game `path` when `mods_path` is unset).
pub async fn get_configured_mods_path(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let value: Option<Option<String>> =
        sqlx::query_scalar("SELECT mods_path FROM games WHERE id = ?")
            .bind(game_id)
            .fetch_optional(pool)
            .await?;
    Ok(value.flatten())
}

/// Raw `game_type` discriminant, used to pick the matching Master DB resource file.
pub async fn get_game_type_raw(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!("SELECT game_type FROM games WHERE id = ?", game_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_mod_path(pool: &SqlitePool, game_id: &str) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COALESCE(NULLIF(mods_path, ''), path) AS mods_path FROM games WHERE id = ?",
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        use sqlx::Row;
        Ok(r.try_get("mods_path").ok())
    } else {
        Ok(None)
    }
}

pub async fn ensure_game_exists(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    game_name: &str,
    game_type: crate::domain::models::GameType,
    mods_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO games (id, name, game_type, path, mods_path) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(game_id)
    .bind(game_name)
    .bind(game_type)
    .bind(mods_path)
    .bind(mods_path)
    .execute(conn)
    .await?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/game_repo_test.rs"]
mod tests;
