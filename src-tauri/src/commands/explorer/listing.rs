use std::collections::HashMap;
use std::path::Path;
use std::time::UNIX_EPOCH;

use sqlx::Row;

use crate::DISABLED_PREFIX;

use super::helpers::{analyze_mod_metadata, is_db_misplaced, try_resolve_alternate};
use super::types::ModFolder;

/// Read the filesystem, build `ModFolder` entries, then optionally enrich with DB IDs.
pub(crate) async fn scan_fs_folders(
    target: &Path,
    sub_path: Option<&str>,
    pool: Option<&sqlx::SqlitePool>,
    game_id: Option<&str>,
) -> Result<Vec<ModFolder>, String> {
    let entries = std::fs::read_dir(target).map_err(|e| {
        let msg = format!("Failed to read directory {}: {}", target.display(), e);
        log::error!("{}", msg);
        msg
    })?;

    let mut folders: Vec<ModFolder> = entries
        .flatten()
        .filter_map(|entry| build_mod_folder_from_fs_entry(entry, sub_path))
        .collect();

    folders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Enrich FS folders with DB metadata to mark registered mods.
    if let (Some(p), Some(gid)) = (pool, game_id) {
        let like_pattern = format!("{}%", target.to_string_lossy());
        let db_rows = sqlx::query(
            "SELECT id, folder_path FROM mods WHERE game_id = ? AND folder_path LIKE ?",
        )
        .bind(gid)
        .bind(&like_pattern)
        .fetch_all(p)
        .await
        .unwrap_or_default();

        let mod_map: HashMap<String, String> = db_rows
            .into_iter()
            .filter_map(|r| {
                let fp: String = r.try_get("folder_path").ok()?;
                let id: String = r.try_get("id").ok()?;
                Some((fp, id))
            })
            .collect();

        for folder in &mut folders {
            if let Some(id) = mod_map.get(&folder.path) {
                folder.id = Some(id.clone());
            }
        }
    }

    Ok(folders)
}

/// Attempt to serve the mod list from the DB cache.
/// Returns `Some(folders)` if DB had results, `None` to signal a cache miss
/// (caller should fall back to FS scan).
pub(crate) async fn try_db_cache(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: Option<&str>,
    sub_path: Option<&str>,
) -> Result<Option<Vec<ModFolder>>, String> {
    let db_mods = if let Some(oid) = object_id {
        log::debug!("Filtering mods by object_id: {}", oid);
        sqlx::query(
            "SELECT id, actual_name, folder_path, status, object_id FROM mods WHERE game_id = ? AND object_id = ?",
        )
        .bind(game_id)
        .bind(oid)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query(
            "SELECT id, actual_name, folder_path, status, object_id FROM mods WHERE game_id = ?",
        )
        .bind(game_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?
    };

    log::debug!("DB returned {} rows", db_mods.len());

    if db_mods.is_empty() {
        return Ok(None);
    }

    let obj_rows = sqlx::query("SELECT id, folder_name FROM objects WHERE game_id = ?")
        .bind(game_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    let obj_folder_map: HashMap<String, String> = obj_rows
        .into_iter()
        .filter_map(|r| {
            let oid: String = r.try_get("id").ok()?;
            let fname: String = r.try_get("folder_name").ok()?;
            Some((oid, fname))
        })
        .collect();

    let mut folders = Vec::new();
    for row in db_mods {
        if let Some(folder) =
            build_mod_folder_from_db_row(row, pool, sub_path, &obj_folder_map).await?
        {
            folders.push(folder);
        }
    }

    folders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    log::info!(
        "Listed {} mod folders from DB Cache for game {}",
        folders.len(),
        game_id
    );
    Ok(Some(folders))
}

/// Builds a `ModFolder` from a filesystem `DirEntry`. Returns `None` if the entry
/// should be skipped (non-directory, hidden, or no file name).
pub(crate) fn build_mod_folder_from_fs_entry(
    entry: std::fs::DirEntry,
    sub_path: Option<&str>,
) -> Option<ModFolder> {
    let path = entry.path();
    if !path.is_dir() {
        return None;
    }

    let folder_name = path.file_name()?.to_string_lossy().to_string();
    if folder_name.starts_with('.') {
        return None;
    }

    let (is_enabled, display_name) =
        if let Some(stripped) = folder_name.strip_prefix(DISABLED_PREFIX) {
            (false, stripped.to_string())
        } else {
            (true, folder_name.clone())
        };

    // Call metadata once and reuse for both modified_at and size_bytes.
    let entry_meta = entry.metadata().ok();
    let modified_at = entry_meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let size_bytes = entry_meta.map(|m| m.len()).unwrap_or(0);

    let info = analyze_mod_metadata(&path, sub_path);

    Some(ModFolder {
        id: None,
        name: display_name,
        folder_name,
        path: path.to_string_lossy().to_string(),
        is_enabled,
        is_directory: true,
        thumbnail_path: None,
        modified_at,
        size_bytes,
        has_info_json: info.has_info_json,
        is_favorite: info.is_favorite,
        is_misplaced: info.is_misplaced,
        is_safe: info.is_safe,
        metadata: info.metadata,
        category: info.category,
    })
}

/// Builds a `ModFolder` from a DB row. Self-heals the DB if the path is stale.
/// Returns `None` if the mod is truly gone from disk.
pub(crate) async fn build_mod_folder_from_db_row(
    row: sqlx::sqlite::SqliteRow,
    pool: &sqlx::SqlitePool,
    sub_path: Option<&str>,
    obj_folder_map: &HashMap<String, String>,
) -> Result<Option<ModFolder>, String> {
    let folder_path_str: String = row.try_get("folder_path").map_err(|e| e.to_string())?;
    let id: String = row.try_get("id").map_err(|e| e.to_string())?;
    let name: String = row.try_get("actual_name").map_err(|e| e.to_string())?;

    let db_path = Path::new(&folder_path_str);

    // Disk is source of truth — self-heal stale DB paths.
    let (resolved_path, resolved_path_str, is_enabled) = if db_path.exists() {
        let fname = db_path.file_name().unwrap_or_default().to_string_lossy();
        let enabled = !fname.starts_with(DISABLED_PREFIX);
        (db_path.to_path_buf(), folder_path_str.clone(), enabled)
    } else {
        match try_resolve_alternate(db_path) {
            Some((alt_path, alt_enabled)) => {
                let alt_str = alt_path.to_string_lossy().to_string();
                let new_status = if alt_enabled { "ENABLED" } else { "DISABLED" };
                let _ = sqlx::query("UPDATE mods SET folder_path = ?, status = ? WHERE id = ?")
                    .bind(&alt_str)
                    .bind(new_status)
                    .bind(&id)
                    .execute(pool)
                    .await;
                log::info!("Self-healed mod {}: {} → {}", id, folder_path_str, alt_str);
                (alt_path, alt_str, alt_enabled)
            }
            None => {
                log::debug!("Skipping missing mod {}: {}", id, folder_path_str);
                return Ok(None); // truly gone from disk
            }
        }
    };

    let folder_name = resolved_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let fs_meta = resolved_path.metadata().ok();
    let modified_at = fs_meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let size_bytes = fs_meta.map(|m| m.len()).unwrap_or(0);

    let info = analyze_mod_metadata(&resolved_path, sub_path);
    let mod_obj_id: Option<String> = row.try_get("object_id").ok();
    let db_misplaced = is_db_misplaced(mod_obj_id, &resolved_path, obj_folder_map);

    Ok(Some(ModFolder {
        id: Some(id),
        name,
        folder_name,
        path: resolved_path_str,
        is_enabled,
        is_directory: true, // DB mods are always folders
        thumbnail_path: None,
        modified_at,
        size_bytes,
        has_info_json: info.has_info_json,
        is_favorite: info.is_favorite,
        is_misplaced: info.is_misplaced || db_misplaced,
        is_safe: info.is_safe,
        metadata: info.metadata,
        category: info.category,
    }))
}

pub(crate) async fn list_mod_folders_inner(
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

    // Resolve target directory (base + optional sub_path).
    // Also checks DISABLED prefix variant for disabled object folders.
    let target = match &sub_path {
        Some(sp) if !sp.is_empty() => {
            let resolved = base.join(sp);
            if resolved.exists() && resolved.is_dir() {
                resolved
            } else {
                // Try with DISABLED prefix (e.g., "Hanya" → "DISABLED Hanya")
                let disabled_resolved = base.join(format!("{}{}", DISABLED_PREFIX, sp));
                if disabled_resolved.exists() && disabled_resolved.is_dir() {
                    disabled_resolved
                } else {
                    return Err(format!("Sub-path does not exist: {sp}"));
                }
            }
        }
        _ => base.to_path_buf(),
    };

    // Strategy:
    // 1. If game_id is provided and we are at root (sub_path is empty/None), try DB.
    // 2. DB only tracks "Mods", not generic subfolders.
    // 3. If DB has results, return them.
    // 4. Fallback to FS scan if DB is empty or if we are in a subfolder.

    if let Some(ref gid) = game_id {
        if sub_path.is_none() || sub_path.as_deref() == Some("") {
            log::debug!("Checking DB cache for game_id: {}", gid);
            if let Some(p) = pool {
                if let Some(folders) =
                    try_db_cache(p, gid, object_id.as_deref(), sub_path.as_deref()).await?
                {
                    return Ok(folders);
                }
            } else {
                log::warn!("Game ID provided but no DB pool. Skipping DB cache.");
            }
        }
    }

    // Cache miss or subpath — fall back to filesystem scan.
    log::info!(
        "Cache miss or subpath. Falling back to FS scan for {}",
        target.display()
    );

    let folders = scan_fs_folders(&target, sub_path.as_deref(), pool, game_id.as_deref()).await?;

    log::info!(
        "Listed {} mod folders from {} (sub: {:?})",
        folders.len(),
        mods_path,
        sub_path
    );

    Ok(folders)
}
