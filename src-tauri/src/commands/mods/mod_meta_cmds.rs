use crate::services::mod_files::{info_json, metadata};
use std::path::Path;

#[tauri::command]
pub async fn repair_orphan_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<usize, String> {
    use sqlx::Row;
    let orphans =
        sqlx::query("SELECT id, actual_name FROM mods WHERE game_id = ? AND object_id IS NULL")
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
        // Strip DISABLED prefix so we never create "DISABLED xyz" objects
        let clean_name = actual_name
            .strip_prefix(crate::DISABLED_PREFIX)
            .unwrap_or(&actual_name)
            .to_string();

        let mut new_objects_count = 0;
        let object_id = crate::services::scanner::sync::helpers::ensure_object_exists(
            &mut tx,
            &game_id,
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
    id: String,
    favorite: bool,
) -> Result<(), String> {
    sqlx::query("UPDATE mods SET is_favorite = ? WHERE id = ?")
        .bind(favorite)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let folder_path: Option<String> =
        sqlx::query_scalar("SELECT folder_path FROM mods WHERE id = ?")
            .bind(&id)
            .fetch_optional(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

    if let Some(path_str) = folder_path {
        let update = info_json::ModInfoUpdate {
            is_favorite: Some(favorite),
            ..Default::default()
        };
        let _ = info_json::update_info_json(Path::new(&path_str), &update);
    }
    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct RandomModResult {
    pub id: String,
    pub name: String,
    pub thumbnail_path: Option<String>,
}

#[tauri::command]
pub async fn pick_random_mod(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    is_safe: bool,
) -> Result<Option<RandomModResult>, String> {
    use rand::seq::SliceRandom;
    use sqlx::Row;

    let mut query = "SELECT id, actual_name, folder_path FROM mods WHERE game_id = ? AND status = 'DISABLED' AND folder_path NOT LIKE '%/.%' AND folder_path NOT LIKE '%\\.%'".to_string();
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

    let candidates: Vec<(String, String, String)> = rows
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

    if candidates.is_empty() {
        return Ok(None);
    }

    let mut rng = rand::thread_rng();
    if let Some((id, name, _path)) = candidates.choose(&mut rng) {
        Ok(Some(RandomModResult {
            id: id.clone(),
            name: name.clone(),
            thumbnail_path: None,
        }))
    } else {
        Ok(None)
    }
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

    let mut ini_files = Vec::new();
    for row in rows {
        let path_str: String = row.get("folder_path");
        let path = Path::new(&path_str);
        if path.exists() {
            let content = crate::services::scanner::core::walker::scan_folder_content(path, 3);
            ini_files.extend(content.ini_files);
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
    mod_id: String,
    target_object_id: String,
    status: Option<String>,
) -> Result<(), String> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM objects WHERE id = ?)")
        .bind(&target_object_id)
        .fetch_one(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    if !exists {
        return Err(format!("Target object does not exist: {target_object_id}"));
    }

    sqlx::query("UPDATE mods SET object_id = ? WHERE id = ?")
        .bind(&target_object_id)
        .bind(&mod_id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(ref status_val) = status {
        if status_val == "disabled" {
            sqlx::query("UPDATE mods SET is_enabled = 0 WHERE id = ?")
                .bind(&mod_id)
                .execute(&*pool)
                .await
                .map_err(|e| e.to_string())?;
        } else if status_val == "only-enable" {
            sqlx::query("UPDATE mods SET is_enabled = 0 WHERE object_id = ? AND id != ?")
                .bind(&target_object_id)
                .bind(&mod_id)
                .execute(&*pool)
                .await
                .map_err(|e| e.to_string())?;
            sqlx::query("UPDATE mods SET is_enabled = 1 WHERE id = ?")
                .bind(&mod_id)
                .execute(&*pool)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
