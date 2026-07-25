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

pub(super) async fn get_game_mod_path_for_mod_id<'c, E>(
    executor: E,
    mod_id: &str,
) -> Result<Option<String>, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "SELECT g.mods_path
         FROM mods m
         JOIN games g ON g.id = m.game_id
         WHERE m.id = ?",
    )
    .bind(mod_id)
    .fetch_optional(executor)
    .await
    .map(|row| {
        row.and_then(|value| value.try_get("mods_path").ok())
            .flatten()
    })
}
