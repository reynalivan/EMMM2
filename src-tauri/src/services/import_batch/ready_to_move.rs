use crate::domain::errors::AppError;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tauri::Manager;

pub async fn resolve_mod_inbox_root(
    app: &tauri::AppHandle,
    db: &SqlitePool,
    game_id: &str,
    path_override: Option<&str>,
) -> Result<PathBuf, AppError> {
    let (game_name, configured_path) =
        crate::repo::game_repo::get_ready_to_move_config(db, game_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Game '{game_id}'")))?;
    Ok(
        match path_override
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(path) => PathBuf::from(path),
            None => match configured_path {
                Some(path) if !path.trim().is_empty() => PathBuf::from(path),
                _ => app
                    .path()
                    .download_dir()
                    .map_err(AppError::from)?
                    .join("mods")
                    .join(sanitize_game_name(&game_name)),
            },
        },
    )
}

pub async fn validate_mod_inbox_root(
    db: &SqlitePool,
    game_id: &str,
    root: &Path,
) -> Result<(), AppError> {
    let Some(mods_path) = crate::repo::game_repo::get_mod_path(db, game_id).await? else {
        return Ok(());
    };
    let mods_root = Path::new(&mods_path).canonicalize().map_err(|error| {
        AppError::Validation(format!("Configured mods path is unavailable: {error}"))
    })?;
    let ready_root = canonicalize_future_path(root)?;
    if ready_root.starts_with(&mods_root) || mods_root.starts_with(&ready_root) {
        return Err(AppError::Validation(
            "Mod Inbox must be outside the configured mods workspace".to_string(),
        ));
    }
    Ok(())
}

fn canonicalize_future_path(path: &Path) -> Result<PathBuf, AppError> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let mut existing = path;
    let mut suffix = Vec::new();
    while !existing.exists() {
        let name = existing.file_name().ok_or_else(|| {
            AppError::Validation(format!(
                "Mod Inbox path has no existing ancestor: {}",
                path.display()
            ))
        })?;
        suffix.push(name.to_os_string());
        existing = existing.parent().ok_or_else(|| {
            AppError::Validation(format!(
                "Mod Inbox path has no existing ancestor: {}",
                path.display()
            ))
        })?;
    }
    let mut canonical = existing.canonicalize()?;
    for segment in suffix.into_iter().rev() {
        canonical.push(segment);
    }
    Ok(canonical)
}

fn sanitize_game_name(name: &str) -> String {
    let mut output = String::new();
    let mut separator_pending = false;
    for character in name.chars() {
        if character.is_alphanumeric() || matches!(character, '-' | '_') {
            if separator_pending && !output.is_empty() {
                output.push('-');
            }
            separator_pending = false;
            output.push(character);
        } else {
            separator_pending = true;
        }
    }
    if output.is_empty() {
        "game".to_string()
    } else {
        output
    }
}
