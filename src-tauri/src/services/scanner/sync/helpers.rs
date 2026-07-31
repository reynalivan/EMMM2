use crate::common::corridor_constants::{CORRIDOR_SOURCE_AUTO_TAGGED, CORRIDOR_SOURCE_UNKNOWN};
use crate::services::scanner::deep_matcher;
use std::path::Path;

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
    crate::common::path_key::canonical_name_key(entry_name)
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
    let normalized = crate::common::normalizer::normalize_display_name(physical_name_hint);
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

        let existing_object_id = crate::repo::object_repo::get_object_id_by_folder_key(
            conn,
            game_id,
            &crate::common::path_key::folder_path_key(&candidate, None),
        )
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

/// Upsert the game record into the `games` table so FK constraints are satisfied.
pub async fn ensure_game_exists(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    game_name: &str,
    game_type: &str,
    mods_path: &str,
) -> Result<(), String> {
    let parsed_game_type =
        std::str::FromStr::from_str(game_type).unwrap_or(crate::domain::models::GameType::GIMI);
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
    crate::repo::object_repo::ensure_object_exists(conn, input, new_objects_count)
        .await
        .map_err(|e| e.to_string())
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
