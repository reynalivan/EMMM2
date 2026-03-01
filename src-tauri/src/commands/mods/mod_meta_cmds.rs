use crate::services::mods::{info_json, metadata};
use std::path::Path;

#[tauri::command]
pub async fn repair_orphan_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<usize, String> {
    metadata::repair_orphan_mods(pool.inner(), &game_id).await
}

#[tauri::command]
pub async fn pin_mod(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    id: String,
    pin: bool,
) -> Result<(), String> {
    metadata::toggle_pin(pool.inner(), &id, pin).await
}

#[tauri::command]
pub async fn toggle_favorite(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_path: String,
    favorite: bool,
) -> Result<(), String> {
    metadata::toggle_favorite(pool.inner(), &game_id, &folder_path, favorite).await
}

#[tauri::command]
pub async fn toggle_mod_safe(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_path: String,
    safe: bool,
) -> Result<(), String> {
    metadata::toggle_mod_safe(pool.inner(), &game_id, &folder_path, safe).await
}

#[tauri::command]
pub async fn suggest_random_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    is_safe: bool,
) -> Result<Vec<metadata::RandomModProposal>, String> {
    metadata::suggest_random_mods(pool.inner(), &game_id, is_safe).await
}

#[tauri::command]
pub async fn get_active_mod_conflicts(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<crate::services::scanner::conflict::ConflictInfo>, String> {
    metadata::get_active_mod_conflicts(pool.inner(), &game_id).await
}

#[tauri::command]
pub async fn read_mod_info(folder_path: String) -> Result<Option<info_json::ModInfo>, String> {
    info_json::read_info_json(Path::new(&folder_path))
}

#[tauri::command]
pub async fn update_mod_info(
    folder_path: String,
    update: info_json::ModInfoUpdate,
) -> Result<info_json::ModInfo, String> {
    info_json::update_info_json(Path::new(&folder_path), &update)
}

#[tauri::command]
pub async fn set_mod_category(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_path: String,
    category: String,
) -> Result<(), String> {
    metadata::set_mod_category(&pool, &game_id, &folder_path, &category).await
}

#[tauri::command]
pub async fn move_mod_to_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_path: String,
    target_object_id: String,
    status: Option<String>,
) -> Result<(), String> {
    crate::services::mods::organizer_ext::move_mod_to_object_service(
        pool.inner(),
        &game_id,
        &folder_path,
        &target_object_id,
        status.as_deref(),
    )
    .await
}

#[cfg(test)]
#[path = "tests/mod_meta_cmds_tests.rs"]
mod tests;
