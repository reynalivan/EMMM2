//! Folder entry listing service for the Scan Review tooltip.
//!
//! Resolves the game's mods_path from the DB and reads the filesystem to
//! return a bounded list of child entries from a given directory.

use serde::Serialize;

/// A single item in a folder listing (used by the Scan Review hover tooltip).
#[derive(Debug, Clone, Serialize)]
pub struct FolderEntry {
    pub name: String,
    pub is_dir: bool,
}

/// List the immediate children (up to 50) of `folder_path` after verifying it
/// is safe (within the game's mods root) and resolving the game's mods_path from DB.
pub async fn list_folder_entries(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    folder_path: &str,
) -> Result<Vec<FolderEntry>, String> {
    use std::path::Path;

    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Failed to fetch game mods path".to_string())?;

    let base = Path::new(&mods_path);
    let path = Path::new(folder_path);

    if !crate::services::fs_utils::path_utils::is_path_safe(base, path) {
        return Err("Path attempts to escape mods directory bounds".to_string());
    }

    if !path.is_dir() {
        return Err(format!("Not a directory: {}", folder_path));
    }

    let read_dir = std::fs::read_dir(path).map_err(|e| format!("Cannot read directory: {}", e))?;

    let mut entries: Vec<FolderEntry> = read_dir
        .filter_map(|e| e.ok())
        .map(|e| FolderEntry {
            name: e.file_name().to_string_lossy().to_string(),
            is_dir: e.file_type().map(|ft| ft.is_dir()).unwrap_or(false),
        })
        .take(50)
        .collect();

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));

    Ok(entries)
}
