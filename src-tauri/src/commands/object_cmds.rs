use crate::services::schema_loader;
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

    let schema = schema_loader::load_schema(&resource_dir, &game_type);
    Ok(schema)
}
