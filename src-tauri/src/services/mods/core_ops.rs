use std::path::Path;

use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::fs_utils::path_utils;
use crate::services::scanner::watcher::WatcherState;

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

    let new_absolute_path =
        crate::commands::mods::mod_core_cmds::toggle_mod_inner(state, path.clone(), enable).await?;
    let new_status = if enable { "ENABLED" } else { "DISABLED" };

    // DB stores folder_path as absolute (set by scanner). Try absolute first, then relative fallback.
    let abs_result = crate::database::mod_repo::update_mod_path_and_status(
        pool,
        game_id,
        &path,
        &new_absolute_path,
        new_status,
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

        let rel_result = crate::database::mod_repo::update_mod_path_and_status(
            pool, game_id, &old_rel, &new_rel, new_status,
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

pub async fn rename_mod_folder_inner_service(
    pool: &sqlx::SqlitePool,
    state: &WatcherState,
    op_lock: &OperationLock,
    old_path: String,
    new_name: String,
    game_id: &str,
) -> Result<crate::commands::mods::mod_core_cmds::RenameResult, String> {
    let _lock = op_lock.acquire().await?;

    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Failed to fetch game mods path".to_string())?;

    let base = Path::new(&mods_path);

    if !path_utils::is_path_safe(base, Path::new(&old_path)) {
        return Err("Security Error: Path attempts to escape directory bounds".to_string());
    }

    let new_absolute_path = crate::commands::mods::mod_core_cmds::rename_mod_folder_inner(
        state,
        old_path.clone(),
        new_name.clone(),
    )
    .await?;

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

    Ok(crate::commands::mods::mod_core_cmds::RenameResult {
        old_path,
        new_path: new_absolute_path.new_path,
        new_name: new_absolute_path.new_name,
    })
}
