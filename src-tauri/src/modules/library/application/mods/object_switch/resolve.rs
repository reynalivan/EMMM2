//! Locating an object root folder on disk when the stored path drifted,
//! and healing the DB back to what disk actually shows.

use crate::shared::errors::AppError;
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
            .join(
                crate::modules::library::application::mods::core_ops::standardize_prefix(
                    object_name,
                    false,
                ),
            )
            .to_string_lossy()
            .to_string(),
    );

    candidates
}

fn find_matching_object_root(mods_path: &Path, object_name: &str) -> Option<String> {
    let expected_key = crate::shared::path_key::canonical_name_key(object_name);
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

        let folder_key = crate::shared::path_key::canonical_name_key(&folder_name);
        if folder_key == expected_key {
            return Some(path.to_string_lossy().to_string());
        }
    }

    None
}

fn ensure_object_root_containment(
    canonical_mods_root: &Path,
    candidate: &Path,
) -> Result<(), AppError> {
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|error| AppError::Validation(format!("Object folder is unavailable: {error}")))?;
    if canonical_candidate == canonical_mods_root
        || !canonical_candidate.starts_with(canonical_mods_root)
    {
        return Err(AppError::Security(format!(
            "Object folder escapes the configured Mods root: {}",
            candidate.display()
        )));
    }
    Ok(())
}

pub(super) async fn heal_object_root_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    old_folder_path: &str,
    new_folder_path: &str,
    mods_path: &str,
) -> Result<(), AppError> {
    if old_folder_path == new_folder_path {
        return Ok(());
    }

    crate::modules::catalog::adapters::sqlite::object::update_object_runtime_folder_path(
        pool,
        game_id,
        old_folder_path,
        new_folder_path,
    )
    .await?;

    crate::modules::library::adapters::sqlite::mods::update_child_paths(
        pool,
        game_id,
        old_folder_path,
        new_folder_path,
        Some(mods_path),
    )
    .await?;

    if crate::modules::collections::application::collection::classify_collection_path_transition(
        old_folder_path,
        new_folder_path,
    ) == crate::modules::collections::application::collection::CollectionPathTransitionKind::SemanticMoveOrRename
    {
        let mut tx = pool.begin().await?;
        crate::modules::collections::application::collection::handle_object_renamed_tx(
            &mut tx,
            game_id,
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
        crate::modules::workspace::application::scanner::core::types::GameObject,
        String,
        String,
    ),
    AppError,
> {
    let object =
        crate::modules::catalog::adapters::sqlite::object::get_game_object_by_id(pool, object_id)
            .await?
            .filter(|object| object.game_id == game_id)
            .ok_or_else(|| AppError::NotFound(format!("Object not found: {object_id}")))?;
    let mods_path = crate::modules::games::adapters::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let mods_root = Path::new(&mods_path);
    let canonical_mods_root = mods_root
        .canonicalize()
        .map_err(|error| AppError::Validation(format!("Mods folder is unavailable: {error}")))?;

    for candidate in build_object_path_candidates(mods_root, &object.folder_path, &object.name) {
        if Path::new(&candidate).exists() {
            ensure_object_root_containment(&canonical_mods_root, Path::new(&candidate))?;
            return Ok((object, mods_path, candidate));
        }
    }

    if let Some(found_path) = find_matching_object_root(mods_root, &object.name) {
        ensure_object_root_containment(&canonical_mods_root, Path::new(&found_path))?;
        return Ok((object, mods_path, found_path));
    }

    Err(AppError::RuntimePathNotFound {
        target: object.name.clone(),
    })
}
