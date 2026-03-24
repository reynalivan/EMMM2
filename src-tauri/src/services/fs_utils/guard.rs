use crate::services::config::ConfigService;
use std::path::{Path, PathBuf};

pub struct PathGuard;

impl PathGuard {
    /// Validates that `candidate_path` is within the configured `mods_path` for the given game.
    /// Returns the canonicalized absolute path if valid, or an error string if not.
    pub fn validate_path(
        config: &ConfigService,
        game_id: &str,
        candidate_path: &str,
    ) -> Result<PathBuf, String> {
        let settings = config.get_settings();
        let game = settings
            .games
            .iter()
            .find(|g| g.id == game_id)
            .ok_or_else(|| format!("Game not found: {}", game_id))?;

        let mods_root = &game.mod_path;
        if mods_root.as_os_str().is_empty() {
            return Err(format!("Mods path not configured for game: {}", game.name));
        }

        let canonical_root = mods_root
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize mods root: {}", e))?;

        let candidate = Path::new(candidate_path);
        let absolute_candidate = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            canonical_root.join(candidate)
        };

        let canonical_candidate = absolute_candidate
            .canonicalize()
            .map_err(|e| format!("Path does not exist or invalid: {}", e))?;

        if !canonical_candidate.starts_with(&canonical_root) {
            return Err(
                "Security Violation: Path escapes the configured mods directory".to_string(),
            );
        }

        Ok(canonical_candidate)
    }

    /// Validates that a filename (not a path) is safe and doesn't contain traversal components.
    pub fn validate_filename(filename: &str) -> Result<(), String> {
        if filename.trim().is_empty() {
            return Err("Filename cannot be empty".to_string());
        }

        let path = Path::new(filename);
        if path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(
                "Invalid filename: must not contain directory components or traversal".to_string(),
            );
        }

        Ok(())
    }
}
