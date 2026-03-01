use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

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

/// Get the mod path for a specific game by ID.
pub async fn get_mod_path(pool: &SqlitePool, game_id: &str) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT mod_path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(pool)
        .await?;

    if let Some(r) = row {
        use sqlx::Row;
        Ok(r.try_get("mod_path").ok())
    } else {
        Ok(None)
    }
}

pub async fn ensure_game_exists(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    game_name: &str,
    game_type: &str,
    mods_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR IGNORE INTO games (id, name, game_type, path) VALUES (?, ?, ?, ?)")
        .bind(game_id)
        .bind(game_name)
        .bind(game_type)
        .bind(mods_path)
        .execute(conn)
        .await?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/game_repo_test.rs"]
mod tests;
