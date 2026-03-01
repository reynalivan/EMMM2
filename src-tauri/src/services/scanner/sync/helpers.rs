use crate::services::scanner::deep_matcher;
use sqlx::SqlitePool;

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
    let rows = crate::database::mod_repo::get_mods_with_uuid_format(pool)
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

        crate::database::collection_repo::update_collection_item_mod_id_global(
            &mut *tx, old_id, &new_id,
        )
        .await
        .map_err(|e| e.to_string())?;

        crate::database::mod_repo::update_mod_id(&mut *tx, old_id, &new_id)
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
    crate::database::game_repo::ensure_game_exists(
        &mut **tx, game_id, game_name, game_type, mods_path,
    )
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
    crate::database::object_repo::ensure_object_exists(
        tx,
        game_id,
        folder_path,
        obj_name,
        obj_type,
        db_thumbnail,
        db_tags_json,
        db_metadata_json,
        new_objects_count,
    )
    .await
}
