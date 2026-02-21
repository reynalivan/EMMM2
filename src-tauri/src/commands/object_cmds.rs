use crate::services::scanner::deep_matcher;
use crate::services::scanner::deep_matcher::content::IniTokenizationConfig;
use crate::services::scanner::types;
use crate::services::scanner::walker::{FolderContent, ModCandidate};
use crate::services::schema_loader;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::Manager;

/// Get the game schema (categories + filters) for a specific game type.
/// Falls back to default [Character, Weapon, UI, Other] if schema.json is missing/corrupt.
///
/// Covers: NC-3.4-02 (Schema Load Failure → fallback)
#[tauri::command]
pub async fn get_game_schema(
    app: tauri::AppHandle,
    game_type: String,
) -> Result<schema_loader::GameSchema, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {e}"))?;

    log::info!("get_game_schema: resource_dir = {}", resource_dir.display());

    let schema = schema_loader::load_schema(&resource_dir, &game_type);
    Ok(schema)
}

/// Get a single object by ID (full details including metadata).
#[tauri::command]
pub async fn get_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    id: String,
) -> Result<Option<crate::services::scanner::types::GameObject>, String> {
    let row = sqlx::query_as::<_, crate::services::scanner::types::GameObject>(
        "SELECT * FROM objects WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row)
}

/// Get the MasterDB JSON for a specific game type.
/// Loads from `resources/databases/{game_type}.json`.
/// Returns empty structure if file not found (with warning).
#[tauri::command]
pub async fn get_master_db(app: tauri::AppHandle, game_type: String) -> Result<String, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {e}"))?;

    let canonical = schema_loader::normalize_game_type(&game_type);
    let db_path = resource_dir
        .join("databases")
        .join(format!("{}.json", canonical));

    if !db_path.exists() {
        log::warn!(
            "MasterDB not found for {}: {}",
            game_type,
            db_path.display()
        );
        return Ok("[]".to_string());
    }

    let json_content =
        std::fs::read_to_string(&db_path).map_err(|e| format!("Failed to read MasterDB: {e}"))?;

    // Parse JSON to modify thumbnail paths
    let mut entries: Vec<serde_json::Value> = serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse MasterDB JSON: {e}"))?;

    for entry in &mut entries {
        // Resolve thumbnail_path to absolute
        if let Some(thumb_rel) = entry.get("thumbnail_path").and_then(|v| v.as_str()) {
            let abs_path = resource_dir.join(thumb_rel);
            if let Some(abs_str) = abs_path.to_str() {
                entry["thumbnail_path"] = serde_json::Value::String(abs_str.to_string());
            }
        }

        // Also resolve custom_skins thumbnails
        if let Some(skins) = entry.get_mut("custom_skins").and_then(|v| v.as_array_mut()) {
            for skin in skins {
                if let Some(skin_thumb_rel) =
                    skin.get("thumbnail_skin_path").and_then(|v| v.as_str())
                {
                    let abs_path = resource_dir.join(skin_thumb_rel);
                    if let Some(abs_str) = abs_path.to_str() {
                        skin["thumbnail_skin_path"] =
                            serde_json::Value::String(abs_str.to_string());
                    }
                }
            }
        }
    }

    serde_json::to_string(&entries).map_err(|e| format!("Failed to serialize MasterDB: {e}"))
}

/// Pin or unpin an object in the database.
#[tauri::command]
pub async fn pin_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    id: String,
    pin: bool,
) -> Result<(), String> {
    sqlx::query("UPDATE objects SET is_pinned = ? WHERE id = ?")
        .bind(pin)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete an object by ID.
#[tauri::command]
pub async fn delete_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM objects WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Matched DB entry returned to frontend with resolved absolute thumbnail path.
#[derive(Debug, Clone, Serialize)]
pub struct MatchedDbEntry {
    pub name: String,
    pub object_type: String,
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
    pub thumbnail_path: Option<String>,
    /// Which pipeline level produced the match (e.g. "L1Name", "L2Token", "L5Fuzzy")
    pub match_level: String,
    /// Confidence of the match (e.g. "High", "Medium", "Low")
    pub match_confidence: String,
    /// Human-readable detail about how the match was found
    pub match_detail: String,
}

fn match_object_with_staged_pipeline(
    db: &deep_matcher::MasterDb,
    object_name: &str,
) -> deep_matcher::StagedMatchResult {
    let candidate = ModCandidate {
        path: PathBuf::from(object_name),
        raw_name: object_name.to_string(),
        display_name: object_name.to_string(),
        is_disabled: false,
    };
    let content = FolderContent {
        subfolder_names: Vec::new(),
        files: Vec::new(),
        ini_files: Vec::new(),
    };
    let ini_config = IniTokenizationConfig::default();

    deep_matcher::match_folder_quick(&candidate, db, &content, &ini_config)
}

fn resolve_thumbnail_path(resource_dir: &Path, entry: &deep_matcher::DbEntry) -> Option<String> {
    let rel_path = entry.thumbnail_path.as_ref()?;
    let abs_path = resource_dir.join(rel_path);

    if !abs_path.exists() {
        log::warn!("Thumbnail not found for {}: {}", entry.name, abs_path.display());
        return None;
    }

    abs_path.to_str().map(|path| path.to_string())
}

fn build_matched_db_entry_from_staged(
    resource_dir: &Path,
    db: &deep_matcher::MasterDb,
    match_result: &deep_matcher::StagedMatchResult,
) -> Option<MatchedDbEntry> {
    let candidate = types::staged_primary_candidate(match_result)?;
    let entry = db.entries.get(candidate.entry_id)?;

    Some(MatchedDbEntry {
        name: entry.name.clone(),
        object_type: entry.object_type.clone(),
        tags: entry.tags.clone(),
        metadata: entry.metadata.clone(),
        thumbnail_path: resolve_thumbnail_path(resource_dir, entry),
        match_level: types::match_status_label(&match_result.status).to_string(),
        match_confidence: types::staged_confidence_label(match_result).to_string(),
        match_detail: types::staged_match_detail(match_result),
    })
}

/// Match a single object name against the MasterDB for a specific game.
/// Uses staged quick matcher semantics and adapter labels.
///
/// This is used for the "Sync with DB" context menu action on individual objects/folders.
#[tauri::command]
pub async fn match_object_with_db(
    app: tauri::AppHandle,
    game_type: String,
    object_name: String,
) -> Result<Option<MatchedDbEntry>, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {e}"))?;

    // Load MasterDB (normalize legacy game_type → canonical XXMI code)
    let canonical = schema_loader::normalize_game_type(&game_type);
    let db_path = resource_dir
        .join("databases")
        .join(format!("{}.json", canonical));

    if !db_path.exists() {
        return Ok(None);
    }

    let json =
        std::fs::read_to_string(&db_path).map_err(|e| format!("Failed to read MasterDB: {e}"))?;

    let db = deep_matcher::MasterDb::from_json(&json)?;

    let match_result = match_object_with_staged_pipeline(&db, &object_name);
    Ok(build_matched_db_entry_from_staged(
        &resource_dir,
        &db,
        &match_result,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::scanner::deep_matcher::{CustomSkin, DbEntry, MatchStatus, MasterDb};
    use tempfile::TempDir;

    fn build_entry(name: &str, tags: &[&str], aliases: &[&str]) -> DbEntry {
        let custom_skins = if aliases.is_empty() {
            Vec::new()
        } else {
            vec![CustomSkin {
                name: aliases[0].to_string(),
                aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
                thumbnail_skin_path: None,
                rarity: None,
            }]
        };

        DbEntry {
            name: name.to_string(),
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            object_type: "Character".to_string(),
            custom_skins,
            thumbnail_path: None,
            metadata: None,
            hashes: Vec::new(),
        }
    }

    #[test]
    fn test_object_command_auto_matched_uses_staged_status_adapters() {
        let db = MasterDb::new(vec![build_entry(
            "Raiden Shogun",
            &["raiden"],
            &["Raiden Wish"],
        )]);
        let match_result = match_object_with_staged_pipeline(&db, "Raiden Wish");
        let temp = TempDir::new().expect("temp dir");

        let item = build_matched_db_entry_from_staged(temp.path(), &db, &match_result)
            .expect("expected staged auto match payload");

        assert_eq!(match_result.status, MatchStatus::AutoMatched);
        assert_eq!(item.name, "Raiden Shogun");
        assert_eq!(item.match_level, "AutoMatched");
        assert_eq!(item.match_confidence, "High");
        assert_eq!(item.match_detail, "Matched by AliasStrict");
    }

    #[test]
    fn test_object_command_needs_review_keeps_candidate_with_low_confidence_label() {
        let db = MasterDb::new(vec![
            build_entry("Amber", &["sunset"], &["Sunset"]),
            build_entry("Lisa", &["sunset"], &["Sunset"]),
        ]);
        let match_result = match_object_with_staged_pipeline(&db, "Sunset Pack");
        let temp = TempDir::new().expect("temp dir");

        let item = build_matched_db_entry_from_staged(temp.path(), &db, &match_result)
            .expect("expected review payload");

        assert_eq!(match_result.status, MatchStatus::NeedsReview);
        assert_eq!(item.match_level, "NeedsReview");
        assert_eq!(item.match_confidence, "Low");
        assert_eq!(item.match_detail, "Ambiguous: Amber vs Lisa");
    }

    #[test]
    fn test_object_command_has_no_active_fuzzy_fallback() {
        let db = MasterDb::new(vec![build_entry("Albedo", &[], &[])]);
        let match_result = match_object_with_staged_pipeline(&db, "Albato");
        let temp = TempDir::new().expect("temp dir");

        assert_eq!(match_result.status, MatchStatus::NoMatch);
        assert!(build_matched_db_entry_from_staged(temp.path(), &db, &match_result).is_none());
    }
}
