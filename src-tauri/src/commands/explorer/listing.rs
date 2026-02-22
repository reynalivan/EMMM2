use std::collections::HashMap;
use std::path::Path;
use std::time::UNIX_EPOCH;

use sqlx::Row;

use crate::DISABLED_PREFIX;

use super::helpers::analyze_mod_metadata;
use super::types::ModFolder;

/// Read the filesystem, build `ModFolder` entries, then optionally enrich with DB IDs.
pub(crate) async fn scan_fs_folders(
    target: &Path,
    sub_path: Option<&str>,
    pool: Option<&sqlx::SqlitePool>,
    game_id: Option<&str>,
) -> Result<Vec<ModFolder>, String> {
    let entries = match std::fs::read_dir(target) {
        Ok(e) => e,
        Err(e) => {
            log::debug!("Could not read directory (may not exist yet): {}", e);
            return Ok(Vec::new());
        }
    };

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
    let (node_type, classification_reasons) = super::classifier::classify_folder(&path);

    Some(ModFolder {
        node_type: node_type.as_str().to_string(),
        classification_reasons,
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

pub(crate) async fn list_mod_folders_inner(
    pool: Option<&sqlx::SqlitePool>,
    game_id: Option<String>,
    mods_path: String,
    sub_path: Option<String>,
    _object_id: Option<String>,
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
    let target = match &sub_path {
        Some(sp) if !sp.is_empty() => base.join(sp),
        _ => base.to_path_buf(),
    };

    // Always scan the filesystem as the source of truth.
    // scan_fs_folders will automatically enrich FS results with DB metadata if game_id/pool are provided.
    log::info!("Scanning filesystem for mods at {}", target.display());

    let folders = scan_fs_folders(&target, sub_path.as_deref(), pool, game_id.as_deref()).await?;

    log::info!(
        "Listed {} mod folders from {} (sub: {:?})",
        folders.len(),
        mods_path,
        sub_path
    );

    Ok(folders)
}
