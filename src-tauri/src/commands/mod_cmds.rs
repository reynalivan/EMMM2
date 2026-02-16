use crate::services::file_ops::archive::{extract_archive, ArchiveFormat};
use crate::services::file_ops::trash;
use crate::services::operation_lock::OperationLock;
use crate::services::watcher::SuppressionGuard;
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
struct BulkProgressPayload {
    label: String,
    current: usize,
    total: usize,
    active: bool,
}

use crate::DISABLED_PREFIX;

/// Regex for matching messy disabled prefixes.
/// Handles: disabled_, DISABLED-, Disable , dis_, DIS-, etc.
/// Covers: EC-5.06 (Bad Prefix Fix)
static DISABLED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(disabled|disable|dis)[_\-\s]*").unwrap());

/// Standardize a folder name by cleaning up messy disabled prefixes.
/// Returns the clean name (enabled) or with standard "DISABLED " prefix.
///
/// - `folder_name`: The current folder name (may have messy prefix).
/// - `target_enabled`: true = remove prefix (enable), false = add prefix (disable).
///
/// # Covers: EC-5.06
pub fn standardize_prefix(folder_name: &str, target_enabled: bool) -> String {
    // 1. Clean any existing disabled prefix variant
    let clean_name = DISABLED_RE.replace(folder_name, "").trim().to_string();

    // 2. If clean_name is empty (edge case: folder was literally just "disabled")
    let clean_name = if clean_name.is_empty() {
        folder_name.to_string()
    } else {
        clean_name
    };

    if target_enabled {
        clean_name
    } else {
        format!("DISABLED {clean_name}")
    }
}

/// Open a folder in the system file explorer.
/// Covers: DI-4.04
#[tauri::command]
pub async fn open_in_explorer(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path) // Use reference to avoid move if needed, though String is fine
            .spawn()
            .map_err(|e| format!("Failed to open explorer: {}", e))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Fallback for dev on non-Windows
        return Err("Open in explorer only supported on Windows".to_string());
    }
    Ok(())
}

use crate::services::watcher::WatcherState;
use tauri::State;

/// Toggle a mod enabled/disabled by renaming the folder.
/// Adds or removes "DISABLED " prefix with regex standardization.
/// Acquires OperationLock to prevent concurrent destructive operations.
/// Covers: TC-5.1-01, EC-5.06, NC-5.1-04
#[tauri::command]
pub async fn toggle_mod(
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    path: String,
    enable: bool,
) -> Result<String, String> {
    let _lock = op_lock.acquire().await?;
    toggle_mod_inner(&state, path, enable).await
}

pub async fn toggle_mod_inner(
    state: &WatcherState,
    path: String,
    enable: bool,
) -> Result<String, String> {
    let path_obj = Path::new(&path);
    if !path_obj.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let parent = path_obj.parent().ok_or("Invalid path parent")?;
    let file_name = path_obj
        .file_name()
        .ok_or("Invalid file name")?
        .to_string_lossy()
        .to_string();

    // Use regex standardization to handle messy prefixes
    let new_name = standardize_prefix(&file_name, enable);

    // No-op if name unchanged
    if new_name == file_name {
        return Ok(path);
    }

    let new_path = parent.join(&new_name);

    // Check if target exists (collision) — case-insensitive on Windows
    if new_path.exists() && file_name.to_lowercase() != new_name.to_lowercase() {
        return Err(format!(
            "Target path already exists: {}",
            new_path.display()
        ));
    }

    // Suppress watcher during rename
    {
        let _guard = SuppressionGuard::new(&state.suppressor);
        fs::rename(&path, &new_path).map_err(|e| format!("Failed to rename: {}", e))?;
    }

    Ok(new_path.to_string_lossy().to_string())
}

/// Move a mod to the trash.
/// Covers: TC-4.5-01, NC-5.1-04
#[tauri::command]
pub async fn delete_mod(
    app: AppHandle,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    path: String,
    game_id: Option<String>,
) -> Result<(), String> {
    let _lock = op_lock.acquire().await?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let trash_dir = app_data_dir.join("trash");

    // Ensure trash dir exists
    if !trash_dir.exists() {
        fs::create_dir_all(&trash_dir).map_err(|e| format!("Failed to create trash dir: {}", e))?;
    }

    delete_mod_inner(&state, &trash_dir, path, game_id).await
}

pub async fn delete_mod_inner(
    state: &WatcherState,
    trash_dir: &Path,
    path: String,
    game_id: Option<String>,
) -> Result<(), String> {
    let path_obj = Path::new(&path);
    // Suppress watcher during move
    {
        let _guard = SuppressionGuard::new(&state.suppressor);
        trash::move_to_trash(path_obj, trash_dir, game_id).map(|_| ())
    }
}

/// Pin or unpin a mod in the database.
#[tauri::command]
pub async fn pin_mod(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    id: String,
    pin: bool,
) -> Result<(), String> {
    sqlx::query("UPDATE mods SET is_pinned = ? WHERE folder_path = ?")
        .bind(pin)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Epic 4 additions ─────────────────────────────────────────────

use crate::services::file_ops::info_json;

/// Result of a rename operation.
#[derive(Debug, Clone, Serialize)]
pub struct RenameResult {
    pub old_path: String,
    pub new_path: String,
    pub new_name: String,
}

/// Rename a mod folder on disk.
/// Preserves the DISABLED prefix if the mod is disabled.
/// Covers: NC-4.1-03 (conflict detection), NC-5.1-04
#[tauri::command]
pub async fn rename_mod_folder(
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    folder_path: String,
    new_name: String,
) -> Result<RenameResult, String> {
    let _lock = op_lock.acquire().await?;
    rename_mod_folder_inner(&state, folder_path, new_name).await
}

pub async fn rename_mod_folder_inner(
    state: &WatcherState,
    folder_path: String,
    new_name: String,
) -> Result<RenameResult, String> {
    let path = Path::new(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Folder does not exist: {folder_path}"));
    }

    if new_name.is_empty() || new_name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("Invalid folder name — contains reserved characters".to_string());
    }

    let parent = path.parent().ok_or("Cannot determine parent directory")?;
    let old_folder_name = path
        .file_name()
        .ok_or("Invalid folder name")?
        .to_string_lossy()
        .to_string();

    // Preserve DISABLED prefix if the mod is currently disabled
    let new_folder_name = if old_folder_name.starts_with(DISABLED_PREFIX) {
        format!("{DISABLED_PREFIX}{new_name}")
    } else {
        new_name.clone()
    };

    let new_path = parent.join(&new_folder_name);
    if new_path.exists() {
        return Err(format!(
            "A folder named '{}' already exists",
            new_folder_name
        ));
    }

    // Suppress watcher
    {
        let _guard = SuppressionGuard::new(&state.suppressor);
        fs::rename(path, &new_path).map_err(|e| format!("Failed to rename folder: {e}"))?;
    }

    // Update info.json if it exists
    if new_path.join("info.json").exists() {
        let update = info_json::ModInfoUpdate {
            actual_name: Some(new_name.clone()),
            ..Default::default()
        };
        let _ = info_json::update_info_json(&new_path, &update);
    }

    log::info!("Renamed '{}' -> '{}'", old_folder_name, new_folder_name);

    Ok(RenameResult {
        old_path: folder_path,
        new_path: new_path.to_string_lossy().to_string(),
        new_name,
    })
}

/// Restore a mod from trash to its original location.
#[tauri::command]
pub async fn restore_mod(app: AppHandle, trash_id: String) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    let trash_dir = app_data_dir.join("trash");
    trash::restore_from_trash(&trash_id, &trash_dir)
}

/// List all items currently in the trash.
#[tauri::command]
pub async fn list_trash(app: AppHandle) -> Result<Vec<trash::TrashMetadata>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    let trash_dir = app_data_dir.join("trash");
    trash::list_trash(&trash_dir)
}

/// Permanently delete all items in the trash.
/// Returns the number of entries removed.
/// Covers: US-4.4 (Empty Trash)
#[tauri::command]
pub async fn empty_trash(app: AppHandle) -> Result<u64, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    let trash_dir = app_data_dir.join("trash");
    trash::empty_trash(&trash_dir)
}

/// Read info.json from a mod folder.
#[tauri::command]
pub async fn read_mod_info(folder_path: String) -> Result<Option<info_json::ModInfo>, String> {
    info_json::read_info_json(Path::new(&folder_path))
}

/// Update info.json in a mod folder (partial merge — only specified fields change).
#[tauri::command]
pub async fn update_mod_info(
    folder_path: String,
    update: info_json::ModInfoUpdate,
) -> Result<info_json::ModInfo, String> {
    info_json::update_info_json(Path::new(&folder_path), &update)
}

/// Set the category (Object Type) for a mod.
#[tauri::command]
pub async fn set_mod_category(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_path: String,
    category: String,
) -> Result<(), String> {
    use crate::services::file_ops::metadata;
    metadata::set_mod_category(&pool, &game_id, &folder_path, &category).await
}

/// Move a mod to a different object by updating its object_id in the DB.
#[tauri::command]
pub async fn move_mod_to_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    mod_id: String,
    target_object_id: String,
    status: Option<String>,
) -> Result<(), String> {
    // Verify the target object exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM objects WHERE id = ?)")
        .bind(&target_object_id)
        .fetch_one(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    if !exists {
        return Err(format!("Target object does not exist: {target_object_id}"));
    }

    // Move mod to new object
    sqlx::query("UPDATE mods SET object_id = ? WHERE id = ?")
        .bind(&target_object_id)
        .bind(&mod_id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    // Handle status logic
    if let Some(ref status) = status {
        match status.as_str() {
            "disabled" => {
                sqlx::query("UPDATE mods SET is_enabled = 0 WHERE id = ?")
                    .bind(&mod_id)
                    .execute(&*pool)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            "only-enable" => {
                // Disable all other mods in this object
                sqlx::query("UPDATE mods SET is_enabled = 0 WHERE object_id = ? AND id != ?")
                    .bind(&target_object_id)
                    .bind(&mod_id)
                    .execute(&*pool)
                    .await
                    .map_err(|e| e.to_string())?;
                // Enable this mod
                sqlx::query("UPDATE mods SET is_enabled = 1 WHERE id = ?")
                    .bind(&mod_id)
                    .execute(&*pool)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            "keep" => {
                // Do nothing
            }
            _ => {}
        }
    }

    log::info!("Moved mod {} to object {} (status: {:?})", mod_id, target_object_id, status);
    Ok(())
}

#[tauri::command]
pub async fn update_mod_thumbnail(
    folder_path: String,
    source_path: String,
) -> Result<String, String> {
    use crate::services::file_ops::metadata;
    metadata::update_mod_thumbnail(&folder_path, &source_path)
}

/// Get the discovered thumbnail path for a mod folder.
#[tauri::command]
pub async fn get_thumbnail(folder_path: String) -> Result<Option<String>, String> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    use crate::services::scanner::thumbnail::find_thumbnail;

    let path = Path::new(&folder_path);
    if !path.exists() {
        return Err(format!("Path does not exist: {folder_path}"));
    }

    if let Some(original) = find_thumbnail(path) {
        match ThumbnailCache::get_thumbnail(&original) {
            Ok(cached) => Ok(Some(cached.to_string_lossy().to_string())),
            Err(e) => {
                log::warn!("Thumbnail cache failed: {}", e);
                // Fallback to original
                Ok(Some(original.to_string_lossy().to_string()))
            }
        }
    } else {
        Ok(None)
    }
}

/// Save a thumbnail from base64/bytes (used for Paste/Import).
/// Saves as `preview_custom.png` in the mod folder.
#[tauri::command]
pub async fn paste_thumbnail(folder_path: String, image_data: Vec<u8>) -> Result<String, String> {
    use image::ImageFormat;
    // use std::io::Cursor; // Removed
    const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

    let path = Path::new(&folder_path);
    if !path.exists() {
        return Err("Folder does not exist".to_string());
    }

    if image_data.len() > MAX_IMAGE_BYTES {
        return Err("Image too large. Max 10MB.".to_string());
    }

    // Decode image to verify it's valid
    let img =
        image::load_from_memory(&image_data).map_err(|e| format!("Invalid image data: {}", e))?;

    let target_path = path.join("preview_custom.png");

    // Save as PNG
    img.save_with_format(&target_path, ImageFormat::Png)
        .map_err(|e| format!("Failed to save image: {}", e))?;

    // Invalidate cache for this folder so new thumb is picked up
    // We invalidate the *old* thumbnail path if we knew it, but here we just ensure
    // future calls to get_thumbnail will see the new file (since find_thumbnail prioritizes preview_custom)
    // Actually, find_thumbnail prioritizes "preview*".

    // We should probably force a cache invalidation if there was an old cached thumbnail.
    // simpler: The next list_mod_folders will call find_thumbnail, which will find preview_custom.png,
    // then call ThumbnailCache::get_thumbnail(preview_custom.png).
    // The Cache keys are by *original path*. So if original path changes (from old.jpg to preview_custom.png),
    // it's a new cache entry. Old one eventually falls out of LRU.

    Ok(target_path.to_string_lossy().to_string())
}

/// Info about folder contents, used for delete confirmation.
/// Covers: NC-3.3-02
#[derive(Debug, Clone, Serialize)]
pub struct FolderContentInfo {
    pub path: String,
    pub name: String,
    pub item_count: usize,
    pub is_empty: bool,
}

/// Check folder contents before deletion.
/// Returns item count so frontend can show confirmation for non-empty folders.
/// Covers: NC-3.3-02
pub fn check_folder_contents(path: &Path) -> Result<FolderContentInfo, String> {
    if !path.exists() || !path.is_dir() {
        return Err(format!(
            "Path does not exist or is not a directory: {}",
            path.display()
        ));
    }

    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let item_count = fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory: {e}"))?
        .filter_map(|e| e.ok())
        .count();

    Ok(FolderContentInfo {
        path: path.to_string_lossy().to_string(),
        name,
        item_count,
        is_empty: item_count == 0,
    })
}

/// Tauri command wrapper for check_folder_contents.
/// Covers: NC-3.3-02
#[tauri::command]
pub async fn pre_delete_check(path: String) -> Result<FolderContentInfo, String> {
    check_folder_contents(Path::new(&path))
}

// ── Epic 10: QoL Commands ────────────────────────────────────────

/// Toggle the favorite status of a mod.
/// Covers: US-10.3, TC-10.3-02
#[tauri::command]
pub async fn toggle_favorite(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    id: String,
    favorite: bool,
) -> Result<(), String> {
    // 1. Update DB
    sqlx::query("UPDATE mods SET is_favorite = ? WHERE id = ?")
        .bind(favorite)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // 2. Sync to info.json logic
    // We need the folder path to update info.json
    // Ideally we should fetch folder_path first.
    let folder_path: Option<String> = sqlx::query_scalar("SELECT folder_path FROM mods WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    if let Some(path_str) = folder_path {
        let update = info_json::ModInfoUpdate {
            is_favorite: Some(favorite),
            ..Default::default()
        };
        // We don't fail the command if info.json update fails, just log it.
        // Or should we? TRD says Portable Truth. Let's try to update.
        if let Err(e) = info_json::update_info_json(Path::new(&path_str), &update) {
            log::warn!("Failed to sync favorite to info.json: {}", e);
        }
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct RandomModResult {
    pub id: String,
    pub name: String,
    pub thumbnail_path: Option<String>,
}

/// Pick a random mod to enable for the given game.
/// Filters by Safe Mode and verifies candidates are currently DISABLED.
/// Covers: US-10.2, TC-10.2-01, TC-10.2-02
#[tauri::command]
pub async fn pick_random_mod(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    is_safe: bool,
) -> Result<Option<RandomModResult>, String> {
    use rand::seq::SliceRandom;
    use sqlx::Row;

    // Fetch all disabled mods
    // Exclude hidden mods (starting with .)
    // Since we don't have a structured Mod model in this file, we fetch raw rows
    let mut query = "SELECT id, actual_name, folder_path, is_safe FROM mods WHERE game_id = ? AND status = 'DISABLED' AND folder_path NOT LIKE '%/.%' AND folder_path NOT LIKE '%\\.%'".to_string();
    
    // SQLite doesn't have robust path parsing in SQL, so we filter hidden folders in Rust to be safe
    // But basic LIKE exclusion helps.
    
    if is_safe {
        query.push_str(" AND is_safe = 1");
    }

    let rows = sqlx::query(&query)
        .bind(&game_id)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    if rows.is_empty() {
        return Ok(None);
    }

    // Filter in Rust for hidden folders (starting with .) just in case SQL LIKE missed specific OS separators
    let candidates: Vec<(String, String, String)> = rows
        .into_iter()
        .filter_map(|row| {
             let path: String = row.get("folder_path");
             let path_obj = Path::new(&path);
             
             // Check if folder name starts with dot
             if let Some(name) = path_obj.file_name() {
                 if name.to_string_lossy().starts_with('.') {
                     return None;
                 }
                 // Also exclude system/trash folders if they sneak in
             }
             
             Some((row.get("id"), row.get("actual_name"), path))
        })
        .collect();

    if candidates.is_empty() {
        return Ok(None);
    }

    let mut rng = rand::thread_rng();
    if let Some((id, name, _path)) = candidates.choose(&mut rng) {
        // Try to find a thumbnail (folder.jpg, etc.)
        // We reuse logic from get_mod_thumbnail or just check common names?
        // Actually, we can just return None for now and let the frontend use a default or fetch it.
        // But better to grab it if we can. 
        // We can use `crate::services::images::thumbnail_cache`.
        // Or simpler: scan for image.
        // Let's just return what we have. Frontend can fetch thumbnail via `get_mod_thumbnail` if needed, 
        // but `get_mod_thumbnail` takes a path. So we return the path too?
        // No, `get_mod_thumbnail` takes folder path string.
        
        // Let's try to resolve thumbnail here for "Premium" experience (immediate preview).
        // Since we have the path, we can check quickly.
        // Thumbnail: let frontend fetch via `get_mod_thumbnail` if needed.
        // The find_thumbnail_in_folder function was not implemented in folder_cmds.
        let thumb: Option<String> = None;

        Ok(Some(RandomModResult {
            id: id.clone(),
            name: name.clone(),
            thumbnail_path: thumb,
        }))
    } else {
        Ok(None)
    }
}

/// Get all active hash conflicts for the current game.
/// Covers: US-10.4
#[tauri::command]
pub async fn get_active_mod_conflicts(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<crate::services::scanner::conflict::ConflictInfo>, String> {
    use crate::services::scanner::conflict;
    use sqlx::Row;

    // 1. Get all ENABLED mods
    let rows = sqlx::query("SELECT folder_path FROM mods WHERE game_id = ? AND status = 'ENABLED'")
        .bind(&game_id)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut ini_files = Vec::new();

    // 2. Collect .ini files from these mods
    // We reuse scanner::walker::scan_folder_content but only for inis? 
    // Or just a quick walk. walker::scan_folder_content is efficient.
    for row in rows {
        let path_str: String = row.get("folder_path");
        let path = Path::new(&path_str);
        if path.exists() {
            let content = crate::services::scanner::walker::scan_folder_content(path, 3);
            ini_files.extend(content.ini_files);
        }
    }

    // 3. Detect conflicts
    let conflicts = conflict::detect_conflicts(&ini_files);

    Ok(conflicts)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // NC-3.3-02: Non-empty folder should report item count
    #[test]
    fn check_folder_contents_non_empty() {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path().join("TestMod");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("file1.ini"), "data").unwrap();
        fs::write(folder.join("file2.buf"), "data").unwrap();
        fs::create_dir(folder.join("subfolder")).unwrap();

        let info = check_folder_contents(&folder).unwrap();

        assert_eq!(info.name, "TestMod");
        assert_eq!(info.item_count, 3); // 2 files + 1 dir
        assert!(!info.is_empty);
    }

    // NC-3.3-02: Empty folder should report zero items
    #[test]
    fn check_folder_contents_empty() {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path().join("EmptyMod");
        fs::create_dir(&folder).unwrap();

        let info = check_folder_contents(&folder).unwrap();

        assert_eq!(info.name, "EmptyMod");
        assert_eq!(info.item_count, 0);
        assert!(info.is_empty);
    }

    // NC-3.3-02: Non-existent path should return error
    #[test]
    fn check_folder_contents_nonexistent() {
        let result = check_folder_contents(Path::new("/does/not/exist"));
        assert!(result.is_err());
    }

    // EC-3.04: Renaming to a case-variant of an existing folder should be blocked (Windows)
    #[test]
    fn rename_rejects_case_insensitive_duplicate() {
        let tmp = TempDir::new().unwrap();
        let raiden = tmp.path().join("Raiden");
        fs::create_dir(&raiden).unwrap();

        let other = tmp.path().join("Other");
        fs::create_dir(&other).unwrap();

        // Attempt to rename "Other" to "raiden" (lowercase variant of existing "Raiden")
        let rt = tokio::runtime::Runtime::new().unwrap();
        let state = WatcherState::new();
        let result = rt.block_on(rename_mod_folder_inner(
            &state,
            other.to_string_lossy().to_string(),
            "raiden".to_string(),
        ));

        // On Windows, Path::exists() is case-insensitive, so this should be caught
        assert!(result.is_err(), "Should reject case-insensitive duplicate");
        let err = result.unwrap_err();
        assert!(
            err.contains("already exists"),
            "Error should mention existing folder: {err}"
        );
    }

    // ── Epic 5: Prefix Standardization Tests ──────────────────────

    // Covers: EC-5.06 — lowercase "disabled_" variant
    #[test]
    fn standardize_prefix_lowercase_underscore() {
        assert_eq!(standardize_prefix("disabled_Ayaka", true), "Ayaka");
        assert_eq!(
            standardize_prefix("disabled_Ayaka", false),
            "DISABLED Ayaka"
        );
    }

    // Covers: EC-5.06 — dash variant
    #[test]
    fn standardize_prefix_dash_variant() {
        assert_eq!(standardize_prefix("DISABLED-Keqing", true), "Keqing");
        assert_eq!(
            standardize_prefix("DISABLED-Keqing", false),
            "DISABLED Keqing"
        );
    }

    // Covers: EC-5.06 — "Disable " with space
    #[test]
    fn standardize_prefix_partial_word() {
        assert_eq!(standardize_prefix("Disable Ayaka", true), "Ayaka");
        assert_eq!(standardize_prefix("dis Ayaka", false), "DISABLED Ayaka");
    }

    // Covers: TC-5.1-01 — standard "DISABLED " prefix (happy path)
    #[test]
    fn standardize_prefix_standard() {
        assert_eq!(standardize_prefix("DISABLED Raiden", true), "Raiden");
        assert_eq!(
            standardize_prefix("DISABLED Raiden", false),
            "DISABLED Raiden"
        );
    }

    // Covers: TC-5.1-01 — already enabled, target enable = no-op
    #[test]
    fn standardize_prefix_already_clean() {
        assert_eq!(standardize_prefix("Raiden", true), "Raiden");
        assert_eq!(standardize_prefix("Raiden", false), "DISABLED Raiden");
    }

    // Covers: EC-5.06 — toggle with messy prefix on filesystem
    #[test]
    fn toggle_with_bad_prefix_filesystem() {
        let tmp = TempDir::new().unwrap();
        let bad_name = tmp.path().join("disabled_Ayaka");
        fs::create_dir(&bad_name).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let state = WatcherState::new();
        let result = rt.block_on(toggle_mod_inner(
            &state,
            bad_name.to_string_lossy().to_string(),
            true,
        ));

        assert!(result.is_ok());
        let new_path = result.unwrap();
        assert!(
            new_path.ends_with("Ayaka"),
            "Should end with clean name: {new_path}"
        );
        assert!(
            !new_path.contains("disabled_"),
            "Should not contain messy prefix: {new_path}"
        );
    }

    // Covers: TC-5.1-01 — toggle disable then enable round-trip
    #[test]
    fn toggle_round_trip() {
        let tmp = TempDir::new().unwrap();
        let mod_name = tmp.path().join("Raiden");
        fs::create_dir(&mod_name).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let state = WatcherState::new();

        // Disable
        let disabled_path = rt
            .block_on(toggle_mod_inner(
                &state,
                mod_name.to_string_lossy().to_string(),
                false,
            ))
            .unwrap();
        assert!(
            disabled_path.ends_with("DISABLED Raiden"),
            "Should have DISABLED prefix: {disabled_path}"
        );

        // Enable
        let enabled_path = rt
            .block_on(toggle_mod_inner(&state, disabled_path, true))
            .unwrap();
        assert!(
            enabled_path.ends_with("Raiden"),
            "Should end with clean name: {enabled_path}"
        );
        assert!(
            !enabled_path.contains("DISABLED"),
            "Should not contain DISABLED: {enabled_path}"
        );
    }

    // Covers: NC-6.2-01 (Large Image Paste)
    #[test]
    fn paste_thumbnail_rejects_oversize() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("ModThumb");
        fs::create_dir(&mod_dir).unwrap();

        let oversized = vec![0_u8; 10 * 1024 * 1024 + 1];

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(paste_thumbnail(
            mod_dir.to_string_lossy().to_string(),
            oversized,
        ));

        assert!(
            result.is_err(),
            "Oversized clipboard image should be rejected"
        );
        assert!(
            result.unwrap_err().contains("Image too large"),
            "Error should mention image too large"
        );
    }
}

// ── Epic 5: Bulk Operations ──────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct BulkActionError {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkResult {
    pub success: Vec<String>, // List of successful paths/ids
    pub failures: Vec<BulkActionError>,
}

#[tauri::command]
pub async fn bulk_toggle_mods(
    app: AppHandle,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    paths: Vec<String>,
    enable: bool,
) -> Result<BulkResult, String> {
    let _lock = op_lock.acquire().await?;

    let total = paths.len();
    let action_label = if enable { "Enabling" } else { "Disabling" };

    // Emit start
    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: format!("{} {} mods...", action_label, total),
            current: 0,
            total,
            active: true,
        },
    );

    let mut success = Vec::new();
    let mut failures = Vec::new();

    for (i, path) in paths.iter().enumerate() {
        // Update progress
        let _ = app.emit(
            "bulk-progress",
            BulkProgressPayload {
                label: format!("{} {}/{}", action_label, i + 1, total),
                current: i + 1,
                total,
                active: true,
            },
        );

        match toggle_mod_inner(&state, path.clone(), enable).await {
            Ok(new_path) => success.push(new_path),
            Err(e) => failures.push(BulkActionError {
                path: path.clone(),
                error: e,
            }),
        }
    }

    Ok(BulkResult { success, failures })
}

pub async fn bulk_toggle_mods_inner(
    state: &WatcherState,
    paths: Vec<String>,
    enable: bool,
) -> Result<BulkResult, String> {
    let mut success = Vec::new();
    let mut failures = Vec::new();

    for path in paths {
        match toggle_mod_inner(state, path.clone(), enable).await {
            Ok(new_path) => success.push(new_path),
            Err(e) => failures.push(BulkActionError { path, error: e }),
        }
    }

    Ok(BulkResult { success, failures })
}

/// Bulk delete mods to trash.
/// Covers: TC-5.5-01, NC-5.1-04
#[tauri::command]
pub async fn bulk_delete_mods(
    app: AppHandle,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    paths: Vec<String>,
    game_id: Option<String>,
) -> Result<BulkResult, String> {
    let _lock = op_lock.acquire().await?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let trash_dir = app_data_dir.join("trash");

    // Ensure trash dir exists
    if !trash_dir.exists() {
        fs::create_dir_all(&trash_dir).map_err(|e| format!("Failed to create trash dir: {}", e))?;
    }

    let total = paths.len();
    // Emit start
    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: format!("Deleting {} mods...", total),
            current: 0,
            total,
            active: true,
        },
    );

    let mut success = Vec::new();
    let mut failures = Vec::new();

    for (i, path) in paths.iter().enumerate() {
        // Update progress
        let _ = app.emit(
            "bulk-progress",
            BulkProgressPayload {
                label: format!("Deleting {}/{}", i + 1, total),
                current: i + 1,
                total,
                active: true,
            },
        );

        match delete_mod_inner(&state, &trash_dir, path.clone(), game_id.clone()).await {
            Ok(_) => success.push(path.clone()),
            Err(e) => failures.push(BulkActionError {
                path: path.clone(),
                error: e,
            }),
        }
    }

    Ok(BulkResult { success, failures })
}

pub async fn bulk_delete_mods_inner(
    state: &WatcherState,
    trash_dir: &Path,
    paths: Vec<String>,
    game_id: Option<String>,
) -> Result<BulkResult, String> {
    let mut success = Vec::new();
    let mut failures = Vec::new();

    for path in paths {
        match delete_mod_inner(state, trash_dir, path.clone(), game_id.clone()).await {
            Ok(_) => success.push(path),
            Err(e) => failures.push(BulkActionError { path, error: e }),
        }
    }

    Ok(BulkResult { success, failures })
}

/// Bulk update info.json for multiple mods (e.g. adding tags, setting safe mode).
#[tauri::command]
pub async fn bulk_update_info(
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<BulkResult, String> {
    let mut success = Vec::new();
    let mut failures = Vec::new();

    for path in paths {
        // Since update_mod_info returns the updated info, we just check for explicit error
        match update_mod_info(path.clone(), update.clone()).await {
            Ok(_) => success.push(path),
            Err(e) => failures.push(BulkActionError { path, error: e }),
        }
    }

    Ok(BulkResult { success, failures })
}

/// Import strategy for drag & drop.
#[derive(Debug, Clone, serde::Deserialize)]
pub enum ImportStrategy {
    Raw,
    AutoOrganize,
}

/// Import external folders/files into the mods directory.
/// Used for Drag & Drop from Windows Explorer.
/// Covers: Epic 4 Gap (DnD Persistence) & Epic 3 Gap (Smart Drop), NC-5.1-04
#[tauri::command]
pub async fn import_mods_from_paths(
    app: AppHandle,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    paths: Vec<String>,
    target_dir: String,
    strategy: ImportStrategy,
    db_json: Option<String>,
) -> Result<BulkResult, String> {
    let _lock = op_lock.acquire().await?;

    let total = paths.len();
    // Emit start
    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: format!("Importing {} items...", total),
            current: 0,
            total,
            active: true,
        },
    );

    let mut success = Vec::new();
    let mut failures = Vec::new();

    let target = Path::new(&target_dir);
    if !target.exists() || !target.is_dir() {
        return Err(format!("Target directory does not exist: {}", target_dir));
    }

    // Load DB if needed
    let db = if let ImportStrategy::AutoOrganize = strategy {
        if let Some(json) = db_json {
            Some(crate::services::scanner::deep_matcher::MasterDb::from_json(
                &json,
            )?)
        } else {
            return Err("Auto-Organize requires db_json".to_string());
        }
    } else {
        None
    };

    for (i, path_str) in paths.iter().enumerate() {
        // Update progress
        let _ = app.emit(
            "bulk-progress",
            BulkProgressPayload {
                label: format!("Importing {}/{}", i + 1, total),
                current: i + 1,
                total,
                active: true,
            },
        );

        let path = Path::new(&path_str);
        if !path.exists() {
            failures.push(BulkActionError {
                path: path_str.clone(),
                error: "Source path does not exist".to_string(),
            });
            continue;
        }

        // Handle Auto-Organize
        if let (ImportStrategy::AutoOrganize, Some(master_db)) = (&strategy, &db) {
            // Suppress watcher
            let _guard = SuppressionGuard::new(&state.suppressor);

            match crate::services::scanner::organizer::organize_mod(path, target, master_db) {
                Ok(res) => success.push(res.new_path.to_string_lossy().to_string()),
                Err(e) => failures.push(BulkActionError {
                    path: path_str.clone(),
                    error: e,
                }),
            }
            continue;
        }

        // Check for Archive Import first (US-5.2)
        if let Some(format) = ArchiveFormat::from_path(path) {
            log::info!("Detected archive import: {:?} ({:?})", path_str, format);

            // Suppress watcher during extraction
            let _guard = SuppressionGuard::new(&state.suppressor);

            // Extract (smart flatten enabled by default in extract_archive)
            // No password support in simple DnD import for now (could prompt later if needed)
            match extract_archive(path, target, None, false) {
                Ok(result) => {
                    if result.success {
                        // Rename to DISABLED prefix by default for safety
                        let extracted_path = Path::new(&result.dest_path);
                        let folder_name = extracted_path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy();

                        if !folder_name.starts_with(DISABLED_PREFIX) {
                            let new_name = format!("{}{}", DISABLED_PREFIX, folder_name);
                            let new_path = target.join(&new_name);
                            if let Err(e) = fs::rename(extracted_path, &new_path) {
                                log::warn!("Failed to add DISABLED prefix to extracted mod: {}", e);
                                success.push(result.dest_path);
                            } else {
                                // Smart Organize Step (US-5.2 Enhanced)
                                if let Some(master_db) = &db {
                                    match crate::services::scanner::organizer::organize_mod(
                                        &new_path, target, master_db,
                                    ) {
                                        Ok(res) => {
                                            success.push(res.new_path.to_string_lossy().to_string())
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "Smart Organization failed for {}: {}",
                                                new_name,
                                                e
                                            );
                                            success.push(new_path.to_string_lossy().to_string())
                                        }
                                    }
                                } else {
                                    success.push(new_path.to_string_lossy().to_string());
                                }
                            }
                        } else {
                            // Already has prefix, try organize directly
                            let extracted_path = Path::new(&result.dest_path);
                            if let Some(master_db) = &db {
                                match crate::services::scanner::organizer::organize_mod(
                                    extracted_path,
                                    target,
                                    master_db,
                                ) {
                                    Ok(res) => {
                                        success.push(res.new_path.to_string_lossy().to_string())
                                    }
                                    Err(e) => {
                                        log::warn!("Smart Organization failed: {}", e);
                                        success.push(result.dest_path)
                                    }
                                }
                            } else {
                                success.push(result.dest_path);
                            }
                        }
                    } else {
                        failures.push(BulkActionError {
                            path: path_str.clone(),
                            error: result
                                .error
                                .unwrap_or_else(|| "Unknown extraction error".into()),
                        });
                    }
                }
                Err(e) => {
                    failures.push(BulkActionError {
                        path: path_str.clone(),
                        error: e,
                    });
                }
            }
            continue;
        }

        // Handle Raw Import (Existing Logic)
        let file_name = match path.file_name() {
            Some(n) => n,
            None => {
                failures.push(BulkActionError {
                    path: path_str.clone(),
                    error: "Invalid file name".to_string(),
                });
                continue;
            }
        };

        let dest = target.join(file_name);
        if dest.exists() {
            failures.push(BulkActionError {
                path: path_str.clone(),
                error: "Destination already exists".to_string(),
            });
            continue;
        }

        // Suppress watcher
        let _guard = SuppressionGuard::new(&state.suppressor);

        // Try rename (Move)
        if let Err(e) = fs::rename(path, &dest) {
            // If rename fails (e.g. cross-device), try copy
            log::warn!("Rename failed, treating as copy error for now: {}", e);
            failures.push(BulkActionError {
                path: path_str.clone(),
                error: format!(
                    "Failed to move (cross-device move not yet implemented): {}",
                    e
                ),
            });
        } else {
            success.push(path_str.to_string());
        }
    }

    Ok(BulkResult { success, failures })
}
