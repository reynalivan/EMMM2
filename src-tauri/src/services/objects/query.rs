use std::collections::HashMap;

use crate::database::object_repo::{ObjectFilter, ObjectSummary};
use crate::services::scanner::core::normalizer::{is_disabled_folder, normalize_display_name};

pub async fn get_filtered_objects_with_conflict_check(
    pool: &sqlx::SqlitePool,
    filter: &ObjectFilter,
) -> Result<Vec<ObjectSummary>, String> {
    let mut objects = crate::database::object_repo::get_filtered_objects(pool, filter)
        .await
        .map_err(|e| e.to_string())?;

    let mod_path_opt = crate::database::game_repo::get_mod_path(pool, &filter.game_id)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(mod_path) = mod_path_opt {
        let mods_dir = std::path::Path::new(&mod_path);
        if mods_dir.is_dir() {
            let mut norm_set: HashMap<String, Vec<String>> = HashMap::new();
            if let Ok(entries) = std::fs::read_dir(mods_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !name.starts_with('.') {
                            let key = normalize_display_name(&name).to_lowercase();
                            norm_set.entry(key).or_default().push(name);
                        }
                    }
                }
            }

            for obj in &mut objects {
                let key = normalize_display_name(&obj.folder_path).to_lowercase();
                if let Some(variants) = norm_set.get(&key) {
                    let has_enabled = variants.iter().any(|v| !is_disabled_folder(v));
                    let has_disabled = variants.iter().any(|v| is_disabled_folder(v));
                    obj.has_naming_conflict = has_enabled && has_disabled;
                }
            }
        }
    }

    Ok(objects)
}

pub async fn get_category_counts_service(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    safe_mode: bool,
) -> Result<Vec<crate::database::object_repo::CategoryCount>, String> {
    crate::database::object_repo::get_category_counts(pool, game_id, safe_mode)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_object_by_id_service(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<crate::services::scanner::core::types::GameObject>, String> {
    crate::database::object_repo::get_game_object_by_id(pool, id)
        .await
        .map_err(|e| e.to_string())
}
