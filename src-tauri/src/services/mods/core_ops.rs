use std::path::Path;

use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::fs_utils::path_utils;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

static DISABLED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(disabled|disable|dis)[_\-\s]*").unwrap());

pub fn standardize_prefix(folder_name: &str, target_enabled: bool) -> String {
    let clean_name = DISABLED_RE.replace(folder_name, "").trim().to_string();
    let valid_name = if clean_name.is_empty() {
        folder_name
    } else {
        &clean_name
    };

    if target_enabled {
        return valid_name.to_string();
    }

    format!("DISABLED {valid_name}")
}

pub async fn toggle_mod_inner(
    state: &WatcherState,
    path: String,
    enable: bool,
) -> Result<String, String> {
    // Hold suppression for the entire function so watcher events don't
    // leak through between the fs::rename and function return.
    let _guard = SuppressionGuard::new(&state.suppressor);

    let src = Path::new(&path);
    if !src.exists() || !src.is_dir() {
        return Err(format!("Mod folder does not exist: {path}"));
    }

    let parent = src.parent().unwrap_or_else(|| Path::new(""));
    let old_name = src.file_name().unwrap_or_default().to_string_lossy();

    let new_name = standardize_prefix(&old_name, enable);
    if new_name == old_name {
        return Ok(path);
    }

    let new_path = parent.join(&new_name);

    // Guard: target already exists → rename collision (both X and DISABLED X on disk)
    if new_path.exists() {
        let base = crate::services::scanner::core::normalizer::normalize_display_name(&old_name);
        return Err(format!(
            r#"{{"type":"RenameConflict","attempted_target":"{}","existing_path":"{}","base_name":"{}"}}"#,
            new_path
                .to_string_lossy()
                .replace('\\', "\\\\")
                .replace('"', "\\\""),
            new_path
                .to_string_lossy()
                .replace('\\', "\\\\")
                .replace('"', "\\\""),
            base.replace('"', "\\\""),
        ));
    }

    crate::services::fs_utils::file_utils::rename_cross_drive_fallback(src, &new_path)
        .map_err(|e| format!("Failed to rename mod folder: {e}"))?;

    log::info!("Toggled mod: '{}' -> '{}'", old_name, new_path.display());

    Ok(new_path.to_string_lossy().to_string())
}

pub async fn toggle_mod_inner_service(
    pool: &sqlx::SqlitePool,
    state: &WatcherState,
    op_lock: &OperationLock,
    path: String,
    enable: bool,
    game_id: &str,
) -> Result<String, String> {
    let _lock = op_lock.acquire().await?;

    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Failed to fetch game mods path".to_string())?;

    let base = Path::new(&mods_path);

    if !path_utils::is_path_safe(base, Path::new(&path)) {
        return Err("Security Error: Path attempts to escape mods directory bounds".to_string());
    }

    let new_absolute_path = toggle_mod_inner(state, path.clone(), enable).await?;
    let new_status = if enable { "ENABLED" } else { "DISABLED" };

    let disabled_reason = if enable { None } else { Some("USER") };

    // DB stores folder_path as absolute (set by scanner). Try absolute first, then relative fallback.
    let abs_result = crate::database::mod_repo::update_mod_path_status_and_reason(
        pool,
        game_id,
        &path,
        &new_absolute_path,
        new_status,
        disabled_reason,
    )
    .await;

    let rows_updated = match &abs_result {
        Ok(()) => {
            // Check if any rows were actually affected by querying the new path
            let check = crate::database::mod_repo::get_mod_id_and_status_by_path_pool(
                pool,
                &new_absolute_path,
                game_id,
            )
            .await;
            if check.ok().flatten().is_some() {
                1
            } else {
                0
            }
        }
        Err(_) => 0,
    };

    if rows_updated == 0 {
        // Fallback: try relative paths (legacy or migrated DBs)
        let new_rel = Path::new(&new_absolute_path)
            .strip_prefix(base)
            .unwrap_or(Path::new(&new_absolute_path))
            .to_string_lossy()
            .to_string();

        let old_rel = Path::new(&path)
            .strip_prefix(base)
            .unwrap_or(Path::new(&path))
            .to_string_lossy()
            .to_string();

        let rel_result = crate::database::mod_repo::update_mod_path_status_and_reason(
            pool,
            game_id,
            &old_rel,
            &new_rel,
            new_status,
            disabled_reason,
        )
        .await;

        if let Err(e) = &rel_result {
            log::warn!(
                "toggle_mod DB update failed for '{}' (both abs and rel): {}",
                path,
                e
            );
        }
    }

    // Update object folder_path and child paths if this is a top-level folder
    let old_rel = Path::new(&path)
        .strip_prefix(base)
        .unwrap_or(Path::new(&path))
        .to_string_lossy()
        .to_string();
    let new_rel = Path::new(&new_absolute_path)
        .strip_prefix(base)
        .unwrap_or(Path::new(&new_absolute_path))
        .to_string_lossy()
        .to_string();

    let rel_components: Vec<_> = Path::new(&old_rel).components().collect();
    if rel_components.len() == 1 {
        let _ = crate::database::object_repo::update_object_folder_path(
            pool, game_id, &old_rel, &new_rel,
        )
        .await;

        // Also try updating with absolute paths for objects
        let _ = crate::database::object_repo::update_object_folder_path(
            pool,
            game_id,
            &path,
            &new_absolute_path,
        )
        .await;

        let old_prefix = format!("{}\\", old_rel);
        let new_prefix = format!("{}\\", new_rel);
        let old_prefix_fwd = format!("{}/", old_rel);
        let new_prefix_fwd = format!("{}/", new_rel);

        let _ =
            crate::database::mod_repo::update_child_paths(pool, game_id, &old_prefix, &new_prefix)
                .await;

        let _ = crate::database::mod_repo::update_child_paths(
            pool,
            game_id,
            &old_prefix_fwd,
            &new_prefix_fwd,
        )
        .await;

        // Also try absolute-path child updates
        let old_abs_prefix = format!("{}\\", path);
        let new_abs_prefix = format!("{}\\", new_absolute_path);
        let _ = crate::database::mod_repo::update_child_paths(
            pool,
            game_id,
            &old_abs_prefix,
            &new_abs_prefix,
        )
        .await;
    }

    Ok(new_absolute_path)
}

/// Toggle a mod on disk and sync all DB state (path, object, children).
/// Used by privacy corridor handoff and single-mod toggle.
pub async fn toggle_and_sync_db(
    pool: &sqlx::SqlitePool,
    watcher_state: &WatcherState,
    mods_path: &str,
    game_id: &str,
    id: &str,
    rel_path: &str,
    enable: bool,
) -> Result<String, String> {
    let abs_path = Path::new(mods_path)
        .join(rel_path)
        .to_string_lossy()
        .to_string();
    let new_abs = toggle_mod_inner(watcher_state, abs_path, enable).await?;

    let new_rel = Path::new(&new_abs)
        .strip_prefix(mods_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(new_abs.clone());

    if new_rel != rel_path {
        let _ = crate::database::mod_repo::update_mod_path_by_id(pool, id, &new_rel).await;

        // Top-level folder → also update object + children
        let rel_components: Vec<_> = Path::new(rel_path).components().collect();
        if rel_components.len() == 1 {
            let _ = crate::database::object_repo::update_object_folder_path(
                pool, game_id, rel_path, &new_rel,
            )
            .await;
            for (old_sep, new_sep) in [("\\", "\\"), ("/", "/")] {
                let _ = crate::database::mod_repo::update_child_paths(
                    pool,
                    game_id,
                    &format!("{}{}", rel_path, old_sep),
                    &format!("{}{}", new_rel, new_sep),
                )
                .await;
            }
        }
    }
    Ok(new_abs)
}

#[derive(Debug, Clone, Serialize)]
pub struct RenameResult {
    pub old_path: String,
    pub new_path: String,
    pub new_name: String,
}

pub async fn rename_mod_folder_inner(
    state: &WatcherState,
    folder_path: String,
    new_name: String,
) -> Result<RenameResult, String> {
    // Hold suppression for the entire function so watcher events don't
    // leak through between the fs::rename and function return.
    let _guard = SuppressionGuard::new(&state.suppressor);

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

    let new_folder_name =
        if crate::services::scanner::core::normalizer::is_disabled_folder(&old_folder_name) {
            format!("{}{}", crate::DISABLED_PREFIX, new_name)
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

    crate::services::fs_utils::file_utils::rename_cross_drive_fallback(path, &new_path)
        .map_err(|e| format!("Failed to rename folder: {e}"))?;

    update_info_json_name(&new_path, &new_name);

    log::info!("Renamed '{}' -> '{}'", old_folder_name, new_folder_name);

    Ok(RenameResult {
        old_path: folder_path,
        new_path: new_path.to_string_lossy().to_string(),
        new_name,
    })
}

fn update_info_json_name(folder_path: &Path, new_name: &str) {
    use crate::services::mods::info_json;
    if folder_path.join("info.json").exists() {
        let update = info_json::ModInfoUpdate {
            actual_name: Some(new_name.to_string()),
            ..Default::default()
        };
        let _ = info_json::update_info_json(folder_path, &update);
    }
}

pub async fn rename_mod_folder_inner_service(
    pool: &sqlx::SqlitePool,
    state: &WatcherState,
    op_lock: &OperationLock,
    old_path: String,
    new_name: String,
    game_id: &str,
) -> Result<RenameResult, String> {
    let _lock = op_lock.acquire().await?;

    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Failed to fetch game mods path".to_string())?;

    let base = Path::new(&mods_path);

    if !path_utils::is_path_safe(base, Path::new(&old_path)) {
        return Err("Security Error: Path attempts to escape directory bounds".to_string());
    }

    let new_absolute_path =
        rename_mod_folder_inner(state, old_path.clone(), new_name.clone()).await?;

    let old_rel = Path::new(&old_path)
        .strip_prefix(base)
        .unwrap_or(Path::new(&old_path))
        .to_string_lossy()
        .to_string();

    let new_rel = Path::new(&new_absolute_path.new_path)
        .strip_prefix(base)
        .unwrap_or(Path::new(&new_absolute_path.new_path))
        .to_string_lossy()
        .to_string();

    let _ = crate::database::mod_repo::update_mod_path_by_old_path(pool, &old_rel, &new_rel).await;

    let rel_components: Vec<_> = Path::new(&old_rel).components().collect();
    if rel_components.len() == 1 {
        let _ = crate::database::object_repo::update_object_folder_path(
            pool, game_id, &old_rel, &new_rel,
        )
        .await;

        let old_prefix = format!("{}\\", old_rel);
        let new_prefix = format!("{}\\", new_rel);
        let old_prefix_fwd = format!("{}/", old_rel);
        let new_prefix_fwd = format!("{}/", new_rel);

        let _ =
            crate::database::mod_repo::update_child_paths(pool, game_id, &old_prefix, &new_prefix)
                .await;

        let _ = crate::database::mod_repo::update_child_paths(
            pool,
            game_id,
            &old_prefix_fwd,
            &new_prefix_fwd,
        )
        .await;
    }

    Ok(RenameResult {
        old_path,
        new_path: new_absolute_path.new_path,
        new_name: new_absolute_path.new_name,
    })
}
