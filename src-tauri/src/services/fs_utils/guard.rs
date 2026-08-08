use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use std::path::{Component, Path, PathBuf};

/// A canonical path proven to live inside a configured mods root.
///
/// Only this module constructs one, so an API that takes `ValidatedPath`
/// cannot receive an unchecked client path — skipping the guard is a compile
/// error, not a review miss.
#[derive(Debug, Clone)]
pub struct ValidatedPath {
    canonical: PathBuf,
    original: String,
}

impl ValidatedPath {
    /// The path exactly as the caller supplied it.
    ///
    /// Canonicalization rewrites a Windows path to its `\\?\C:\...`
    /// extended-length form, which does not compare equal to the strings the
    /// database stores. Callers that key on the caller's spelling keep using
    /// this; callers that touch the filesystem deref to the canonical form.
    pub fn original(&self) -> &str {
        &self.original
    }

    pub fn into_path_buf(self) -> PathBuf {
        self.canonical
    }
}

impl std::ops::Deref for ValidatedPath {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.canonical
    }
}

impl AsRef<Path> for ValidatedPath {
    fn as_ref(&self) -> &Path {
        &self.canonical
    }
}

/// Resolves a path to its canonical form, reporting failure the same way at
/// every containment check so the two entry points below stay in step.
fn canonicalize_for_guard(path: &Path) -> Result<PathBuf, AppError> {
    path.canonicalize()
        .map_err(|e| AppError::Security(format!("Path does not exist or invalid: {}", e)))
}

/// Validates that `candidate_path` is within the configured `mods_path` for the given game.
/// Returns the canonicalized absolute path if valid, or `AppError::Security` if not.
pub fn validate_path(
    config: &ConfigService,
    game_id: &str,
    candidate_path: &str,
) -> Result<ValidatedPath, AppError> {
    let canonical_root = canonical_mods_root(config, game_id)?;
    validate_against_root(&canonical_root, candidate_path)
}

/// Batch form of [`validate_path`]: the mods root is resolved and
/// canonicalized once instead of once per candidate — a 500-item bulk toggle
/// previously canonicalized the same root 500 times.
pub fn validate_paths(
    config: &ConfigService,
    game_id: &str,
    candidate_paths: &[String],
) -> Result<Vec<ValidatedPath>, AppError> {
    let canonical_root = canonical_mods_root(config, game_id)?;
    candidate_paths
        .iter()
        .map(|candidate| validate_against_root(&canonical_root, candidate))
        .collect()
}

fn canonical_mods_root(config: &ConfigService, game_id: &str) -> Result<PathBuf, AppError> {
    // Read only the one field: `get_settings` would deep-clone every game,
    // keyword, and hotkey binding, and this runs per item in bulk loops.
    let mods_root = config
        .mods_root_for(game_id)
        .ok_or_else(|| AppError::Security(format!("Game not found: {}", game_id)))?;

    if mods_root.as_os_str().is_empty() {
        return Err(AppError::Security(format!(
            "Mods path not configured for game: {}",
            game_id
        )));
    }

    mods_root
        .canonicalize()
        .map_err(|e| AppError::Security(format!("Failed to canonicalize mods root: {}", e)))
}

fn validate_against_root(
    canonical_root: &Path,
    candidate_path: &str,
) -> Result<ValidatedPath, AppError> {
    let candidate = Path::new(candidate_path);
    let absolute_candidate = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        canonical_root.join(candidate)
    };

    let canonical_candidate = canonicalize_for_guard(&absolute_candidate)?;

    if !canonical_candidate.starts_with(canonical_root) {
        return Err(AppError::Security(
            "Security Violation: Path escapes the configured mods directory".to_string(),
        ));
    }

    Ok(ValidatedPath {
        canonical: canonical_candidate,
        original: candidate_path.to_string(),
    })
}

/// Validates that `candidate_path` is within the mods root of ANY configured
/// game. For commands that receive a target directory without a game id
/// (import/drop flows) — the invariant is that writes may only land inside a
/// configured mods tree, whichever game it belongs to.
pub fn validate_dir_in_configured_roots(
    config: &ConfigService,
    candidate_path: &str,
) -> Result<ValidatedPath, AppError> {
    let canonical_candidate = canonicalize_for_guard(Path::new(candidate_path))?;
    containment_in_any_root(config, canonical_candidate)
}

/// Like [`validate_dir_in_configured_roots`], but for a directory that may not
/// exist yet (import flows create the target folder). The deepest existing
/// ancestor is canonicalized and the remaining segments re-joined, so the
/// containment check still runs on a canonical path.
pub fn validate_future_dir_in_configured_roots(
    config: &ConfigService,
    candidate_path: &str,
) -> Result<ValidatedPath, AppError> {
    let candidate = Path::new(candidate_path);
    if !candidate.is_absolute() {
        return Err(AppError::Security(format!(
            "Target directory must be absolute: {candidate_path}"
        )));
    }
    if candidate
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(AppError::Security(
            "Security Violation: Target directory must not contain '..'".to_string(),
        ));
    }

    // Walk up to the deepest existing ancestor, canonicalize it, re-join.
    let mut existing = candidate;
    let mut suffix: Vec<&std::ffi::OsStr> = Vec::new();
    while !existing.exists() {
        let Some(name) = existing.file_name() else {
            return Err(AppError::Security(format!(
                "Target directory has no existing ancestor: {candidate_path}"
            )));
        };
        suffix.push(name);
        existing = existing.parent().ok_or_else(|| {
            AppError::Security(format!(
                "Target directory has no existing ancestor: {candidate_path}"
            ))
        })?;
    }

    let mut canonical = canonicalize_for_guard(existing)?;
    for segment in suffix.into_iter().rev() {
        canonical.push(segment);
    }

    containment_in_any_root(config, canonical)
}

fn containment_in_any_root(
    config: &ConfigService,
    canonical_candidate: PathBuf,
) -> Result<ValidatedPath, AppError> {
    let settings = config.get_settings();
    for game in &settings.games {
        if game.mod_path.as_os_str().is_empty() {
            continue;
        }

        let Ok(canonical_root) = game.mod_path.canonicalize() else {
            continue;
        };

        if canonical_candidate.starts_with(&canonical_root) {
            return Ok(ValidatedPath {
                original: canonical_candidate.to_string_lossy().to_string(),
                canonical: canonical_candidate,
            });
        }
    }

    Err(AppError::Security(
        "Security Violation: Target is outside every configured mods directory".to_string(),
    ))
}

#[cfg(test)]
#[path = "tests/guard_tests.rs"]
mod tests;
