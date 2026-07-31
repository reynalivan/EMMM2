use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use std::path::{Path, PathBuf};

/// Validates that `candidate_path` is within the configured `mods_path` for the given game.
/// Returns the canonicalized absolute path if valid, or `AppError::Security` if not.
pub fn validate_path(
    config: &ConfigService,
    game_id: &str,
    candidate_path: &str,
) -> Result<PathBuf, AppError> {
    let settings = config.get_settings();
    let game = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| AppError::Security(format!("Game not found: {}", game_id)))?;

    let mods_root = &game.mod_path;
    if mods_root.as_os_str().is_empty() {
        return Err(AppError::Security(format!(
            "Mods path not configured for game: {}",
            game.name
        )));
    }

    let canonical_root = mods_root
        .canonicalize()
        .map_err(|e| AppError::Security(format!("Failed to canonicalize mods root: {}", e)))?;

    let candidate = Path::new(candidate_path);
    let absolute_candidate = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        canonical_root.join(candidate)
    };

    let canonical_candidate = absolute_candidate
        .canonicalize()
        .map_err(|e| AppError::Security(format!("Path does not exist or invalid: {}", e)))?;

    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(AppError::Security(
            "Security Violation: Path escapes the configured mods directory".to_string(),
        ));
    }

    Ok(canonical_candidate)
}
