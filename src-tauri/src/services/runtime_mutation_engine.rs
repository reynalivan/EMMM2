use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use sqlx::SqlitePool;

use crate::domain::workspace::WorkspacePathRewrite;
use crate::services::fs_utils::file_utils::rename_cross_drive_fallback;
use crate::services::mods::core_ops::standardize_prefix;
use crate::services::path_key::folder_path_key;

#[derive(Debug, Clone)]
pub struct RuntimeToggleTarget {
    pub id: String,
    pub folder_path: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeToggleRequest {
    pub game_id: String,
    pub mods_path: PathBuf,
    pub targets: Vec<RuntimeToggleTarget>,
    pub target_enabled: bool,
    pub disabled_reason: Option<String>,
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
    pub changed_count: usize,
    pub enabled_count: usize,
    pub disabled_count: usize,
    pub warnings: Vec<String>,
    pub path_rewrites: Vec<WorkspacePathRewrite>,
}

#[derive(Debug, Clone)]
struct RenamePlan {
    id: String,
    new_rel: String,
    old_abs: PathBuf,
    new_abs: PathBuf,
    target_enabled: bool,
    disabled_reason: Option<String>,
}

pub async fn toggle_mods(
    pool: &SqlitePool,
    request: RuntimeToggleRequest,
) -> Result<RuntimeToggleResult, String> {
    let operations = request
        .targets
        .into_iter()
        .map(|target| RuntimeToggleOperation {
            id: target.id,
            folder_path: target.folder_path,
            target_enabled: request.target_enabled,
            disabled_reason: request.disabled_reason.clone(),
        })
        .collect();

    toggle_mods_mixed(
        pool,
        RuntimeToggleBatchRequest {
            game_id: request.game_id,
            mods_path: request.mods_path,
            operations,
        },
    )
    .await
}

pub async fn toggle_mods_mixed(
    pool: &SqlitePool,
    request: RuntimeToggleBatchRequest,
) -> Result<RuntimeToggleResult, String> {
    if request.operations.is_empty() {
        return Ok(empty_result());
    }

    let mut plans = Vec::new();
    let mut warnings = Vec::new();
    for operation in &request.operations {
        match build_plan(&request.mods_path, operation) {
            Ok(Some(plan)) => plans.push(plan),
            Ok(None) => {}
            Err(error) => return Err(error),
        }
    }

    if plans.is_empty() {
        return Ok(RuntimeToggleResult {
            warnings,
            ..empty_result()
        });
    }
    validate_plans(&plans)?;

    let mut renamed = Vec::new();
    for plan in &plans {
        match rename_cross_drive_fallback(&plan.old_abs, &plan.new_abs) {
            Ok(()) => renamed.push(plan.clone()),
            Err(error) => {
                rollback_successes(&renamed, &mut warnings);
                return Err(format!(
                    "Failed to rename '{}' to '{}': {error}",
                    plan.old_abs.display(),
                    plan.new_abs.display()
                ));
            }
        }
    }

    if let Err(error) = commit_db(pool, &request, &renamed).await {
        rollback_successes(&renamed, &mut warnings);
        return Err(format!(
            "Runtime DB update failed after filesystem rename: {error}; rollback attempted"
        ));
    }
    let enabled_count = renamed.iter().filter(|plan| plan.target_enabled).count();
    let disabled_count = renamed.len().saturating_sub(enabled_count);

    Ok(RuntimeToggleResult {
        changed_count: renamed.len(),
        enabled_count,
        disabled_count,
        warnings,
        path_rewrites: renamed
            .iter()
            .filter(|plan| plan.old_abs != plan.new_abs)
            .map(|plan| WorkspacePathRewrite {
                old_path: plan.old_abs.to_string_lossy().to_string(),
                new_path: plan.new_abs.to_string_lossy().to_string(),
            })
            .collect(),
    })
}

fn empty_result() -> RuntimeToggleResult {
    RuntimeToggleResult {
        changed_count: 0,
        enabled_count: 0,
        disabled_count: 0,
        warnings: Vec::new(),
        path_rewrites: Vec::new(),
    }
}

fn build_plan(
    mods_path: &Path,
    operation: &RuntimeToggleOperation,
) -> Result<Option<RenamePlan>, String> {
    validate_relative_path(&operation.folder_path)?;

    let old_abs = mods_path.join(&operation.folder_path);
    if !old_abs.exists() {
        return Err(format!("Mod folder does not exist: {}", old_abs.display()));
    }

    let old_name = old_abs
        .file_name()
        .ok_or_else(|| format!("Mod path has no file name: {}", old_abs.display()))?
        .to_string_lossy()
        .to_string();
    let new_name = standardize_prefix(&old_name, operation.target_enabled);
    if new_name == old_name {
        return Ok(None);
    }

    let new_abs = old_abs.with_file_name(&new_name);
    if new_abs.exists() && new_abs != old_abs {
        return Err(format!(
            "Target folder already exists: {}",
            new_abs.display()
        ));
    }

    let new_rel = new_abs
        .strip_prefix(mods_path)
        .map_err(|_| format!("Resolved path escaped mods root: {}", new_abs.display()))?
        .to_string_lossy()
        .to_string();

    Ok(Some(RenamePlan {
        id: operation.id.clone(),
        new_rel,
        old_abs,
        new_abs,
        target_enabled: operation.target_enabled,
        disabled_reason: operation.disabled_reason.clone(),
    }))
}

fn validate_plans(plans: &[RenamePlan]) -> Result<(), String> {
    let mut old_paths = HashSet::new();
    let mut new_paths = HashSet::new();

    for plan in plans {
        if !old_paths.insert(normalize_for_collision(&plan.old_abs)) {
            return Err(format!(
                "Duplicate mutation source path detected: {}",
                plan.old_abs.display()
            ));
        }

        if !new_paths.insert(normalize_for_collision(&plan.new_abs)) {
            return Err(format!(
                "Duplicate mutation target path detected: {}",
                plan.new_abs.display()
            ));
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
        let status = if plan.target_enabled { 1i32 } else { 0i32 };
        let result = sqlx::query(
            r#"
            UPDATE mods
            SET folder_path = ?, folder_path_key = ?, status = ?, disabled_reason = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ? AND game_id = ?
            "#,
        )
        .bind(&plan.new_rel)
        .bind(folder_path_key(&plan.new_rel, Some(&mods_path)))
        .bind(status)
        .bind(plan.disabled_reason.as_deref())
        .bind(&plan.id)
        .bind(&request.game_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() != 1 {
            return Err(sqlx::Error::RowNotFound);
        }
    }

    tx.commit().await
}

fn rollback_successes(plans: &[RenamePlan], warnings: &mut Vec<String>) {
    for plan in plans.iter().rev() {
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

fn validate_relative_path(path: &str) -> Result<(), String> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() {
        return Err("Mod folder path is empty".to_string());
    }

    if path.is_absolute() {
        return Err(format!(
            "Absolute mod folder path is not allowed: {}",
            path.display()
        ));
    }

    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(format!(
                    "Unsafe mod folder path is not allowed: {}",
                    path.display()
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::{GameType, ItemStatus};
    use crate::test_utils::{
        init_test_db, insert_test_game, insert_test_mod, TestGameFixture, TestModFixture,
    };

    #[tokio::test]
    async fn toggle_mods_mixed_returns_runtime_path_rewrites() {
        let ctx = init_test_db().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        std::fs::create_dir_all(mods_path.join("Variant")).expect("mod folder");
        let mods_path_string = mods_path.to_string_lossy().to_string();

        insert_test_game(
            &ctx.pool,
            &TestGameFixture {
                id: "game-runtime-toggle",
                name: "Game",
                game_type: GameType::GIMI,
                path: temp.path().to_string_lossy().as_ref(),
                mods_path: Some(&mods_path_string),
            },
        )
        .await
        .expect("insert game");
        insert_test_mod(
            &ctx.pool,
            &TestModFixture {
                id: "mod-runtime-toggle",
                game_id: "game-runtime-toggle",
                object_id: None,
                actual_name: "Variant",
                folder_path: "Variant",
                status: ItemStatus::Enabled,
                is_safe: true,
                object_type: Some("Character"),
                mods_path: Some(&mods_path_string),
            },
        )
        .await
        .expect("insert mod");

        let result = toggle_mods_mixed(
            &ctx.pool,
            RuntimeToggleBatchRequest {
                game_id: "game-runtime-toggle".to_string(),
                mods_path: mods_path.clone(),
                operations: vec![RuntimeToggleOperation {
                    id: "mod-runtime-toggle".to_string(),
                    folder_path: "Variant".to_string(),
                    target_enabled: false,
                    disabled_reason: None,
                }],
            },
        )
        .await
        .expect("toggle");

        assert_eq!(result.path_rewrites.len(), 1);
        assert_eq!(
            result.path_rewrites[0].old_path,
            mods_path.join("Variant").to_string_lossy().to_string()
        );
        assert_eq!(
            result.path_rewrites[0].new_path,
            mods_path
                .join("DISABLED Variant")
                .to_string_lossy()
                .to_string()
        );
    }
}
