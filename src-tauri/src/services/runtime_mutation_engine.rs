//! Executes runtime toggle batches: renames mod folders on disk first
//! (filesystem is truth), then syncs the matching `mods` rows.
//!
//! Does NOT maintain `object_runtime_projection` — after a successful batch,
//! callers refresh it via `repo::runtime_projection_repo`.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use sqlx::SqlitePool;

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
    requested_abs: PathBuf,
    old_abs: PathBuf,
    new_abs: PathBuf,
    target_enabled: bool,
    disabled_reason: Option<String>,
}

pub async fn toggle_mods_mixed(
    pool: &SqlitePool,
    request: RuntimeToggleBatchRequest,
) -> Result<RuntimeToggleResult, String> {
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
            Err(error) => return Err(error),
        }
    }

    if plans.is_empty() {
        return Ok(empty_result());
    }
    validate_plans(&plans)?;

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

    let requested_abs = mods_path.join(&operation.folder_path);
    let old_abs = crate::services::mods::core_ops::resolve_existing_runtime_variant(
        mods_path,
        &requested_abs,
        operation.target_enabled,
    )
    .unwrap_or_else(|| requested_abs.clone());
    if !old_abs.exists() {
        return Err(format!("Mod folder does not exist: {}", old_abs.display()));
    }

    let old_name = old_abs
        .file_name()
        .ok_or_else(|| format!("Mod path has no file name: {}", old_abs.display()))?
        .to_string_lossy()
        .to_string();
    let new_name = standardize_prefix(&old_name, operation.target_enabled);
    if new_name == old_name && requested_abs == old_abs {
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
        requested_abs,
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
    use crate::domain::models::{GameType, ItemStatus};
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

    #[tokio::test]
    async fn toggle_mods_mixed_repairs_stale_disabled_db_path_when_disk_is_enabled() {
        let ctx = init_test_db().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        let enabled_path = mods_path.join("Variant");
        std::fs::create_dir_all(&enabled_path).expect("mod folder");
        let mods_path_string = mods_path.to_string_lossy().to_string();

        insert_test_game(
            &ctx.pool,
            &TestGameFixture {
                id: "game-runtime-toggle-repair",
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
                id: "mod-runtime-toggle-repair",
                game_id: "game-runtime-toggle-repair",
                object_id: None,
                actual_name: "Variant",
                folder_path: "DISABLED Variant",
                status: ItemStatus::Disabled,
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
                game_id: "game-runtime-toggle-repair".to_string(),
                mods_path: mods_path.clone(),
                operations: vec![RuntimeToggleOperation {
                    id: "mod-runtime-toggle-repair".to_string(),
                    folder_path: "DISABLED Variant".to_string(),
                    target_enabled: true,
                    disabled_reason: None,
                }],
            },
        )
        .await
        .expect("toggle");

        assert_eq!(result.changed_count, 1);
        assert_eq!(result.path_rewrites.len(), 1);
        assert_eq!(
            result.path_rewrites[0].old_path,
            mods_path
                .join("DISABLED Variant")
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(
            result.path_rewrites[0].new_path,
            enabled_path.to_string_lossy().to_string()
        );

        let row: (String, i64) =
            sqlx::query_as("SELECT folder_path, status FROM mods WHERE id = ?")
                .bind("mod-runtime-toggle-repair")
                .fetch_one(&ctx.pool)
                .await
                .expect("mod row");
        assert_eq!(row.0, "Variant");
        assert_eq!(row.1, ItemStatus::Enabled as i64);
    }

    #[tokio::test]
    async fn toggle_mods_mixed_returns_empty_result_for_empty_operations() {
        let ctx = init_test_db().await;

        let result = toggle_mods_mixed(
            &ctx.pool,
            RuntimeToggleBatchRequest {
                game_id: "game-1".to_string(),
                mods_path: PathBuf::from("does-not-matter"),
                operations: Vec::new(),
            },
        )
        .await
        .expect("empty batch is a no-op");

        assert_eq!(result.changed_count, 0);
        assert_eq!(result.enabled_count, 0);
        assert_eq!(result.disabled_count, 0);
        assert!(result.warnings.is_empty());
        assert!(result.path_rewrites.is_empty());
    }

    #[tokio::test]
    async fn toggle_mods_mixed_rejects_absolute_and_traversal_paths() {
        let ctx = init_test_db().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let absolute = temp.path().join("Variant").to_string_lossy().to_string();

        let absolute_error = toggle_mods_mixed(
            &ctx.pool,
            RuntimeToggleBatchRequest {
                game_id: "game-1".to_string(),
                mods_path: temp.path().to_path_buf(),
                operations: vec![RuntimeToggleOperation {
                    id: "mod-1".to_string(),
                    folder_path: absolute,
                    target_enabled: false,
                    disabled_reason: None,
                }],
            },
        )
        .await
        .expect_err("absolute path must be rejected");
        assert!(
            absolute_error.contains("Absolute mod folder path is not allowed"),
            "{absolute_error}"
        );

        let traversal_error = toggle_mods_mixed(
            &ctx.pool,
            RuntimeToggleBatchRequest {
                game_id: "game-1".to_string(),
                mods_path: temp.path().to_path_buf(),
                operations: vec![RuntimeToggleOperation {
                    id: "mod-1".to_string(),
                    folder_path: "../Escape".to_string(),
                    target_enabled: false,
                    disabled_reason: None,
                }],
            },
        )
        .await
        .expect_err("parent traversal must be rejected");
        assert!(
            traversal_error.contains("Unsafe mod folder path is not allowed"),
            "{traversal_error}"
        );
    }

    #[tokio::test]
    async fn toggle_mods_mixed_errors_when_mod_folder_is_missing_on_disk() {
        let ctx = init_test_db().await;
        let temp = tempfile::tempdir().expect("tempdir");

        let error = toggle_mods_mixed(
            &ctx.pool,
            RuntimeToggleBatchRequest {
                game_id: "game-1".to_string(),
                mods_path: temp.path().to_path_buf(),
                operations: vec![RuntimeToggleOperation {
                    id: "mod-1".to_string(),
                    folder_path: "Ghost".to_string(),
                    target_enabled: false,
                    disabled_reason: None,
                }],
            },
        )
        .await
        .expect_err("missing folder must error");
        assert!(error.contains("Mod folder does not exist"), "{error}");
    }

    #[tokio::test]
    async fn toggle_mods_mixed_rejects_duplicate_source_paths_before_renaming() {
        let ctx = init_test_db().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        std::fs::create_dir_all(mods_path.join("Variant")).expect("mod folder");

        let operation = RuntimeToggleOperation {
            id: "mod-1".to_string(),
            folder_path: "Variant".to_string(),
            target_enabled: false,
            disabled_reason: None,
        };
        let error = toggle_mods_mixed(
            &ctx.pool,
            RuntimeToggleBatchRequest {
                game_id: "game-1".to_string(),
                mods_path: mods_path.clone(),
                operations: vec![operation.clone(), operation],
            },
        )
        .await
        .expect_err("duplicate source must be rejected");

        assert!(
            error.contains("Duplicate mutation source path detected"),
            "{error}"
        );
        // Validation happens before any rename: disk untouched.
        assert!(mods_path.join("Variant").exists());
        assert!(!mods_path.join("DISABLED Variant").exists());
    }

    #[tokio::test]
    async fn toggle_mods_mixed_rolls_back_filesystem_rename_when_db_commit_fails() {
        let ctx = init_test_db().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        std::fs::create_dir_all(mods_path.join("Orphan")).expect("mod folder");
        let mods_path_string = mods_path.to_string_lossy().to_string();

        insert_test_game(
            &ctx.pool,
            &TestGameFixture {
                id: "game-rollback",
                name: "Game",
                game_type: GameType::GIMI,
                path: temp.path().to_string_lossy().as_ref(),
                mods_path: Some(&mods_path_string),
            },
        )
        .await
        .expect("insert game");

        // No matching mods row exists, so the DB commit fails after the
        // filesystem rename already happened.
        let error = toggle_mods_mixed(
            &ctx.pool,
            RuntimeToggleBatchRequest {
                game_id: "game-rollback".to_string(),
                mods_path: mods_path.clone(),
                operations: vec![RuntimeToggleOperation {
                    id: "mod-not-in-db".to_string(),
                    folder_path: "Orphan".to_string(),
                    target_enabled: false,
                    disabled_reason: None,
                }],
            },
        )
        .await
        .expect_err("db failure must surface");

        assert!(error.contains("rollback attempted"), "{error}");
        // The folder must be restored to its original name.
        assert!(mods_path.join("Orphan").exists());
        assert!(!mods_path.join("DISABLED Orphan").exists());
    }
}
