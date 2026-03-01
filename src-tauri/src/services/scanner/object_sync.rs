use crate::types::errors::CommandResult;
use std::collections::HashSet;

pub async fn sync_objects_for_game(pool: &sqlx::SqlitePool, game_id: &str) -> CommandResult<()> {
    // Phase 1: Filesystem as source of truth for instance existence
    // We scan the mod folder for this game and ensure a basic DB object exists for it
    if let Ok(Some(mod_path)) = crate::database::game_repo::get_mod_path(pool, game_id).await {
        let p = std::path::Path::new(&mod_path);
        if p.exists() && p.is_dir() {
            let mut fs_folders = HashSet::new();
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let folder_name = entry.file_name().to_string_lossy().to_string();
                        // Ignore hidden config dirs but DO NOT ignore DISABLED-prefixed folders
                        if !folder_name.starts_with('.') {
                            fs_folders.insert(folder_name);
                        }
                    }
                }
            }

            let current_objects_paths =
                crate::database::object_repo::get_objects_folder_paths(pool, game_id).await?;

            // Build a set of normalized (prefix-stripped, lowercased) DB folder names for dedup.
            // This ensures "DISABLED Acheron" (FS) matches "Acheron" (DB) and vice versa.
            let mut db_folders_normalized = HashSet::new();
            for fp in &current_objects_paths {
                let norm = crate::services::scanner::core::normalizer::normalize_display_name(fp);
                db_folders_normalized.insert(norm.to_lowercase());
            }

            let mut new_objects_count = 0;
            if let Ok(mut tx) = pool.begin().await {
                let mut changes = false;
                for folder in &fs_folders {
                    let norm =
                        crate::services::scanner::core::normalizer::normalize_display_name(folder);
                    if !db_folders_normalized.contains(&norm.to_lowercase()) {
                        changes = true;
                        let obj_name =
                            crate::services::scanner::core::normalizer::normalize_display_name(
                                folder,
                            );

                        let _ = crate::services::scanner::sync::helpers::ensure_object_exists(
                            &mut tx,
                            game_id,
                            folder,    // folder_path
                            &obj_name, // stripped alias name
                            "Other",   // default obj_type
                            None,
                            "[]",
                            "{}",
                            &mut new_objects_count,
                        )
                        .await;
                    }
                }
                if changes {
                    let _ = tx.commit().await;
                }
            }

            // Fix stale folder_path: update objects.folder_path to match the actual FS name.
            // Matches on normalized (prefix-stripped, lowercased) names so DB "Acheron"
            // gets corrected to FS "DISABLED Acheron" after a toggle.
            {
                let fs_norm_map: std::collections::HashMap<String, &String> = fs_folders
                    .iter()
                    .map(|f| {
                        let norm =
                            crate::services::scanner::core::normalizer::normalize_display_name(f);
                        (norm.to_lowercase(), f)
                    })
                    .collect();
                for fp in &current_objects_paths {
                    let db_norm =
                        crate::services::scanner::core::normalizer::normalize_display_name(fp)
                            .to_lowercase();
                    if let Some(actual) = fs_norm_map.get(&db_norm) {
                        if fp != *actual {
                            let _ = crate::database::object_repo::update_object_folder_path(
                                pool, game_id, fp, actual,
                            )
                            .await;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
