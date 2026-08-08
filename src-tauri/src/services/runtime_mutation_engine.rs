//! Executes runtime toggle batches: renames mod folders on disk first
//! (filesystem is truth), then syncs the matching `mods` rows.
//!
//! Does NOT maintain `object_runtime_projection` — after a successful batch,
//! callers refresh it via `repo::runtime_projection_repo`.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use sqlx::SqlitePool;

use crate::domain::errors::{AppError, CollectionError};
use crate::domain::workspace::WorkspacePathRewrite;
use crate::services::fs_utils::file_utils::rename_cross_drive_fallback;
use crate::services::mods::core_ops::standardize_prefix;

#[derive(Debug, Clone)]
pub struct RuntimeToggleTarget {
    pub id: String,
    pub folder_path: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeToggleOperation {
    pub id: String,
    pub folder_path: String,
    pub target_enabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeToggleBatchRequest {
    pub game_id: String,
    pub mods_path: PathBuf,
    pub operations: Vec<RuntimeToggleOperation>,
}

#[derive(Debug, Clone)]
pub struct RuntimeToggleResult {
    pub enabled_count: usize,
    pub disabled_count: usize,
    pub warnings: Vec<String>,
    pub path_rewrites: Vec<WorkspacePathRewrite>,
}

#[derive(Debug, Clone)]
struct RenamePlan {
    id: String,
    new_rel: String,
    requested_abs: PathBuf,
    old_abs: PathBuf,
    new_abs: PathBuf,
    target_enabled: bool,
    disabled_reason: Option<String>,
}

/// Map a failed folder rename to a structured error, so a locked folder keeps
/// its `FileInUse` / `PathBusy` classification all the way to the UI.
fn classify_rename_failure(src: &std::path::Path, error: std::io::Error) -> CollectionError {
    match crate::services::mods::core_ops::map_toggle_error(src, "mod folder", error) {
        AppError::FileInUse { path, processes } => CollectionError::FileInUse { path, processes },
        AppError::PathBusy { path } => CollectionError::PathBusy { path },
        other => CollectionError::Io(other.to_string()),
    }
}

pub async fn toggle_mods_mixed(
    pool: &SqlitePool,
    request: RuntimeToggleBatchRequest,
) -> Result<RuntimeToggleResult, CollectionError> {
    if request.operations.is_empty() {
        return Ok(empty_result());
    }

    // Planning either yields a plan, skips a no-op, or aborts the whole batch —
    // it never produces warnings, so an empty plan set is just an empty result.
    let mut plans = Vec::new();
    for operation in &request.operations {
        match build_plan(&request.mods_path, operation) {
            Ok(Some(plan)) => plans.push(plan),
            Ok(None) => {}
            Err(error) => return Err(CollectionError::Validation(error.to_string())),
        }
    }

    if plans.is_empty() {
        return Ok(empty_result());
    }
    validate_plans(&plans).map_err(|error| CollectionError::Validation(error.to_string()))?;

    // Only rollback populates warnings, and rollback cannot run before this point.
    let mut warnings = Vec::new();

    let mut renamed = Vec::new();
    for plan in &plans {
        if plan.old_abs == plan.new_abs {
            renamed.push(plan.clone());
            continue;
        }

        match rename_cross_drive_fallback(&plan.old_abs, &plan.new_abs) {
            Ok(()) => renamed.push(plan.clone()),
            Err(error) => {
                rollback_successes(&renamed, &mut warnings);
                // Classify before stringifying, so a folder held by the game
                // reports the holding process rather than "Access is denied".
                return Err(classify_rename_failure(&plan.old_abs, error));
            }
        }
    }

    if let Err(error) = commit_db(pool, &request, &renamed).await {
        rollback_successes(&renamed, &mut warnings);
        return Err(CollectionError::Db(format!(
            "Runtime DB update failed after filesystem rename: {error}; rollback attempted"
        )));
    }
    let enabled_count = renamed.iter().filter(|plan| plan.target_enabled).count();
    let disabled_count = renamed.len().saturating_sub(enabled_count);

    Ok(RuntimeToggleResult {
        enabled_count,
        disabled_count,
        warnings,
        path_rewrites: renamed
            .iter()
            .filter(|plan| plan.requested_abs != plan.new_abs)
            .map(|plan| WorkspacePathRewrite {
                old_path: plan.requested_abs.to_string_lossy().to_string(),
                new_path: plan.new_abs.to_string_lossy().to_string(),
            })
            .collect(),
    })
}

fn empty_result() -> RuntimeToggleResult {
    RuntimeToggleResult {
        enabled_count: 0,
        disabled_count: 0,
        warnings: Vec::new(),
        path_rewrites: Vec::new(),
    }
}

fn build_plan(
    mods_path: &Path,
    operation: &RuntimeToggleOperation,
) -> Result<Option<RenamePlan>, AppError> {
    validate_relative_path(&operation.folder_path)?;

    let requested_abs = mods_path.join(&operation.folder_path);
    let old_abs = crate::services::mods::core_ops::resolve_existing_runtime_variant(
        mods_path,
        &requested_abs,
        operation.target_enabled,
    )
    .unwrap_or_else(|| requested_abs.clone());
    if !old_abs.exists() {
        return Err(AppError::Internal(format!(
            "Mod folder does not exist: {}",
            old_abs.display()
        )));
    }

    let old_name = old_abs
        .file_name()
        .ok_or_else(|| {
            AppError::Internal(format!("Mod path has no file name: {}", old_abs.display()))
        })?
        .to_string_lossy()
        .to_string();
    let new_name = standardize_prefix(&old_name, operation.target_enabled);
    if new_name == old_name && requested_abs == old_abs {
        return Ok(None);
    }

    let new_abs = old_abs.with_file_name(&new_name);
    if new_abs.exists() && new_abs != old_abs {
        return Err(AppError::Internal(format!(
            "Target folder already exists: {}",
            new_abs.display()
        )));
    }

    let new_rel = new_abs
        .strip_prefix(mods_path)
        .map_err(|_| {
            AppError::Security(format!(
                "Resolved path escaped mods root: {}",
                new_abs.display()
            ))
        })?
        .to_string_lossy()
        .to_string();

    Ok(Some(RenamePlan {
        id: operation.id.clone(),
        new_rel,
        requested_abs,
        old_abs,
        new_abs,
        target_enabled: operation.target_enabled,
        disabled_reason: operation.disabled_reason.clone(),
    }))
}

fn validate_plans(plans: &[RenamePlan]) -> Result<(), AppError> {
    let mut old_paths = HashSet::new();
    let mut new_paths = HashSet::new();

    for plan in plans {
        if !old_paths.insert(normalize_for_collision(&plan.old_abs)) {
            return Err(AppError::Internal(format!(
                "Duplicate mutation source path detected: {}",
                plan.old_abs.display()
            )));
        }

        if !new_paths.insert(normalize_for_collision(&plan.new_abs)) {
            return Err(AppError::Internal(format!(
                "Duplicate mutation target path detected: {}",
                plan.new_abs.display()
            )));
        }
    }

    Ok(())
}

fn normalize_for_collision(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

async fn commit_db(
    pool: &SqlitePool,
    request: &RuntimeToggleBatchRequest,
    plans: &[RenamePlan],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mods_path = request.mods_path.to_string_lossy().to_string();

    for plan in plans {
        let affected = crate::repo::mod_repo::update_mod_runtime_toggle(
            &mut tx,
            &request.game_id,
            &plan.id,
            &plan.new_rel,
            &mods_path,
            plan.target_enabled,
            plan.disabled_reason.as_deref(),
        )
        .await?;

        if affected != 1 {
            return Err(sqlx::Error::RowNotFound);
        }
    }

    tx.commit().await
}

fn rollback_successes(plans: &[RenamePlan], warnings: &mut Vec<String>) {
    for plan in plans.iter().rev() {
        if plan.old_abs == plan.new_abs {
            continue;
        }

        if !plan.new_abs.exists() {
            warnings.push(format!(
                "Rollback skipped for '{}': source missing",
                plan.new_abs.display()
            ));
            continue;
        }

        if plan.old_abs.exists() {
            warnings.push(format!(
                "Rollback skipped for '{}': target already exists",
                plan.old_abs.display()
            ));
            continue;
        }

        if let Err(error) = rename_cross_drive_fallback(&plan.new_abs, &plan.old_abs) {
            warnings.push(format!(
                "Rollback failed for '{}' to '{}': {error}",
                plan.new_abs.display(),
                plan.old_abs.display()
            ));
        }
    }
}

fn validate_relative_path(path: &str) -> Result<(), AppError> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() {
        return Err(AppError::Internal("Mod folder path is empty".to_string()));
    }

    if path.is_absolute() {
        return Err(AppError::Internal(format!(
            "Absolute mod folder path is not allowed: {}",
            path.display()
        )));
    }

    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(AppError::Internal(format!(
                    "Unsafe mod folder path is not allowed: {}",
                    path.display()
                )));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests/runtime_mutation_engine_tests.rs"]
mod tests;
