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

    let _ = crate::database::mod_repo::update_mod_path_and_status(
        pool, game_id, &old_rel, &new_rel, new_status,
    )
    .await;

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
        old_path: old_path,
        new_path: new_absolute_path.new_path,
        new_name: new_absolute_path.new_name,
    })
}
