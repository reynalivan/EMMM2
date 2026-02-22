use crate::services::scanner::deep_matcher;
use sqlx::{Row, SqlitePool};

/// Generate a deterministic mod ID from `game_id` + `relative_path`.
/// Uses BLAKE3 hash (first 32 hex chars) so the same folder always gets the same ID.
/// Per TRD §B.6 — replaces random UUID v4 for mod entries.
pub fn generate_stable_id(game_id: &str, folder_path: &str) -> String {
    let input = format!("{}:{}", game_id, folder_path);
    let hash = blake3::hash(input.as_bytes());
    hash.to_hex()[..32].to_string()
}

pub fn auto_matched_candidate(
    match_result: &deep_matcher::StagedMatchResult,
) -> Option<&deep_matcher::Candidate> {
    if match_result.status != deep_matcher::MatchStatus::AutoMatched {
        return None;
    }

    match_result
        .best
        .as_ref()
        .or_else(|| match_result.candidates_topk.first())
}

/// One-time startup migration: re-hash UUID-format mod IDs to BLAKE3 stable IDs.
///
/// Detects mod rows whose `id` matches UUID v4 format (36 chars with dashes)
/// and replaces them with `generate_stable_id(game_id, folder_path)`.
/// Also updates FK references in `collection_items`.
pub async fn migrate_to_stable_ids(pool: &SqlitePool) -> Result<usize, String> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, game_id, folder_path FROM mods WHERE length(id) = 36 AND id LIKE '%-%-%-%-%'",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to query mods for ID migration: {e}"))?;

    if rows.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let mut migrated = 0;

    for (old_id, game_id, folder_path) in &rows {
        let new_id = generate_stable_id(game_id, folder_path);
        if &new_id == old_id {
            continue;
        }

        sqlx::query("UPDATE collection_items SET mod_id = ? WHERE mod_id = ?")
            .bind(&new_id)
            .bind(old_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        sqlx::query("UPDATE mods SET id = ? WHERE id = ?")
            .bind(&new_id)
            .bind(old_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        migrated += 1;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    if migrated > 0 {
        log::info!("Migrated {migrated} mod IDs from UUID to stable BLAKE3 IDs");
    }

    Ok(migrated)
}

/// Upsert the game record into the `games` table so FK constraints are satisfied.
pub async fn ensure_game_exists(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    game_id: &str,
    game_name: &str,
    game_type: &str,
    mods_path: &str,
) -> Result<(), String> {
    sqlx::query("INSERT OR IGNORE INTO games (id, name, game_type, path) VALUES (?, ?, ?, ?)")
        .bind(game_id)
        .bind(game_name)
        .bind(game_type)
        .bind(mods_path)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to ensure game exists: {e}"))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn ensure_object_exists(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    game_id: &str,
    folder_path: &str,
    obj_name: &str,
    obj_type: &str,
    db_thumbnail: Option<&str>,
    db_tags_json: &str,
    db_metadata_json: &str,
    new_objects_count: &mut usize,
) -> Result<String, String> {
    // Case-insensitive lookup to prevent duplicates (e.g. folder "hook" vs DB alias "Hook")
    let existing = sqlx::query(
        "SELECT id, name, object_type, thumbnail_path, tags, metadata FROM objects WHERE game_id = ? AND folder_path = ?",
    )
    .bind(game_id)
    .bind(folder_path)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(row) = existing {
        let id: String = row.try_get("id").map_err(|e| e.to_string())?;
        let existing_name: String = row.try_get("name").map_err(|e| e.to_string())?;
        let existing_type: String = row
            .try_get("object_type")
            .unwrap_or_else(|_| "Other".to_string());

        // Upgrade name + type when incoming data has richer info (thumbnail from MasterDB)
        if (existing_name != obj_name || existing_type != obj_type) && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET name = ?, object_type = ? WHERE id = ?")
                .bind(obj_name)
                .bind(obj_type)
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        let existing_thumb: Option<String> = row.try_get("thumbnail_path").unwrap_or(None);
        if existing_thumb.is_none() && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET thumbnail_path = ? WHERE id = ?")
                .bind(db_thumbnail)
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        let existing_tags: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
        if existing_tags == "[]" && db_tags_json != "[]" {
            sqlx::query("UPDATE objects SET tags = ? WHERE id = ?")
                .bind(db_tags_json)
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        let existing_meta: String = row.try_get("metadata").unwrap_or_else(|_| "{}".to_string());
        if existing_meta == "{}" && db_metadata_json != "{}" {
            sqlx::query("UPDATE objects SET metadata = ? WHERE id = ?")
                .bind(db_metadata_json)
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        return Ok(id);
    }

    let new_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type, thumbnail_path, tags, metadata) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
        .bind(&new_id)
        .bind(game_id)
        .bind(obj_name)
        .bind(folder_path)
        .bind(obj_type)
        .bind(db_thumbnail)
        .bind(db_tags_json)
        .bind(db_metadata_json)
        .execute(&mut **tx)
        .await
        .map_err(|e| e.to_string())?;

    *new_objects_count += 1;
    Ok(new_id)
}
