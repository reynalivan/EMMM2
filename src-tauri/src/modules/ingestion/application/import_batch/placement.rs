use std::path::{Component, Path, PathBuf};

use crate::shared::errors::AppError;

/// Resolve ownership from the current object row, never from a display name or
/// a client-provided destination path. Preview and commit share this lookup.
pub(crate) async fn resolve_existing_object_directory(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: &str,
    mods_root: &Path,
) -> Result<PathBuf, AppError> {
    let (owner_game_id, folder_path) =
        crate::modules::catalog::adapters::sqlite::object::get_game_id_and_folder_path(
            pool, object_id,
        )
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Destination object '{object_id}'")))?;
    if owner_game_id != game_id {
        return Err(AppError::Security(
            "Destination object belongs to a different game".to_string(),
        ));
    }
    let folder_path = folder_path
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| AppError::Validation("Destination object has no folder path".to_string()))?;
    Ok(mods_root.join(folder_path))
}

pub(crate) fn append_target_subpath(
    root: &Path,
    subpath: Option<&str>,
) -> Result<PathBuf, AppError> {
    let Some(subpath) = subpath.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(root.to_path_buf());
    };
    let relative = Path::new(subpath);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(AppError::Security(
            "Import target subpath must contain only normal relative path segments".to_string(),
        ));
    }
    Ok(root.join(relative))
}
