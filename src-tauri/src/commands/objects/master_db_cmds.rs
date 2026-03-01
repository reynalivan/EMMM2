use crate::services::game::schema_loader;
use crate::services::scanner::deep_matcher;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

/// Cache for MasterDB to avoid parsing 5MB JSON on every keystroke
pub struct MasterDbCache(pub RwLock<HashMap<String, Arc<deep_matcher::MasterDb>>>);

impl Default for MasterDbCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MasterDbCache {
    pub fn new() -> Self {
        Self(RwLock::new(HashMap::new()))
    }
}

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
) -> Result<Option<crate::services::scanner::core::types::GameObject>, String> {
    let row = crate::services::objects::query::get_object_by_id_service(&*pool, &id).await?;
    Ok(row)
}

/// Get the MasterDB JSON for a specific game type.
/// Loads from `resources/databases/{game_type}.json`.
/// Returns array JSON for frontend compatibility (even if file uses new object format).
/// When hash_db is present in source, merges hashes into matching entries.
#[tauri::command]
pub async fn get_master_db(app: tauri::AppHandle, game_type: String) -> Result<String, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {e}"))?;
    crate::services::scanner::master_db::load_master_db_json(&resource_dir, &game_type)
}

/// Pin or unpin an object in the database.
#[tauri::command]
pub async fn pin_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    id: String,
    pin: bool,
) -> Result<(), String> {
    crate::services::objects::mutate::toggle_pin_object(pool.inner(), &id, pin).await
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
) -> Result<Option<crate::services::scanner::master_db::MatchedDbEntry>, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {e}"))?;

    crate::services::scanner::master_db::match_object_with_db_service(
        &resource_dir,
        &game_type,
        &object_name,
    )
}

/// Search Master DB from Rust to offload fuzzy matching from the JS thread.
/// Finds the top results matching `query`, optionally filtering by `object_type`.
#[tauri::command]
pub async fn search_master_db(
    app: tauri::AppHandle,
    cache: tauri::State<'_, MasterDbCache>,
    game_type: String,
    query: String,
    object_type: Option<String>,
) -> Result<Vec<crate::services::scanner::master_db::SearchResultEntry>, String> {
    let canonical = schema_loader::normalize_game_type(&game_type);

    // 1. Try to get from cache
    let db = {
        let lock = cache.0.read().await;
        lock.get(&canonical).cloned()
    };

    // 2. Load from disk if not in cache
    let db = match db {
        Some(cached) => cached,
        None => {
            let resource_dir = app
                .path()
                .resource_dir()
                .map_err(|e| format!("Failed to get resource dir: {e}"))?;

            let db_path = resource_dir
                .join("databases")
                .join(format!("{}.json", canonical));

            if !db_path.exists() {
                return Ok(Vec::new());
            }

            let json = std::fs::read_to_string(&db_path)
                .map_err(|e| format!("Failed to read MasterDB for search: {e}"))?;

            let parsed_db = deep_matcher::MasterDb::from_json(&json)?;
            let arc_db = Arc::new(parsed_db);

            // Re-acquire lock for writing
            let mut write_lock = cache.0.write().await;
            write_lock.insert(canonical.clone(), arc_db.clone());
            arc_db
        }
    };

    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {e}"))?;

    Ok(
        crate::services::scanner::master_db::search_master_db_service(
            &db,
            &resource_dir,
            &query,
            object_type.as_deref(),
        ),
    )
}

#[cfg(test)]
#[path = "tests/master_db_cmds_tests.rs"]
mod tests;
