use crate::services::mod_files::{info_json, metadata};
use std::path::Path;

#[tauri::command]
pub async fn repair_orphan_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<usize, String> {
    use sqlx::Row;
    let orphans =
        sqlx::query("SELECT id, actual_name, folder_path FROM mods WHERE game_id = ? AND object_id IS NULL")
            .bind(&game_id)
            .fetch_all(pool.inner())
            .await
            .map_err(|e| format!("DB error: {}", e))?;

    if orphans.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let mut repaired = 0usize;

    for row in &orphans {
        let mod_id: String = row.try_get("id").map_err(|e| e.to_string())?;
        let actual_name: String = row.try_get("actual_name").map_err(|e| e.to_string())?;
        let mod_folder_path: String = row.try_get("folder_path").map_err(|e| e.to_string())?;

        // Strip DISABLED prefix so we never create "DISABLED xyz" objects
        let clean_name = actual_name
            .strip_prefix(crate::DISABLED_PREFIX)
            .unwrap_or(&actual_name)
            .to_string();

        // Derive object folder_path from actual FS path (parent of mod folder)
        // e.g. "E:\Mods\archeron\SomeMod" → parent folder name = "archeron"
        let obj_folder = std::path::Path::new(&mod_folder_path)
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| clean_name.clone());

        let mut new_objects_count = 0;
        let object_id = crate::services::scanner::sync::helpers::ensure_object_exists(
            &mut tx,
            &game_id,
            &obj_folder, // FS-derived folder path, not display name
            &clean_name,
            "Other", // obj_type
            None,    // db_thumbnail
            "[]",    // sqlite tags json
            "{}",    // sqlite metadata json
            &mut new_objects_count,
        )
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query("UPDATE mods SET object_id = ?, object_type = 'Other' WHERE id = ?")
            .bind(&object_id)
            .bind(&mod_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        repaired += 1;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(repaired)
}

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

#[tauri::command]
pub async fn toggle_favorite(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_path: String,
    favorite: bool,
) -> Result<(), String> {
    use sqlx::Row;

    // 1. Fetch game mods_path to compute relative path
    let game_mod_path: String = sqlx::query("SELECT mod_path FROM games WHERE id = ?")
        .bind(&game_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?
        .and_then(|gr| gr.try_get("mod_path").ok())
        .ok_or_else(|| "Game not found or has no mods_path".to_string())?;

    let base = std::path::Path::new(&game_mod_path);
    let rel_path = std::path::Path::new(&folder_path)
        .strip_prefix(base)
        .unwrap_or(std::path::Path::new(&folder_path))
        .to_string_lossy()
        .to_string();

    // 2. Update DB cache
    let _ = sqlx::query("UPDATE mods SET is_favorite = ? WHERE folder_path = ? AND game_id = ?")
        .bind(favorite)
        .bind(&rel_path)
        .bind(&game_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // 3. Update info.json on disk
    let full_path = std::path::Path::new(&folder_path);
    if full_path.exists() {
        let update = info_json::ModInfoUpdate {
            is_favorite: Some(favorite),
            ..Default::default()
        };
        let _ = info_json::update_info_json(full_path, &update);
    }

    Ok(())
}

#[tauri::command]
pub async fn toggle_mod_safe(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_path: String,
    safe: bool,
) -> Result<(), String> {
    use sqlx::Row;

    let game_mod_path: String = match sqlx::query("SELECT mod_path FROM games WHERE id = ?")
        .bind(&game_id)
        .fetch_optional(pool.inner())
        .await
    {
        Ok(Some(gr)) => gr.try_get("mod_path").unwrap_or_default(),
        _ => return Err("Game not found or has no mods_path".to_string()),
    };

    let base = std::path::Path::new(&game_mod_path);
    let rel_path = std::path::Path::new(&folder_path)
        .strip_prefix(base)
        .unwrap_or(std::path::Path::new(&folder_path))
        .to_string_lossy()
        .to_string();

    let object_id: Option<String> = sqlx::query_scalar("SELECT object_id FROM mods WHERE folder_path = ? AND game_id = ?")
        .bind(&rel_path)
        .bind(&game_id)
        .fetch_optional(pool.inner())
        .await
        .unwrap_or(None)
        .flatten();

    if let Some(oid) = object_id {
        let _ = sqlx::query("UPDATE objects SET is_safe = ? WHERE id = ?")
            .bind(safe)
            .bind(&oid)
            .execute(pool.inner())
            .await;
    }

    let full_path = std::path::Path::new(&folder_path);
    if full_path.exists() {
        let update = info_json::ModInfoUpdate {
            is_safe: Some(safe),
            ..Default::default()
        };
        let _ = info_json::update_info_json(full_path, &update);
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct RandomModProposal {
    pub object_id: String,
    pub object_name: String,
    pub mod_id: String,
    pub name: String,
    pub thumbnail_path: Option<String>,
    pub folder_path: String,
}

#[tauri::command]
pub async fn suggest_random_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    is_safe: bool,
) -> Result<Vec<RandomModProposal>, String> {
    use rand::seq::SliceRandom;
    use sqlx::Row;

    // 1. Get all Character objects for this game
    let characters =
        sqlx::query("SELECT id, name FROM objects WHERE game_id = ? AND object_type = 'Character'")
            .bind(&game_id)
            .fetch_all(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

    if characters.is_empty() {
        return Ok(Vec::new());
    }

    let mut proposals = Vec::new();

    // 2. For each character, find its disabled mods and randomly pick one
    for char_row in characters {
        let object_id: String = char_row.get("id");
        let object_name: String = char_row.get("name");

        let mut query = "SELECT id, actual_name, folder_path FROM mods WHERE object_id = ? AND status = 'DISABLED' AND folder_path NOT LIKE '%/.%' AND folder_path NOT LIKE '%\\.%'".to_string();
        if is_safe {
            query.push_str(" AND is_safe = 1");
        }

        let mods = sqlx::query(&query)
            .bind(&object_id)
            .fetch_all(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

        if mods.is_empty() {
            continue;
        }

        let candidates: Vec<(String, String, String)> = mods
            .into_iter()
            .filter_map(|row| {
                let path: String = row.get("folder_path");
                let path_obj = Path::new(&path);
                if let Some(name) = path_obj.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        return None;
                    }
                }
                Some((row.get("id"), row.get("actual_name"), path))
            })
            .collect();

        let mut rng = rand::thread_rng();
        if let Some((mod_id, name, path)) = candidates.choose(&mut rng) {
            proposals.push(RandomModProposal {
                object_id,
                object_name,
                mod_id: mod_id.clone(),
                name: name.clone(),
                thumbnail_path: None,
                folder_path: path.clone(),
            });
        }
    }

    Ok(proposals)
}

#[tauri::command]
pub async fn get_active_mod_conflicts(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<crate::services::scanner::conflict::ConflictInfo>, String> {
    use sqlx::Row;
    let rows = sqlx::query("SELECT folder_path FROM mods WHERE game_id = ? AND status = 'ENABLED'")
        .bind(&game_id)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut ini_files: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
    for row in rows {
        let path_str: String = row.get("folder_path");
        let path = Path::new(&path_str);
        if path.exists() {
            let content = crate::services::scanner::core::walker::scan_folder_content(path, 3);
            for ini in content.ini_files {
                ini_files.push((path.to_path_buf(), ini));
            }
        }
    }

    Ok(crate::services::scanner::conflict::detect_conflicts(
        &ini_files,
    ))
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
    use sqlx::Row;

    // 1. Get game mod_path
    let game_mod_path: String = match sqlx::query("SELECT mod_path FROM games WHERE id = ?")
        .bind(&game_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?
    {
        Some(gr) => gr.try_get("mod_path").unwrap_or_default(),
        None => return Err("Game not found".to_string()),
    };

    // 2. Get target object relative path
    let target_obj_rel: String = match sqlx::query("SELECT folder_path FROM objects WHERE id = ?")
        .bind(&target_object_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?
    {
        Some(or) => or.try_get("folder_path").unwrap_or_default(),
        None => return Err("Target object not found".to_string()),
    };

    let base_path = Path::new(&game_mod_path);
    let target_obj_path = base_path.join(&target_obj_rel);

    if !target_obj_path.exists() {
        std::fs::create_dir_all(&target_obj_path).map_err(|e| e.to_string())?;
    }

    let current_path = Path::new(&folder_path);
    if !current_path.exists() {
        return Err("Source mod folder does not exist".to_string());
    }

    let mod_folder_name = current_path.file_name().unwrap_or_default().to_string_lossy().into_owned();

    // 3. Determine if we need to change ENABLED/DISABLED prefix
    let is_currently_disabled = mod_folder_name.starts_with("DISABLED ");
    let mut new_mod_folder_name = mod_folder_name.clone();

    if let Some(ref status_val) = status {
        if status_val == "disabled" && !is_currently_disabled {
            new_mod_folder_name = format!("DISABLED {}", mod_folder_name);
        } else if status_val == "only-enable" && is_currently_disabled {
            new_mod_folder_name = mod_folder_name.strip_prefix("DISABLED ").unwrap_or(&mod_folder_name).to_string();
        }
    }

    let new_path = target_obj_path.join(&new_mod_folder_name);

    // 4. Perform the move (Rename)
    if current_path != new_path {
        std::fs::rename(current_path, &new_path).map_err(|e| e.to_string())?;
    }

    // 5. If "only-enable", we must disable all other mods in the target object
    if status.as_deref() == Some("only-enable") {
        if let Ok(entries) = std::fs::read_dir(&target_obj_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && &path != &new_path {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !name.starts_with("DISABLED ") && !name.starts_with('.') {
                        let disabled_name = format!("DISABLED {}", name);
                        let disabled_path = target_obj_path.join(disabled_name);
                        let _ = std::fs::rename(&path, &disabled_path);
                    }
                }
            }
        }
    }

    Ok(())
}
