use crate::domain::conflicts::IgnoredConflict;
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Fetches all ignored conflicts for a game, enriched with object and mod names.
pub async fn list_ignored_object_conflicts(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<IgnoredConflict>, sqlx::Error> {
    let mut list = sqlx::query_as::<_, IgnoredConflict>(
        "SELECT ic.*, o.name as object_name 
         FROM ignored_object_conflicts ic
         LEFT JOIN objects o ON ic.object_id = o.id
         WHERE ic.game_id = ?
         ORDER BY ic.created_at DESC",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;

    // Resolve every referenced mod id to its name in one pass. Rows whose
    // `mod_ids` is not valid JSON simply contribute no names, matching the
    // previous per-row `serde_json` guard.
    let name_rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT ic.id, COALESCE(m.actual_name, je.value)
         FROM ignored_object_conflicts ic
         JOIN json_each(CASE WHEN json_valid(ic.mod_ids) THEN ic.mod_ids ELSE '[]' END) je
         LEFT JOIN mods m ON m.id = je.value
         WHERE ic.game_id = ?
         ORDER BY ic.id, je.key",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;

    let mut names_by_conflict: HashMap<String, Vec<String>> = HashMap::new();
    for (conflict_id, name) in name_rows {
        names_by_conflict.entry(conflict_id).or_default().push(name);
    }

    for item in &mut list {
        if let Some(names) = names_by_conflict.remove(&item.id) {
            item.mod_names = names;
        }
    }

    Ok(list)
}

/// Checks if a specific combination of mod_ids is ignored for an object.
pub async fn is_conflict_ignored(
    pool: &SqlitePool,
    game_id: &str,
    object_id: &str,
    mod_ids: &[String],
) -> Result<bool, sqlx::Error> {
    if mod_ids.is_empty() {
        return Ok(false);
    }

    let mut sorted_ids = mod_ids.to_vec();
    sorted_ids.sort();
    let mod_ids_json = serde_json::to_string(&sorted_ids).unwrap_or_else(|_| "[]".to_string());

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ignored_object_conflicts 
         WHERE game_id = ? AND object_id = ? AND mod_ids = ?",
    )
    .bind(game_id)
    .bind(object_id)
    .bind(mod_ids_json)
    .fetch_one(pool)
    .await?;

    Ok(count > 0)
}

/// Persists a new ignored conflict combination.
pub async fn ignore_object_conflict(
    pool: &SqlitePool,
    game_id: &str,
    object_id: &str,
    mod_ids: &[String],
) -> Result<String, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let mut sorted_ids = mod_ids.to_vec();
    sorted_ids.sort();
    let mod_ids_json = serde_json::to_string(&sorted_ids).unwrap_or_else(|_| "[]".to_string());

    sqlx::query(
        "INSERT OR IGNORE INTO ignored_object_conflicts (id, game_id, object_id, mod_ids)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(game_id)
    .bind(object_id)
    .bind(mod_ids_json)
    .execute(pool)
    .await?;

    Ok(id)
}

/// Revokes an ignored conflict status.
pub async fn revoke_object_conflict(
    pool: &SqlitePool,
    game_id: &str,
    object_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ignored_object_conflicts WHERE game_id = ? AND object_id = ?")
        .bind(game_id)
        .bind(object_id)
        .execute(pool)
        .await?;
    Ok(())
}
