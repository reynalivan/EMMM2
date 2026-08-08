//! Duplicate/conflict resolution against the DB projection.
//!
//! Distinct from `hash_scan`: this half is async service orchestration
//! over the mods table, not filesystem INI parsing.

use std::path::Path;

use crate::domain::errors::AppError;
use crate::domain::workspace::WorkspacePathRewrite;

/// Find all enabled mods in the same object as `folder_path` (i.e. duplicates/conflicts).
pub async fn get_duplicates_for_mod_service(
    pool: &sqlx::SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Vec<crate::domain::mods::DuplicateModInfo>, AppError> {
    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .unwrap_or_default();

    // Resolve the object_id for the given folder
    let object_id =
        crate::repo::mod_repo::get_object_id_by_folder_and_game(pool, folder_path, game_id)
            .await
            .map_err(|e| AppError::Io(format!("DB query failed: {e}")))?;

    let object_id = match object_id {
        Some(id) => id,
        None => return Ok(vec![]), // No object — no duplicates possible
    };

    let duplicates =
        crate::repo::mod_repo::get_enabled_duplicates(pool, &object_id, game_id, folder_path)
            .await
            .map_err(|e| AppError::Io(format!("DB duplicate query failed: {e}")))?;

    let mut result = Vec::new();
    let mut relevant_mod_ids: Vec<String> = Vec::new();

    for (mod_id, path, name) in duplicates {
        // Variant Detection (Epic 11 Alignment)
        let mut is_variant = false;
        let mut parent_path = String::new();

        if let (Some(target_parent), Some(dup_parent)) = (
            Path::new(folder_path).parent(),
            Path::new(path.as_stored()).parent(),
        ) {
            if target_parent == dup_parent {
                let (node_type, _, _) = crate::common::classifier::classify_folder(
                    &Path::new(&mods_path).join(target_parent),
                );
                if node_type == crate::common::classifier::NodeType::VariantContainer {
                    is_variant = true;
                    parent_path = target_parent.to_string_lossy().to_string();
                }
            }
        }

        result.push(crate::domain::mods::DuplicateModInfo {
            mod_id: mod_id.clone(),
            object_id: object_id.clone(),
            folder_path: path.into_stored(),
            actual_name: name,
            is_variant,
            parent_path,
        });
        relevant_mod_ids.push(mod_id);
    }

    // Include the target mod ID in the set to check for ignores
    let target_mod_id_search: Result<Option<(String, Option<String>, i64)>, sqlx::Error> =
        crate::repo::mod_repo::get_mod_id_and_status_by_path(pool, folder_path, game_id).await;

    let target_mod_id = match target_mod_id_search {
        Ok(Some((id, _, _))) => id,
        _ => String::new(),
    };

    if !target_mod_id.is_empty() {
        relevant_mod_ids.push(target_mod_id);
    }

    // Check if this specific combination is ignored
    let ignored = crate::repo::conflict_repo::is_conflict_ignored(
        pool,
        game_id,
        &object_id,
        &relevant_mod_ids,
    )
    .await
    .unwrap_or(false);

    if ignored {
        return Ok(vec![]);
    }

    Ok(result)
}

/// Enable a specific mod and disable all other enabled siblings for the same object.
/// Wrapped here to decouple the command layer from direct database queries and orchestration logic.
pub async fn enable_only_this_service(
    config: &crate::services::config::ConfigService,
    pool: &sqlx::SqlitePool,
    state: &crate::services::scanner::watcher::WatcherState,
    target_path: String,
    game_id: &str,
) -> Result<crate::services::mods::bulk::BulkResult, AppError> {
    use crate::services::mods::bulk::{BulkActionError, BulkResult};
    use crate::services::mods::core_ops::toggle_mod_inner;
    use std::path::Path;

    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found or has no mods path".to_string()))?;

    let target_rel = Path::new(&target_path)
        .strip_prefix(&mods_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| target_path.clone());

    let mut success = Vec::new();
    let mut failures = Vec::new();
    let mut db_updates = Vec::new();
    let mut path_rewrites = Vec::new();

    let target_object_id =
        crate::repo::mod_repo::get_object_id_by_folder_and_game(pool, &target_rel, game_id)
            .await
            .map_err(|e| AppError::Io(format!("DB query failed: {e}")))?;

    if let Some(object_id) = target_object_id {
        let sibling_paths = crate::repo::mod_repo::get_enabled_siblings_paths(
            pool,
            &object_id,
            game_id,
            &target_rel,
        )
        .await
        .map_err(|e| AppError::Io(format!("DB sibling query failed: {e}")))?;

        for sibling_rel in sibling_paths {
            let sibling_abs = Path::new(&mods_path)
                .join(&sibling_rel)
                .to_string_lossy()
                .to_string();
            match toggle_mod_inner(state, sibling_abs.clone(), false).await {
                Ok(new_abs_path) => {
                    let new_rel = Path::new(&new_abs_path)
                        .strip_prefix(&mods_path)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| new_abs_path.clone());

                    db_updates.push((
                        sibling_rel.clone(),
                        new_rel.clone(),
                        crate::domain::models::ItemStatus::Disabled,
                    ));
                    success.push(new_abs_path);

                    if sibling_rel != new_rel {
                        path_rewrites.push(WorkspacePathRewrite {
                            old_path: sibling_abs,
                            new_path: Path::new(&mods_path)
                                .join(&new_rel)
                                .to_string_lossy()
                                .to_string(),
                        });
                    }
                }
                Err(e) => failures.push(BulkActionError {
                    path: sibling_abs,
                    error: e,
                }),
            }
        }
    }

    match toggle_mod_inner(state, target_path.clone(), true).await {
        Ok(new_abs_path) => {
            let new_rel = Path::new(&new_abs_path)
                .strip_prefix(&mods_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| new_abs_path.clone());

            db_updates.push((
                target_rel.clone(),
                new_rel.clone(),
                crate::domain::models::ItemStatus::Enabled,
            ));
            success.push(new_abs_path);

            if target_rel != new_rel {
                path_rewrites.push(WorkspacePathRewrite {
                    old_path: target_path.clone(),
                    new_path: Path::new(&mods_path)
                        .join(&new_rel)
                        .to_string_lossy()
                        .to_string(),
                });
            }
        }
        Err(e) => failures.push(BulkActionError {
            path: target_path,
            error: e,
        }),
    }

    if !db_updates.is_empty() {
        if let Err(e) =
            crate::repo::mod_repo::batch_update_path_and_status(pool, game_id, &db_updates).await
        {
            log::error!(
                "Failed batch updating mod paths after enable-only-this: {}",
                e
            );
        }
    }

    crate::services::app::runtime_effects::finalize_mutation(
        pool,
        config,
        game_id,
        crate::services::app::runtime_effects::MutationOutcome::full_game(),
    )
    .await;

    let mut result = BulkResult::new(success, failures);
    result.path_rewrites = path_rewrites;
    Ok(result)
}
