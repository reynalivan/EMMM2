use crate::domain::errors::AppError;
use crate::services::game::schema_loader;
use tauri::Manager;

/// The bundled-resources directory, or a typed error.
///
/// Five commands re-derived this, and `search_master_db` did it twice in one
/// body so a cache hit still paid a second lookup.
fn resource_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    app.path()
        .resource_dir()
        .map_err(|error| AppError::Internal(format!("Failed to get resource dir: {error}")))
}

/// Get the game schema (categories + filters) for a specific game type.
/// Falls back to default [Character, Weapon, UI, Other] if schema.json is missing/corrupt.
///
/// Covers: NC-3.4-02 (Schema Load Failure → fallback)
#[tauri::command]
#[specta::specta]
pub async fn get_game_schema(
    app: tauri::AppHandle,
    game_type: i32,
) -> Result<schema_loader::GameSchema, AppError> {
    let resource_dir = resource_dir(&app)?;

    log::info!("get_game_schema: resource_dir = {}", resource_dir.display());

    let schema = schema_loader::load_schema(&resource_dir, game_type);
    Ok(schema)
}

/// Get a single object by ID (full details including metadata).
#[tauri::command]
#[specta::specta]
pub async fn get_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    id: String,
) -> Result<Option<crate::services::scanner::core::types::GameObject>, AppError> {
    let row = crate::services::objects::query::get_object_by_id_service(&pool, &id).await?;
    Ok(row)
}

/// Get the MasterDB JSON for a specific game type.
/// Loads from `resources/databases/{game_type}.json`.
/// Returns array JSON for frontend compatibility (even if file uses new object format).
/// When hash_db is present in source, merges hashes into matching entries.
#[tauri::command]
#[specta::specta]
pub async fn get_master_db(app: tauri::AppHandle, game_type: i32) -> Result<String, AppError> {
    let resource_dir = resource_dir(&app)?;
    Ok(crate::services::scanner::master_db::load_master_db_json(
        &resource_dir,
        game_type,
    )?)
}

/// Pin or unpin an object in the database.
#[tauri::command]
#[specta::specta]
pub async fn pin_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    id: String,
    pin: bool,
) -> Result<(), AppError> {
    crate::services::objects::mutate::toggle_pin_object(pool.inner(), &id, pin).await
}

/// Match a single object name against the MasterDB for a specific game.
/// Uses staged quick matcher semantics and adapter labels.
///
/// This is used for the "Sync with DB" context menu action on individual objects/folders.
#[tauri::command]
#[specta::specta]
pub async fn match_object_with_db(
    app: tauri::AppHandle,
    game_type: i32,
    object_name: String,
) -> Result<Option<crate::services::scanner::master_db::MatchedDbEntry>, AppError> {
    let resource_dir = resource_dir(&app)?;

    Ok(
        crate::services::scanner::master_db::match_object_with_db_service(
            &resource_dir,
            game_type,
            &object_name,
        )?,
    )
}

/// Search Master DB from Rust to offload fuzzy matching from the JS thread.
/// Finds the top results matching `query`, optionally filtering by `object_type`.
#[tauri::command]
#[specta::specta]
pub async fn search_master_db(
    app: tauri::AppHandle,
    game_type: i32,
    query: String,
    object_type: Option<String>,
) -> Result<Vec<crate::services::scanner::master_db::SearchResultEntry>, AppError> {
    let Some(db) = crate::services::scanner::master_db::get_cached(&app, game_type).await? else {
        return Ok(Vec::new());
    };

    let resource_dir = resource_dir(&app)?;

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
