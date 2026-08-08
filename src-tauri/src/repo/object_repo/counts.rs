//! Count queries. The terminal-node rules these feed live in
//! `services::objects::terminal` — resolving them walks the disk, which
//! is not something the data-access layer does.

use crate::domain::objects::ObjectSummary;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::types::*;
use crate::common::corridor_constants::{CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN};

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
    safe_mode: bool,
    objects: &[ObjectSummary],
) -> Result<Vec<ObjectCountCandidate>, sqlx::Error> {
    if objects.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT m.object_id, m.folder_path, m.actual_name, m.status FROM mods m WHERE m.game_id = ",
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
    append_corridor_visibility_filter(&mut qb, safe_mode);

    qb.build_query_as::<ObjectCountCandidate>()
        .fetch_all(pool)
        .await
}

pub(super) fn append_corridor_visibility_filter(qb: &mut QueryBuilder<Sqlite>, safe_mode: bool) {
    let expected_is_safe = if safe_mode { 1 } else { 0 };
    qb.push(" AND (COALESCE(m.is_safe, 1) = ");
    qb.push_bind(expected_is_safe);
    qb.push(" OR COALESCE(m.corridor_source, ");
    qb.push_bind(CORRIDOR_SOURCE_UNKNOWN);
    qb.push(") IN (");
    qb.push_bind(CORRIDOR_SOURCE_MANUAL);
    qb.push(", ");
    qb.push_bind(CORRIDOR_SOURCE_UNKNOWN);
    qb.push("))");
}
