use crate::repo::dedup_repo::{delete_whitelist_entry, get_whitelist_detailed, WhitelistEntry};
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn get_ignored_pairs(
    game_id: String,
    db: State<'_, sqlx::SqlitePool>,
) -> Result<Vec<WhitelistEntry>, String> {
    get_whitelist_detailed(db.inner(), &game_id)
        .await
        .map_err(|e| format!("Failed to fetch ignored pairs: {}", e))
}

#[tauri::command]
#[specta::specta]
pub async fn remove_ignored_pair(
    entry_id: String,
    db: State<'_, sqlx::SqlitePool>,
) -> Result<u64, String> {
    delete_whitelist_entry(db.inner(), &entry_id)
        .await
        .map_err(|e| format!("Failed to remove ignored pair: {}", e))
}
