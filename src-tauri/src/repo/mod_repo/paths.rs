//! Resolution of a game's `mods_path`, needed to build stable `folder_path_key` values.

use sqlx::Row;

pub(super) async fn get_game_mod_path<'c, E>(
    executor: E,
    game_id: &str,
) -> Result<Option<String>, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query("SELECT mods_path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(executor)
        .await
        .map(|row| {
            row.and_then(|value| value.try_get("mods_path").ok())
                .flatten()
        })
}
