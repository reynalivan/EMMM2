//! Enable/disable a mod folder on disk. Callers converge the DB through Disk
//! Reconcile after releasing the operation lock.

use super::naming::{
    find_existing_sibling_case_insensitive, rename_conflict_error, standardize_prefix,
    SiblingNameIndex,
};
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::guard::ValidatedPath;
use crate::shared::errors::AppError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ToggleRenamePlan {
    old_path: PathBuf,
    new_path: PathBuf,
}

impl ToggleRenamePlan {
    pub fn old_path(&self) -> &Path {
        &self.old_path
    }

    pub fn new_path(&self) -> &Path {
        &self.new_path
    }

    pub fn apply(&self, noun: &str) -> Result<(), AppError> {
        crate::platform::fs::file_utils::rename_cross_drive_fallback(&self.old_path, &self.new_path)
            .map_err(|error| map_toggle_error(&self.old_path, noun, error))
    }

    pub fn rollback(&self, noun: &str) -> Result<(), AppError> {
        crate::platform::fs::file_utils::rename_cross_drive_fallback(&self.new_path, &self.old_path)
            .map_err(|error| map_toggle_error(&self.new_path, noun, error))
    }

    pub fn rebase_paths(&mut self, rewrites: &[(PathBuf, PathBuf)]) {
        self.old_path = rebase_path(self.old_path.clone(), rewrites);
        self.new_path = rebase_path(self.new_path.clone(), rewrites);
    }
}

fn rebase_path(mut path: PathBuf, rewrites: &[(PathBuf, PathBuf)]) -> PathBuf {
    for (old_path, new_path) in rewrites {
        if let Ok(suffix) = path.strip_prefix(old_path) {
            path = new_path.join(suffix);
        }
    }
    path
}

pub fn plan_toggle_rename(src: &Path, enable: bool) -> Result<Option<ToggleRenamePlan>, AppError> {
    plan_toggle_rename_with_sibling_index(src, enable, None)
}

pub fn plan_toggle_rename_with_sibling_index(
    src: &Path,
    enable: bool,
    sibling_index: Option<&SiblingNameIndex>,
) -> Result<Option<ToggleRenamePlan>, AppError> {
    if !src.exists() || !src.is_dir() {
        return Err(AppError::Io(format!(
            "Mod folder does not exist: {}",
            src.display()
        )));
    }
    let old_name = src.file_name().unwrap_or_default().to_string_lossy();
    let new_name = standardize_prefix(&old_name, enable);
    if new_name == old_name {
        return Ok(None);
    }

    let parent = src
        .parent()
        .ok_or_else(|| AppError::Io("Invalid path".to_string()))?;
    let new_path = parent.join(&new_name);
    let existing_path = match sibling_index {
        Some(index) => index.find_collision(&new_name, src),
        None => find_existing_sibling_case_insensitive(parent, &new_name, src),
    };
    if let Some(existing_path) = existing_path {
        let base = crate::modules::workspace::domain::normalizer::normalize_display_name(&old_name);
        return Err(rename_conflict_error(&new_path, &existing_path, &base));
    }

    Ok(Some(ToggleRenamePlan {
        old_path: src.to_path_buf(),
        new_path,
    }))
}

/// Map a rename failure to a structured error, surfacing the locking
/// processes when the folder is busy.
pub(crate) fn map_toggle_error(src: &Path, noun: &str, error: std::io::Error) -> AppError {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        let processes = crate::platform::fs::locking::get_locking_processes(src);
        if !processes.is_empty() {
            return AppError::FileInUse {
                path: src.to_string_lossy().to_string(),
                processes,
            };
        }

        return AppError::PathBusy {
            path: src.to_string_lossy().to_string(),
        };
    }

    AppError::Io(format!("Failed to rename {noun}: {error}"))
}

/// Rename `src` to its enabled/disabled form on disk.
/// Returns `Ok(None)` when the folder already has the desired prefix state.
pub(crate) fn rename_toggle_on_disk(
    src: &Path,
    enable: bool,
    noun: &str,
) -> Result<Option<PathBuf>, AppError> {
    let Some(plan) = plan_toggle_rename(src, enable)? else {
        return Ok(None);
    };
    plan.apply(noun)?;
    Ok(Some(plan.new_path))
}

pub async fn toggle_mod_inner(
    state: &WatcherState,
    path: String,
    enable: bool,
) -> Result<String, AppError> {
    // Path-scoped: covers both spellings of the rename (same identity key)
    // and keeps suppressing through the async event tail after return.
    let _guard = state.suppressor.suppress_paths([&path]);

    let src = Path::new(&path);
    if !src.exists() || !src.is_dir() {
        return Err(AppError::Io(format!("Mod folder does not exist: {path}")));
    }

    let Some(new_path) = rename_toggle_on_disk(src, enable, "mod folder")? else {
        return Ok(path);
    };

    log::info!(
        "Toggled mod: '{}' -> '{}'",
        src.file_name().unwrap_or_default().to_string_lossy(),
        new_path.display()
    );

    Ok(new_path.to_string_lossy().to_string())
}

/// What a policy-checked toggle changed on disk.
pub struct ModTogglePolicyOutcome {
    pub new_absolute_path: String,
    /// Sibling variants the implicit swap auto-disabled (absolute paths, both
    /// spellings). They can live under other object roots, so the caller's
    /// reconcile scope must include them explicitly.
    pub swapped_paths: Vec<String>,
}

#[allow(clippy::too_many_arguments)] // Service boundary kept stable to preserve toggle and duplicate-resolution callers.
pub async fn toggle_mod_inner_service_with_duplicate_policy(
    pool: &sqlx::SqlitePool,
    state: &WatcherState,
    _op_guard: &crate::platform::fs::operation_lock::OpGuard,
    path: &ValidatedPath,
    enable: bool,
    game_id: &str,
    allow_duplicates: bool,
) -> Result<ModTogglePolicyOutcome, AppError> {
    let canonical_path = path;

    let mods_path = crate::modules::games::adapters::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Failed to fetch game mods path".to_string()))?;

    let base = Path::new(&mods_path);
    let rel_path = canonical_path
        .strip_prefix(base)
        .unwrap_or(canonical_path)
        .to_string_lossy()
        .to_string();
    // AC-29.1: Conflict Detection
    let mut swapped_paths = Vec::new();
    if enable && !allow_duplicates {
        let duplicates: Vec<crate::modules::library::domain::mods::DuplicateModInfo> =
            crate::modules::workspace::application::scanner::conflict::get_duplicates_for_mod_service(
                pool, &rel_path, game_id,
            )
            .await?;

        if !duplicates.is_empty() {
            // Implicit Swap: If ALL duplicates are variants, auto-disable them
            let all_variants = duplicates.iter().all(|d| d.is_variant);
            if all_variants {
                for dup in duplicates {
                    let dup_abs = Path::new(&mods_path)
                        .join(&dup.folder_path)
                        .to_string_lossy()
                        .to_string();
                    let dup_new = toggle_mod_inner(state, dup_abs.clone(), false).await?;
                    swapped_paths.push(dup_abs);
                    swapped_paths.push(dup_new);
                }
            } else {
                // Real conflict -> Signal frontend to show radio resolution modal
                return Err(AppError::DuplicateConflict(duplicates));
            }
        }
    }

    // Disk is the source of truth: the rename is the whole mutation. The DB
    // (status, folder_path, projection) converges via the scoped
    // InternalMutation reconcile the caller runs afterwards — the single
    // writer of those columns.
    let new_absolute_path =
        toggle_mod_inner(state, canonical_path.to_string_lossy().to_string(), enable).await?;

    Ok(ModTogglePolicyOutcome {
        new_absolute_path,
        swapped_paths,
    })
}
