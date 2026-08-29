//! Count queries. The terminal-node rules these feed live in
//! `services::objects::terminal` — resolving them walks the disk, which
//! is not something the data-access layer does.

use crate::domain::objects::ObjectSummary;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::types::*;
pub async fn load_game_mods_path(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT mods_path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(pool)
        .await
        .map(|value| value.flatten())
}

pub async fn load_object_count_candidates(
    pool: &SqlitePool,
    game_id: &str,
    objects: &[ObjectSummary],
) -> Result<Vec<ObjectCountCandidate>, sqlx::Error> {
    if objects.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT m.object_id, m.folder_path, m.actual_name, m.status, m.is_safe, m.safety_source FROM mods m WHERE m.game_id = ",
    );
    qb.push_bind(game_id);
    qb.push(" AND m.object_id IN (");
    {
        let mut separated = qb.separated(", ");
        for object in objects {
            separated.push_bind(&object.id);
        }
    }
    qb.push(")");
    qb.build_query_as::<ObjectCountCandidate>()
        .fetch_all(pool)
        .await
}
