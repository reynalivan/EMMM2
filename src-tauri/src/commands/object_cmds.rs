use crate::services::scanner::deep_matcher;
use crate::services::scanner::normalizer;
use crate::services::schema_loader;
use serde::Serialize;
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

/// Match a single object name against the MasterDB for a specific game.
/// Uses the full deep matcher pipeline: L0 Skin Alias → L1 Name → L2 Token → L5 Fuzzy.
/// (L3 Content and L4 AI are skipped as they require disk scanning.)
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

    // Clean name using the same normalizer as the deep matcher pipeline
    let clean_name = normalizer::strip_noise_prefixes(&object_name);

    // Run matching pipeline: L0 → L1 → L2 → L5 (same order as match_folder, minus L3/L4)
    let match_result = deep_matcher::skin_alias_match(&clean_name, &db)
        .or_else(|| deep_matcher::name_match(&clean_name, &db))
        .or_else(|| deep_matcher::token_match(&clean_name, &db))
        .or_else(|| deep_matcher::fuzzy_match(&clean_name, &db));

    match match_result {
        Some(result) => {
            // Find the full DbEntry to get metadata + thumbnail
            let entry = db.entries.iter().find(|e| e.name == result.object_name);

            match entry {
                Some(entry) => {
                    // Resolve thumbnail to absolute path
                    let resolved_thumbnail = entry.thumbnail_path.as_ref().and_then(|rel_path| {
                        let abs_path = resource_dir.join(rel_path);
                        if abs_path.exists() {
                            abs_path.to_str().map(|s| s.to_string())
                        } else {
                            log::warn!(
                                "Thumbnail not found for {}: {}",
                                entry.name,
                                abs_path.display()
                            );
                            None
                        }
                    });

                    Ok(Some(MatchedDbEntry {
                        name: entry.name.clone(),
                        object_type: entry.object_type.clone(),
                        tags: entry.tags.clone(),
                        metadata: entry.metadata.clone(),
                        thumbnail_path: resolved_thumbnail,
                        match_level: result.level.to_string(),
                        match_confidence: result.confidence.to_string(),
                        match_detail: result.detail.clone(),
                    }))
                }
                None => Ok(None),
            }
        }
        None => Ok(None),
    }
}
