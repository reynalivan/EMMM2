use serde::{Deserialize, Serialize};
use sqlx::{Executor, QueryBuilder, Sqlite, SqliteConnection, SqlitePool};

use crate::modules::games::domain::models::GameType;

/// Game configuration row stored in the `games` table.
/// Uses the extended columns from migration 012.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameRow {
    pub id: String,
    pub name: String,
    pub game_type: crate::modules::games::domain::models::GameType,
    pub path: String,
    pub mods_path: Option<String>,
    pub ready_to_move_path: Option<String>,
    pub game_exe: Option<String>,
    pub launcher_path: Option<String>,
    pub loader_exe: Option<String>,
    pub launch_mode: String,
    pub xxmi_launcher_exe: Option<String>,
    pub launch_args: Option<String>,
}

// ── Games CRUD ──────────────────────────────────────────────

/// Get all configured games.
pub async fn get_all_games(pool: &SqlitePool) -> Result<Vec<GameRow>, sqlx::Error> {
    let rows = sqlx::query_as!(
        GameRow,
        r#"SELECT
            id,
            name,
            game_type AS "game_type: GameType",
            path,
            mods_path,
            ready_to_move_path,
            game_exe,
            launcher_path,
            loader_exe,
            launch_mode,
            xxmi_launcher_exe,
            launch_args
        FROM games
        ORDER BY name"#
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
pub async fn upsert_game<'e, E>(executor: E, game: &GameRow) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query!(
        "INSERT INTO games (id, name, game_type, path, mods_path, ready_to_move_path, game_exe, launcher_path, loader_exe, launch_mode, xxmi_launcher_exe, launch_args, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(id) DO UPDATE SET
           name = excluded.name,
           game_type = excluded.game_type,
           path = excluded.path,
           mods_path = excluded.mods_path,
           ready_to_move_path = excluded.ready_to_move_path,
           game_exe = excluded.game_exe,
           launcher_path = excluded.launcher_path,
           loader_exe = excluded.loader_exe,
           launch_mode = excluded.launch_mode,
           xxmi_launcher_exe = excluded.xxmi_launcher_exe,
           launch_args = excluded.launch_args",
        game.id,
        game.name,
        game.game_type,
        game.path,
        game.mods_path,
        game.ready_to_move_path,
        game.game_exe,
        game.launcher_path,
        game.loader_exe,
        game.launch_mode,
        game.xxmi_launcher_exe,
        game.launch_args,
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Delete only games explicitly removed from the previously loaded snapshot.
pub async fn delete_games_by_ids(
    connection: &mut SqliteConnection,
    removed_ids: &[String],
) -> Result<(), sqlx::Error> {
    if removed_ids.is_empty() {
        return Ok(());
    }

    let mut query = QueryBuilder::<Sqlite>::new("DELETE FROM games WHERE id IN (");
    {
        let mut separated = query.separated(", ");
        for id in removed_ids {
            separated.push_bind(id);
        }
    }
    query.push(")");
    query.build().execute(connection).await?;
    Ok(())
}

/// Count total games (used for check_config_status).
pub async fn count_games(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!("SELECT COUNT(*) FROM games")
        .fetch_one(pool)
        .await
}

/// Get the mod path for a specific game by ID.
/// Raw configured `mods_path` (may be None; no fallback — `get_mod_path`
/// falls back to the game `path` when `mods_path` is unset).
pub async fn get_configured_mods_path(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let value = sqlx::query_scalar!("SELECT mods_path FROM games WHERE id = ?", game_id)
        .fetch_optional(pool)
        .await?;
    Ok(value)
}

pub async fn get_game_type(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<GameType>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT game_type AS "game_type: GameType" FROM games WHERE id = ?"#,
        game_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_mod_path(pool: &SqlitePool, game_id: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT COALESCE(NULLIF(mods_path, ''), path) AS "mods_path!: String" FROM games WHERE id = ?"#,
        game_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_ready_to_move_config(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<(String, Option<String>)>, sqlx::Error> {
    let config = sqlx::query_as!(
        ReadyToMoveConfig,
        "SELECT name, ready_to_move_path FROM games WHERE id = ?",
        game_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(config.map(Into::into))
}

#[derive(Debug)]
struct ReadyToMoveConfig {
    name: String,
    ready_to_move_path: Option<String>,
}

impl From<ReadyToMoveConfig> for (String, Option<String>) {
    fn from(value: ReadyToMoveConfig) -> Self {
        (value.name, value.ready_to_move_path)
    }
}

pub async fn ensure_game_exists(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    game_name: &str,
    game_type: GameType,
    mods_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT OR IGNORE INTO games (id, name, game_type, path, mods_path) VALUES (?, ?, ?, ?, ?)",
        game_id,
        game_name,
        game_type,
        mods_path,
        mods_path,
    )
    .execute(conn)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
