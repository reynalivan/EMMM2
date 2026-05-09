use crate::services::corridor_constants::{CORRIDOR_SOURCE_AUTO_TAGGED, CORRIDOR_SOURCE_UNKNOWN};
use crate::services::scanner::deep_matcher;
use sqlx::{Row, SqlitePool};
use std::path::Path;

/// Generate a deterministic mod ID from `game_id` + `relative_path`.
/// Uses BLAKE3 hash (first 32 hex chars) so the same folder always gets the same ID.
/// Per TRD §B.6 — replaces random UUID v4 for mod entries.
pub fn generate_stable_id(game_id: &str, folder_path: &str) -> String {
    let key = crate::services::path_key::folder_path_key(folder_path, None);
    let input = format!("{}:{}", game_id, key);
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

pub fn canonical_entry_key(entry_name: &str) -> String {
    crate::services::path_key::object_name_key(entry_name)
}

#[derive(Debug, Clone)]
pub struct ResolvedObjectTarget {
    pub object_id: String,
    pub folder_path: String,
}

pub struct ResolveObjectTargetInput<'a> {
    pub game_id: &'a str,
    pub mods_path: &'a str,
    pub physical_name_hint: &'a str,
    pub matched_entry_key: Option<&'a str>,
    pub object_type: &'a str,
    pub db_thumbnail: Option<&'a str>,
    pub db_tags_json: &'a str,
    pub db_metadata_json: &'a str,
    pub db_hash_db_json: Option<&'a str>,
    pub db_custom_skins_json: Option<&'a str>,
}

fn normalize_object_shell_name(physical_name_hint: &str) -> String {
    let normalized =
        crate::services::scanner::core::normalizer::normalize_display_name(physical_name_hint);
    let trimmed = normalized.trim();

    if trimmed.is_empty() {
        return "Imported Object".to_string();
    }

    trimmed.to_string()
}

async fn next_available_object_shell_name(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods_path: &str,
    base_name: &str,
) -> Result<String, String> {
    let mut suffix = 1_u32;

    loop {
        let candidate = if suffix == 1 {
            base_name.to_string()
        } else {
            format!("{base_name} ({suffix})")
        };

        let existing_object_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM objects WHERE game_id = ? AND folder_path_key = ? LIMIT 1",
        )
        .bind(game_id)
        .bind(crate::services::path_key::folder_path_key(&candidate, None))
        .fetch_optional(&mut *conn)
        .await
        .map_err(|error| error.to_string())?;

        let exists_on_disk = Path::new(mods_path).join(&candidate).exists();
        if existing_object_id.is_none() && !exists_on_disk {
            return Ok(candidate);
        }

        suffix += 1;
    }
}

pub async fn resolve_or_create_object_target_for_match(
    conn: &mut sqlx::SqliteConnection,
    input: ResolveObjectTargetInput<'_>,
    new_objects_count: &mut usize,
) -> Result<Option<ResolvedObjectTarget>, String> {
    let Some(entry_key) = input.matched_entry_key else {
        return Ok(None);
    };

    let existing_folder = crate::repo::object_repo::get_object_folder_by_matched_entry_key(
        &mut *conn,
        input.game_id,
        entry_key,
    )
    .await
    .map_err(|error| error.to_string())?;
    let existing_id = crate::repo::object_repo::get_object_id_by_matched_entry_key(
        &mut *conn,
        input.game_id,
        entry_key,
    )
    .await
    .map_err(|error| error.to_string())?;

    if let (Some(folder_path), Some(object_id)) = (existing_folder, existing_id) {
        return Ok(Some(ResolvedObjectTarget {
            object_id,
            folder_path,
        }));
    }

    let base_shell_name = normalize_object_shell_name(input.physical_name_hint);
    let shell_name = next_available_object_shell_name(
        &mut *conn,
        input.game_id,
        input.mods_path,
        &base_shell_name,
    )
    .await?;

    let object_id = ensure_object_exists(
        &mut *conn,
        crate::repo::object_repo::EnsureObjectInput {
            game_id: input.game_id,
            folder_path: &shell_name,
            obj_name: &shell_name,
            obj_type: input.object_type,
            db_thumbnail: input.db_thumbnail,
            db_tags_json: input.db_tags_json,
            db_metadata_json: input.db_metadata_json,
            db_hash_db_json: input.db_hash_db_json,
            db_custom_skins_json: input.db_custom_skins_json,
        },
        new_objects_count,
    )
    .await?;

    Ok(Some(ResolvedObjectTarget {
        object_id,
        folder_path: shell_name,
    }))
}

/// One-time startup migration: Stabilize mod IDs and path keys for mods, objects, and collection members.
///
/// This ensures that identities are stable even when folders alternate between enabled/disabled
/// (prefixing with `DISABLED `).
pub async fn migrate_to_stable_ids(pool: &SqlitePool) -> Result<usize, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let mut migrated = 0;

    // 1. Fetch ALL mods to check for ID or Key stability issues
    let mods = sqlx::query("SELECT id, game_id, folder_path, folder_path_key FROM mods")
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    for row in mods {
        let old_id: String = row.get(0);
        let game_id: String = row.get(1);
        let folder_path: String = row.get(2);
        let old_key: String = row.get(3);

        let new_key = crate::services::path_key::folder_path_key(&folder_path, None);
        let new_id = generate_stable_id(&game_id, &folder_path);

        if new_id != old_id || new_key != old_key {
            sqlx::query("UPDATE mods SET id = ?, folder_path_key = ? WHERE id = ?")
                .bind(&new_id)
                .bind(&new_key)
                .bind(&old_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            migrated += 1;
        }
    }

    // 2. Fetch ALL objects to stabilize their folder_path_key
    let objects = sqlx::query("SELECT id, folder_path, folder_path_key FROM objects")
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    for row in objects {
        let obj_id: String = row.get(0);
        let folder_path: String = row.get(1);
        let old_key: String = row.get(2);

        let new_key = crate::services::path_key::folder_path_key(&folder_path, None);
        if new_key != old_key {
            sqlx::query("UPDATE objects SET folder_path_key = ? WHERE id = ?")
                .bind(&new_key)
                .bind(&obj_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            migrated += 1;
        }
    }

    // 3. Fetch ALL collection mods to stabilize their mod_path_key (if using keys)
    // and potentially mod_path itself if it's stored as a key in some contexts.
    // In EMMM v2, collection_mods.mod_path is the relative path from mods root.
    let col_mods = sqlx::query("SELECT collection_id, mod_path FROM collection_mods")
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    for row in col_mods {
        let coll_id: String = row.get(0);
        let old_path: String = row.get(1);

        let new_key = crate::services::path_key::folder_path_key(&old_path, None);
        if new_key != old_path {
            // If mod_path was stored as a key or needs stabilization
            sqlx::query(
                "UPDATE collection_mods SET mod_path = ? WHERE collection_id = ? AND mod_path = ?",
            )
            .bind(&new_key)
            .bind(&coll_id)
            .bind(&old_path)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            migrated += 1;
        }
    }

    // 4. Fetch ALL collection roots to stabilize their root_path_key
    let roots = sqlx::query("SELECT collection_id, root_path_key FROM collection_roots")
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    for row in roots {
        let coll_id: String = row.get(0);
        let old_key: String = row.get(1);

        let new_key = crate::services::path_key::folder_path_key(&old_key, None);
        if new_key != old_key {
            sqlx::query("UPDATE collection_roots SET root_path_key = ? WHERE collection_id = ? AND root_path_key = ?")
                .bind(&new_key)
                .bind(&coll_id)
                .bind(&old_key)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            migrated += 1;
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    if migrated > 0 {
        log::info!("Stabilized {migrated} IDs and path keys for identity persistence");
    }

    Ok(migrated)
}

/// Upsert the game record into the `games` table so FK constraints are satisfied.
pub async fn ensure_game_exists(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    game_name: &str,
    game_type: &str,
    mods_path: &str,
) -> Result<(), String> {
    let parsed_game_type =
        std::str::FromStr::from_str(game_type).unwrap_or(crate::database::models::GameType::GIMI);
    crate::repo::game_repo::ensure_game_exists(
        conn,
        game_id,
        game_name,
        parsed_game_type,
        mods_path,
    )
    .await
    .map_err(|e| format!("Failed to ensure game exists: {e}"))?;
    Ok(())
}

pub async fn ensure_object_exists(
    conn: &mut sqlx::SqliteConnection,
    input: crate::repo::object_repo::EnsureObjectInput<'_>,
    new_objects_count: &mut usize,
) -> Result<String, String> {
    crate::repo::object_repo::ensure_object_exists(conn, input, new_objects_count).await
}

pub fn classify_corridor(
    display_name: &str,
    safe_mode_keywords: &[String],
) -> (bool, &'static str) {
    let folder_name_lower = display_name.to_lowercase();
    let keyword_match = safe_mode_keywords
        .iter()
        .any(|kw| folder_name_lower.contains(&kw.to_lowercase()));

    if keyword_match {
        return (false, CORRIDOR_SOURCE_AUTO_TAGGED);
    }

    (true, CORRIDOR_SOURCE_UNKNOWN)
}
