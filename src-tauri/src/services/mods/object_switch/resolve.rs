//! Locating an object root folder on disk when the stored path drifted,
//! and healing the DB back to what disk actually shows.

use crate::domain::errors::AppError;
use std::path::Path;

fn build_object_path_candidates(
    mods_path: &Path,
    stored_folder_path: &str,
    object_name: &str,
) -> Vec<String> {
    let mut candidates = Vec::new();
    let stored_path = Path::new(stored_folder_path);
    if stored_path.is_absolute() {
        candidates.push(stored_path.to_string_lossy().to_string());
    } else {
        candidates.push(
            mods_path
                .join(stored_folder_path)
                .to_string_lossy()
                .to_string(),
        );
    }

    candidates.push(mods_path.join(object_name).to_string_lossy().to_string());
    candidates.push(
        mods_path
            .join(format!("{}{}", crate::DISABLED_PREFIX, object_name))
            .to_string_lossy()
            .to_string(),
    );

    candidates
}

fn find_matching_object_root(mods_path: &Path, object_name: &str) -> Option<String> {
    let expected_key = crate::common::path_key::canonical_name_key(object_name);
    let entries = std::fs::read_dir(mods_path).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(folder_name) = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
        else {
            continue;
        };

        let folder_key = crate::common::path_key::canonical_name_key(&folder_name);
        if folder_key == expected_key {
            return Some(path.to_string_lossy().to_string());
        }
    }

    None
}

async fn heal_object_root_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    old_folder_path: &str,
    new_folder_path: &str,
    mods_path: &str,
) -> Result<(), AppError> {
    if old_folder_path == new_folder_path {
        return Ok(());
    }

    crate::repo::object_repo::update_object_runtime_folder_path(
        pool,
        game_id,
        old_folder_path,
        new_folder_path,
    )
    .await?;

    for (old_sep, new_sep) in [
        (
            format!("{old_folder_path}\\"),
            format!("{new_folder_path}\\"),
        ),
        (format!("{old_folder_path}/"), format!("{new_folder_path}/")),
    ] {
        crate::repo::mod_repo::update_child_paths(
            pool,
            game_id,
            &old_sep,
            &new_sep,
            Some(mods_path),
        )
        .await?;
    }

    if crate::services::collection_service::classify_collection_path_transition(
        old_folder_path,
        new_folder_path,
    ) == crate::services::collection_service::CollectionPathTransitionKind::SemanticMoveOrRename
    {
        let mut tx = pool.begin().await?;
        crate::services::collection_service::handle_object_renamed_tx(
            &mut tx,
            old_folder_path,
            new_folder_path,
        )
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
        tx.commit().await?;
    }

    Ok(())
}

pub(super) async fn resolve_object_root_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: &str,
) -> Result<
    (
        crate::services::scanner::core::types::GameObject,
        String,
        String,
    ),
    AppError,
> {
    let object = crate::repo::object_repo::get_game_object_by_id(pool, object_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Object not found: {object_id}")))?;
    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let mods_root = Path::new(&mods_path);

    for candidate in build_object_path_candidates(mods_root, &object.folder_path, &object.name) {
        if Path::new(&candidate).exists() {
            let relative_candidate = Path::new(&candidate)
                .strip_prefix(mods_root)
                .ok()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or(candidate.clone());
            heal_object_root_path(
                pool,
                game_id,
                &object.folder_path,
                &relative_candidate,
                &mods_path,
            )
            .await?;
            return Ok((object, mods_path, candidate));
        }
    }

    if let Some(found_path) = find_matching_object_root(mods_root, &object.name) {
        let relative_candidate = Path::new(&found_path)
            .strip_prefix(mods_root)
            .ok()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or(found_path.clone());
        heal_object_root_path(
            pool,
            game_id,
            &object.folder_path,
            &relative_candidate,
            &mods_path,
        )
        .await?;
        return Ok((object, mods_path, found_path));
    }

    Err(AppError::RuntimePathNotFound {
        target: object.name.clone(),
    })
}
