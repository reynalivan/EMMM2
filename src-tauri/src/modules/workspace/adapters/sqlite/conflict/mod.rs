use crate::modules::workspace::domain::conflicts::IgnoredConflict;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

const MAX_ACTIVE_CONFLICT_GROUPS_PER_REQUEST: usize = 1_000;
const MAX_MOD_PATHS_PER_ACTIVE_CONFLICT_GROUP: usize = 64;

pub fn canonical_active_conflict_group_key(mod_paths: &[String]) -> Result<String, String> {
    if mod_paths.len() < 2 {
        return Err("An active conflict group must contain at least two mod paths".to_string());
    }
    if mod_paths.len() > MAX_MOD_PATHS_PER_ACTIVE_CONFLICT_GROUP {
        return Err(format!(
            "An active conflict group cannot contain more than {MAX_MOD_PATHS_PER_ACTIVE_CONFLICT_GROUP} mod paths"
        ));
    }

    let mut normalized = Vec::with_capacity(mod_paths.len());
    for path in mod_paths {
        if path.trim().is_empty() {
            return Err("An active conflict group cannot contain an empty mod path".to_string());
        }
        normalized.push(path.to_string());
    }

    normalized.sort_unstable();
    normalized.dedup();

    if normalized.len() < 2 {
        return Err("An active conflict group must contain two distinct mod paths".to_string());
    }

    serde_json::to_string(&normalized)
        .map_err(|error| format!("Could not serialize active conflict paths: {error}"))
}

pub async fn list_ignored_active_mod_conflict_group_keys(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT mod_paths
         FROM ignored_active_mod_conflict_groups
         WHERE game_id = ?
         ORDER BY created_at DESC, mod_paths",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
}

pub async fn set_ignored_active_mod_conflict_group_keys(
    pool: &SqlitePool,
    game_id: &str,
    group_keys: &[String],
    ignored: bool,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;

    for group_key in group_keys {
        if ignored {
            sqlx::query(
                "INSERT INTO ignored_active_mod_conflict_groups (game_id, mod_paths)
                 VALUES (?, ?)
                 ON CONFLICT(game_id, mod_paths) DO NOTHING",
            )
            .bind(game_id)
            .bind(group_key)
            .execute(&mut *transaction)
            .await?;
        } else {
            sqlx::query(
                "DELETE FROM ignored_active_mod_conflict_groups
                 WHERE game_id = ? AND mod_paths = ?",
            )
            .bind(game_id)
            .bind(group_key)
            .execute(&mut *transaction)
            .await?;
        }
    }

    transaction.commit().await
}

pub fn canonical_active_conflict_group_keys(
    mod_path_groups: &[Vec<String>],
) -> Result<Vec<String>, String> {
    if mod_path_groups.is_empty() {
        return Err("At least one active conflict group is required".to_string());
    }
    if mod_path_groups.len() > MAX_ACTIVE_CONFLICT_GROUPS_PER_REQUEST {
        return Err(format!(
            "A request cannot contain more than {MAX_ACTIVE_CONFLICT_GROUPS_PER_REQUEST} active conflict groups"
        ));
    }

    let mut group_keys = mod_path_groups
        .iter()
        .map(|mod_paths| canonical_active_conflict_group_key(mod_paths))
        .collect::<Result<Vec<_>, _>>()?;
    group_keys.sort_unstable();
    group_keys.dedup();
    Ok(group_keys)
}

/// Fetches all ignored conflicts for a game, enriched with object and mod names.
pub async fn list_ignored_object_conflicts(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<IgnoredConflict>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT ic.*, o.name as object_name 
         FROM ignored_object_conflicts ic
         LEFT JOIN objects o ON ic.object_id = o.id
         WHERE ic.game_id = ?
         ORDER BY ic.created_at DESC",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;
    let mut list = rows
        .iter()
        .map(|row| {
            Ok(IgnoredConflict {
                id: row.try_get("id")?,
                game_id: row.try_get("game_id")?,
                object_id: row.try_get("object_id")?,
                object_name: row.try_get("object_name")?,
                mod_ids: row.try_get("mod_ids")?,
                mod_names: Vec::new(),
                created_at: row.try_get("created_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

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

#[cfg(test)]
mod tests {
    use super::{canonical_active_conflict_group_key, canonical_active_conflict_group_keys};

    #[test]
    fn canonical_group_key_sorts_and_deduplicates_paths() {
        let key = canonical_active_conflict_group_key(&[
            "E:/Mods/ModB".to_string(),
            "E:/Mods/ModA".to_string(),
            "E:/Mods/ModB".to_string(),
        ])
        .expect("the group should be valid");

        assert_eq!(key, r#"["E:/Mods/ModA","E:/Mods/ModB"]"#);
    }

    #[test]
    fn canonical_group_key_rejects_single_or_blank_paths() {
        assert!(canonical_active_conflict_group_key(&["E:/Mods/ModA".to_string()]).is_err());
        assert!(
            canonical_active_conflict_group_key(&["E:/Mods/ModA".to_string(), " ".to_string(),])
                .is_err()
        );
    }

    #[test]
    fn canonical_group_keys_reject_an_empty_bulk_action() {
        assert!(canonical_active_conflict_group_keys(&[]).is_err());
    }
}
