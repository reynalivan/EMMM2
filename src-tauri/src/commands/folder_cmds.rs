use serde::Serialize;
use sqlx::Row;
use std::collections::HashMap;
use std::path::Path;
use std::time::UNIX_EPOCH;
use crate::services::config::ConfigService;

/// Extracted info from info.json for populating ModFolder fields.
struct InfoAnalysis {
    has_info_json: bool,
    is_favorite: bool,
    is_misplaced: bool,
    is_safe: bool,
    metadata: Option<HashMap<String, String>>,
    category: Option<String>,
}

impl Default for InfoAnalysis {
    fn default() -> Self {
        Self { has_info_json: false, is_favorite: false, is_misplaced: false, is_safe: true, metadata: None, category: None }
    }
}

// Helper to read info.json and determine flags + metadata
fn analyze_mod_metadata(path: &Path, sub_path: Option<&str>) -> InfoAnalysis {
    if !path.join("info.json").exists() {
        return InfoAnalysis::default();
    }

    match crate::services::file_ops::info_json::read_info_json(path) {
        Ok(Some(info)) => {
            let mut misplaced = false;
            if let Some(sp) = sub_path {
                let current_cat = sp.split(['/', '\\']).next().unwrap_or(sp);
                if let Some(meta_char) = info.metadata.get("character") {
                    if !meta_char.eq_ignore_ascii_case(current_cat) {
                        misplaced = true;
                    }
                }
            }
            let category = info.metadata.get("category").cloned();
            let metadata = if info.metadata.is_empty() { None } else { Some(info.metadata.clone()) };
            InfoAnalysis {
                has_info_json: true,
                is_favorite: info.is_favorite,
                is_misplaced: misplaced,
                is_safe: info.is_safe,
                metadata,
                category,
            }
        }
        Ok(None) => InfoAnalysis::default(),
        Err(_) => InfoAnalysis { has_info_json: true, ..InfoAnalysis::default() },
    }
}

fn normalize_keywords(keywords: &[String]) -> Vec<String> {
    keywords
        .iter()
        .map(|k| k.trim().to_lowercase())
        .filter(|k| !k.is_empty())
        .collect()
}

fn contains_filtered_keyword(folder: &ModFolder, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return false;
    }

    let mut haystacks = vec![folder.name.to_lowercase(), folder.folder_name.to_lowercase()];

    if let Ok(Some(info)) = crate::services::file_ops::info_json::read_info_json(Path::new(&folder.path)) {
        haystacks.push(info.actual_name.to_lowercase());
        haystacks.push(info.author.to_lowercase());
        haystacks.push(info.description.to_lowercase());
        haystacks.extend(info.tags.into_iter().map(|tag| tag.to_lowercase()));
    }

    keywords
        .iter()
        .any(|keyword| haystacks.iter().any(|value| value.contains(keyword)))
}

fn apply_safe_mode_filter(folders: Vec<ModFolder>, config: &ConfigService) -> Vec<ModFolder> {
    let settings = config.get_settings();
    if !settings.safe_mode.enabled {
        return folders;
    }

    let keywords = normalize_keywords(&settings.safe_mode.keywords);
    let force_exclusive_mode = settings.safe_mode.force_exclusive_mode;

    folders
        .into_iter()
        .filter(|folder| {
            if !folder.is_safe {
                return false;
            }

            if !force_exclusive_mode {
                return true;
            }

            !contains_filtered_keyword(folder, &keywords)
        })
        .collect()
}

/// Represents a single mod folder entry from the filesystem.
#[derive(Debug, Clone, Serialize)]
pub struct ModFolder {
    /// Database ID (UUID), if available
    pub id: Option<String>,
    /// Display name (without "DISABLED " prefix)
    pub name: String,
    /// Actual folder name on disk
    pub folder_name: String,
    /// Full absolute path
    pub path: String,
    /// Whether the mod is enabled (no "DISABLED " prefix)
    pub is_enabled: bool,
    /// Whether this entry is a directory (vs a file)
    pub is_directory: bool,
    /// Discovered thumbnail image path (if any)
    pub thumbnail_path: Option<String>,
    /// Last modified time (epoch seconds)
    pub modified_at: u64,
    /// Total size in bytes (shallow for directories)
    pub size_bytes: u64,
    /// Whether the folder contains an info.json file
    pub has_info_json: bool,
    /// Whether the mod is marked as favorite
    pub is_favorite: bool,
    /// Whether the mod appears to be in the wrong category (Basic Heuristic)
    pub is_misplaced: bool,
    /// Whether the mod is marked as safe (from info.json)
    pub is_safe: bool,
    /// Metadata from info.json (element, rarity, etc.)
    pub metadata: Option<HashMap<String, String>>,
    /// Category from info.json metadata
    pub category: Option<String>,
}

use crate::DISABLED_PREFIX;

/// List mod folders at a given path, optionally navigating into a sub_path.
///
/// - `mods_path`: The root mods directory for the game.
/// - `sub_path`: Optional relative sub-path for deep navigation (e.g., "Raiden/Set1").
///
/// Returns folder entries with enabled/disabled state, thumbnails, metadata.
/// Covers: TC-4.1-01 (Deep Navigation), TC-4.1-02 (Sort by Date)
#[tauri::command]
pub async fn list_mod_folders(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    config: tauri::State<'_, ConfigService>,
    game_id: Option<String>,
    mods_path: String,
    sub_path: Option<String>,
    object_id: Option<String>,
) -> Result<Vec<ModFolder>, String> {
    let folders = list_mod_folders_inner(Some(&*pool), game_id, mods_path, sub_path, object_id).await?;
    Ok(apply_safe_mode_filter(folders, &config))
}

pub async fn list_mod_folders_inner(
    pool: Option<&sqlx::SqlitePool>,
    game_id: Option<String>,
    mods_path: String,
    sub_path: Option<String>,
    object_id: Option<String>,
) -> Result<Vec<ModFolder>, String> {
    let base = Path::new(&mods_path);

    if !base.exists() {
        return Err(format!("Mods path does not exist: {mods_path}"));
    }
    if !base.is_dir() {
        return Err(format!("Mods path is not a directory: {mods_path}"));
    }

    log::debug!("Listing mods at base: {}", base.display());

    // Resolve target directory (base + optional sub_path)
    let target = match &sub_path {
        Some(sp) if !sp.is_empty() => {
            let resolved = base.join(sp);
            if !resolved.exists() || !resolved.is_dir() {
                return Err(format!("Sub-path does not exist: {sp}"));
            }
            resolved
        }
        _ => base.to_path_buf(),
    };

    // Strategy:
    // 1. If game_id is provided and we are at root (sub_path is empty/None), try DB.
    // 2. DB only tracks "Mods", not generic subfolders.
    // 3. If DB has results, return them.
    // 4. Fallback to FS scan if DB is empty or if we are in a subfolder (DB doesn't index sub-structure explicitly yet or we want to be safe).

    if let Some(gid) = game_id {
        // Deep navigation usually implies browsing *inside* a mod, which might not be indexed the same way
        // OR we might support categories folders later.
        // For now: Cache-First applies to the Main Mod List.
        if sub_path.is_none() || sub_path.as_deref() == Some("") {
            log::debug!("Checking DB cache for game_id: {}", gid);
            // Fetch from DB if pool is available
            if let Some(p) = pool {
                // When object_id is provided, filter mods by their linked object
                let db_mods = if let Some(ref oid) = object_id {
                    log::debug!("Filtering mods by object_id: {}", oid);
                    sqlx::query(
                        "SELECT id, actual_name, folder_path, status FROM mods WHERE game_id = ? AND object_id = ?",
                    )
                    .bind(&gid)
                    .bind(oid)
                    .fetch_all(p)
                    .await
                    .map_err(|e| e.to_string())?
                } else {
                    sqlx::query(
                        "SELECT id, actual_name, folder_path, status FROM mods WHERE game_id = ?",
                    )
                    .bind(&gid)
                    .fetch_all(p)
                    .await
                    .map_err(|e| e.to_string())?
                };

                log::debug!("DB returned {} rows", db_mods.len());

                if !db_mods.is_empty() {
                    // We have cache! Reconstruct ModFolder view models.
                    // We still need to verify existence? "Lazy Verify".
                    // For "Instant Startup", we trust DB.
                    // But we still need thumbnails and info.json (cheap reads).

                    let mut folders = Vec::new();
                    for row in db_mods {
                        let folder_path_str: String =
                            row.try_get("folder_path").map_err(|e| e.to_string())?;
                        let path = Path::new(&folder_path_str);

                        // Quick existence check — skip broken DB entries
                        if !path.exists() {
                            continue;
                        }

                        let folder_name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let id: String = row.try_get("id").map_err(|e| e.to_string())?;
                        let name: String = row.try_get("actual_name").map_err(|e| e.to_string())?;
                        let status: String = row.try_get("status").map_err(|e| e.to_string())?;
                        let is_enabled = status != "DISABLED";

                        let fs_meta = path.metadata().ok();
                        let modified_at = fs_meta
                            .as_ref()
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let size_bytes = fs_meta.map(|m| m.len()).unwrap_or(0);

                        // Thumbnail resolved lazily via get_mod_thumbnail command
                        let thumbnail_path = None;

                        // Info.json Analysis
                        let info = analyze_mod_metadata(path, sub_path.as_deref());

                        folders.push(ModFolder {
                            id: Some(id),
                            name,
                            folder_name,
                            path: folder_path_str,
                            is_enabled,
                            is_directory: true, // DB mods are folders
                            thumbnail_path,
                            modified_at,
                            size_bytes,
                            has_info_json: info.has_info_json,
                            is_favorite: info.is_favorite,
                            is_misplaced: info.is_misplaced,
                            is_safe: info.is_safe,
                            metadata: info.metadata,
                            category: info.category,
                        });
                    }

                    // Sort
                    folders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    log::info!(
                        "Listed {} mod folders from DB Cache for game {}",
                        folders.len(),
                        gid
                    );
                    return Ok(folders);
                }
            } else {
                log::warn!("Game ID provided but no DB pool. Skipping DB cache.");
            }
        }
    }

    // --- FS Fallback (Existing Logic) ---
    // If we reached here, either no game_id, safe-mode subpath, or DB empty.
    log::info!(
        "Cache miss or subpath. Falling back to FS scan for {}",
        target.display()
    );

    let entries = std::fs::read_dir(&target).map_err(|e| {
        let msg = format!("Failed to read directory {}: {}", target.display(), e);
        log::error!("{}", msg);
        msg
    })?;

    log::debug!("FS Scan found entries iterator");

    let mut folders: Vec<ModFolder> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let is_directory = path.is_dir();

        // Skip non-directory files at the top level
        if !is_directory {
            continue;
        }

        let folder_name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        // Skip hidden folders (starting with '.')
        if folder_name.starts_with('.') {
            continue;
        }

        let (is_enabled, display_name) = if let Some(stripped) = folder_name.strip_prefix(DISABLED_PREFIX) {
            (false, stripped.to_string())
        } else {
            (true, folder_name.clone())
        };

        // Get modified time
        let modified_at = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Get size (shallow — just the metadata size, not recursive)
        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);

        // Thumbnail resolved lazily via get_mod_thumbnail command
        let thumbnail_path = None;

        // Read info.json metadata (read-only, no auto-creation during listing)
        let info = if is_directory {
            analyze_mod_metadata(&path, sub_path.as_deref())
        } else {
            InfoAnalysis::default()
        };

        folders.push(ModFolder {
            id: None,
            name: display_name,
            folder_name,
            path: path.to_string_lossy().to_string(),
            is_enabled,
            is_directory,
            thumbnail_path,
            modified_at,
            size_bytes,
            has_info_json: info.has_info_json,
            is_favorite: info.is_favorite,
            is_misplaced: info.is_misplaced,
            is_safe: info.is_safe,
            metadata: info.metadata,
            category: info.category,
        });
    }

    // Sort alphabetically by display name
    folders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    log::info!(
        "Listed {} mod folders from {} (sub: {:?})",
        folders.len(),
        mods_path,
        sub_path
    );

    Ok(folders)
}

/// Lazily resolve thumbnail for a single mod folder.
/// Called per-card from the frontend after the folder list is rendered.
/// Delegates to ThumbnailCache::resolve() which caps concurrency (4 max),
/// checks folder-keyed L1, and falls back to FS traversal + image processing.
#[tauri::command]
pub async fn get_mod_thumbnail(folder_path: String) -> Result<Option<String>, String> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    ThumbnailCache::resolve(&folder_path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_list_mod_folders_basic() {
        let tmp = TempDir::new().unwrap();
        let mods = tmp.path().join("Mods");
        fs::create_dir(&mods).unwrap();
        fs::create_dir(mods.join("Raiden")).unwrap();
        fs::create_dir(mods.join("DISABLED Ayaka")).unwrap();
        fs::create_dir(mods.join("Albedo")).unwrap();

        let result =
            list_mod_folders_inner(None, None, mods.to_string_lossy().to_string(), None, None).await;
        assert!(result.is_ok());

        let folders = result.unwrap();
        assert_eq!(folders.len(), 3);

        // Sorted alphabetically by display_name
        assert_eq!(folders[0].name, "Albedo");
        assert!(folders[0].is_enabled);
        assert!(folders[0].is_directory);

        assert_eq!(folders[1].name, "Ayaka");
        assert!(!folders[1].is_enabled);
        assert_eq!(folders[1].folder_name, "DISABLED Ayaka");

        assert_eq!(folders[2].name, "Raiden");
        assert!(folders[2].is_enabled);
    }

    #[tokio::test]
    async fn test_list_mod_folders_skips_files_and_hidden() {
        let tmp = TempDir::new().unwrap();
        let mods = tmp.path().join("Mods");
        fs::create_dir(&mods).unwrap();
        fs::create_dir(mods.join("ValidMod")).unwrap();
        fs::create_dir(mods.join(".hidden")).unwrap();
        fs::write(mods.join("readme.txt"), "hello").unwrap();

        let result =
            list_mod_folders_inner(None, None, mods.to_string_lossy().to_string(), None, None).await;
        let folders = result.unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].name, "ValidMod");
    }

    #[tokio::test]
    async fn test_list_mod_folders_nonexistent_path() {
        let result =
            list_mod_folders_inner(None, None, "C:\\nonexistent\\fake\\path".to_string(), None, None)
                .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_list_mod_folders_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let mods = tmp.path().join("Mods");
        fs::create_dir(&mods).unwrap();

        let result =
            list_mod_folders_inner(None, None, mods.to_string_lossy().to_string(), None, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    // Covers: TC-4.1-01 (Deep Navigation)
    #[tokio::test]
    async fn test_list_mod_folders_deep_navigation() {
        let tmp = TempDir::new().unwrap();
        let mods = tmp.path().join("Mods");
        let raiden = mods.join("Raiden");
        let set1 = raiden.join("Set1");
        fs::create_dir_all(&set1).unwrap();
        fs::create_dir(raiden.join("Set2")).unwrap();

        // Navigate into "Raiden" subfolder
        let result = list_mod_folders_inner(
            None,
            None,
            mods.to_string_lossy().to_string(),
            Some("Raiden".to_string()),
            None,
        )
        .await;
        assert!(result.is_ok());

        let folders = result.unwrap();
        assert_eq!(folders.len(), 2);
        assert_eq!(folders[0].name, "Set1");
        assert_eq!(folders[1].name, "Set2");
    }

    // Covers: TC-4.1-01 (Deep Navigation — invalid sub_path)
    #[tokio::test]
    async fn test_list_mod_folders_invalid_subpath() {
        let tmp = TempDir::new().unwrap();
        let mods = tmp.path().join("Mods");
        fs::create_dir(&mods).unwrap();

        let result = list_mod_folders_inner(
            None,
            None,
            mods.to_string_lossy().to_string(),
            Some("NonExistent".to_string()),
            None,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Sub-path does not exist"));
    }

    // Covers: TC-4.2-02 (thumbnail resolved lazily via get_mod_thumbnail)
    #[tokio::test]
    async fn test_list_mod_folders_thumbnail_deferred() {
        let tmp = TempDir::new().unwrap();
        let mods = tmp.path().join("Mods");
        let mod_folder = mods.join("Raiden");
        fs::create_dir_all(&mod_folder).unwrap();
        fs::write(mod_folder.join("preview.png"), "fake png data").unwrap();

        let result =
            list_mod_folders_inner(None, None, mods.to_string_lossy().to_string(), None, None).await;
        let folders = result.unwrap();
        assert_eq!(folders.len(), 1);
        // Thumbnails are now resolved lazily via get_mod_thumbnail, not during listing
        assert!(folders[0].thumbnail_path.is_none());
    }

    // Covers: DI-4.03 (info.json detection)
    #[tokio::test]
    async fn test_list_mod_folders_info_json_detection() {
        let tmp = TempDir::new().unwrap();
        let mods = tmp.path().join("Mods");
        let with_info = mods.join("WithInfo");
        let without_info = mods.join("NoInfo");
        fs::create_dir_all(&with_info).unwrap();
        fs::create_dir_all(&without_info).unwrap();
        fs::write(with_info.join("info.json"), "{}").unwrap();

        let result =
            list_mod_folders_inner(None, None, mods.to_string_lossy().to_string(), None, None).await;
        let folders = result.unwrap();
        assert_eq!(folders.len(), 2);

        let info_folder = folders.iter().find(|f| f.name == "NoInfo").unwrap();
        assert!(!info_folder.has_info_json);

        let no_info_folder = folders.iter().find(|f| f.name == "WithInfo").unwrap();
        assert!(no_info_folder.has_info_json);
    }

    // Covers: TC-4.1-02 (modified_at is populated)
    #[tokio::test]
    async fn test_list_mod_folders_has_modified_at() {
        let tmp = TempDir::new().unwrap();
        let mods = tmp.path().join("Mods");
        fs::create_dir_all(mods.join("TestMod")).unwrap();

        let result =
            list_mod_folders_inner(None, None, mods.to_string_lossy().to_string(), None, None).await;
        let folders = result.unwrap();
        assert_eq!(folders.len(), 1);
        // modified_at should be non-zero (a real timestamp)
        assert!(folders[0].modified_at > 0);
    }
}
