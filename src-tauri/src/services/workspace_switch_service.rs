//! Workspace switch orchestration.
//! Moved out of `commands::app::workspace_cmds` so the command layer stays a
//! thin State-extraction wrapper over this service.

use std::path::Path;

use crate::domain::errors::AppError;
use crate::domain::workspace::{
    WorkspaceImpact, WorkspacePathRewrite, WorkspaceRefreshScope, WorkspaceSwitchDuplicate,
    WorkspaceSwitchInput, WorkspaceSwitchResolution, WorkspaceSwitchResult, WorkspaceSwitchStatus,
    WorkspaceSwitchTargetKind,
};
use crate::services::config::ConfigService;
use crate::services::scanner::watcher::WatcherState;

fn map_duplicates(
    duplicates: Vec<crate::domain::mods::DuplicateModInfo>,
) -> Vec<WorkspaceSwitchDuplicate> {
    duplicates
        .into_iter()
        .map(|duplicate| WorkspaceSwitchDuplicate {
            mod_id: duplicate.mod_id,
            object_id: duplicate.object_id,
            folder_path: duplicate.folder_path,
            actual_name: duplicate.actual_name,
            is_variant: duplicate.is_variant,
            parent_path: duplicate.parent_path,
        })
        .collect()
}

async fn resolve_mod_target_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    target_value: &str,
    desired_enabled: bool,
) -> Result<(String, Vec<String>), AppError> {
    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let mods_root = Path::new(&mods_path);
    let target_path = Path::new(target_value);
    let absolute_target = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        mods_root.join(target_path)
    };
    let resolved_target = crate::services::mods::core_ops::resolve_existing_runtime_variant(
        mods_root,
        &absolute_target,
        desired_enabled,
    )
    .unwrap_or(absolute_target);
    let relative_path = resolved_target
        .strip_prefix(mods_root)
        .ok()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| target_value.to_string());

    let mut changed_object_ids = Vec::new();
    if let Some((_, Some(object_id), _)) =
        crate::repo::mod_repo::get_mod_id_and_status_by_path(pool, &relative_path, game_id).await?
    {
        changed_object_ids.push(object_id);
    }

    Ok((
        resolved_target.to_string_lossy().to_string(),
        changed_object_ids,
    ))
}

async fn run_enable_only_this(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    watcher_state: &WatcherState,
    _op_guard: &crate::services::fs_utils::operation_lock::OpGuard,
    target_path: String,
    game_id: &str,
    changed_object_ids: Vec<String>,
) -> Result<WorkspaceSwitchResult, AppError> {
    let result = crate::services::scanner::conflict::enable_only_this_service(
        config,
        pool,
        watcher_state,
        target_path,
        game_id,
    )
    .await?;
    let changed_folder_paths = result.success;
    let primary_path = changed_folder_paths.last().cloned();
    let rewrites = result.path_rewrites.clone();

    let mut impact = build_switch_impact(
        None,
        primary_path.as_deref(),
        &changed_folder_paths,
        &changed_object_ids,
    );
    if !rewrites.is_empty() {
        impact.rewrites = rewrites;
    }

    Ok(WorkspaceSwitchResult {
        status: WorkspaceSwitchStatus::Applied,
        primary_path: primary_path.clone(),
        changed_folder_paths: changed_folder_paths.clone(),
        changed_object_ids: changed_object_ids.clone(),
        duplicates: Vec::new(),
        impact,
    })
}

fn default_switch_refresh_scopes() -> Vec<WorkspaceRefreshScope> {
    vec![
        WorkspaceRefreshScope::WorkspaceChanged,
        WorkspaceRefreshScope::FolderStructureChanged,
        WorkspaceRefreshScope::ObjectRowsChanged,
        WorkspaceRefreshScope::CorridorChanged,
        WorkspaceRefreshScope::CollectionsChanged,
        WorkspaceRefreshScope::DashboardChanged,
        WorkspaceRefreshScope::ActiveKeybindingsChanged,
        WorkspaceRefreshScope::PreviewChanged,
        WorkspaceRefreshScope::ConflictsChanged,
    ]
}

fn build_switch_impact(
    original_path: Option<&str>,
    primary_path: Option<&str>,
    changed_folder_paths: &[String],
    changed_object_ids: &[String],
) -> WorkspaceImpact {
    let rewrites = match (original_path, primary_path) {
        (Some(old_path), Some(new_path)) if old_path != new_path => {
            vec![WorkspacePathRewrite {
                old_path: old_path.to_string(),
                new_path: new_path.to_string(),
            }]
        }
        _ => Vec::new(),
    };

    WorkspaceImpact {
        rewrites,
        changed_object_ids: changed_object_ids.to_vec(),
        changed_folder_paths: changed_folder_paths.to_vec(),
        refresh_scopes: default_switch_refresh_scopes(),
        warnings: Vec::new(),
    }
}

/// Convergence hook: scoped disk reconcile after a switch so DB matches disk
/// even if a manual sync step missed a case. Failure is logged, not fatal —
/// the FS + DB work already succeeded.
async fn reconcile_after_switch(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    changed_paths: Vec<String>,
) {
    if changed_paths.is_empty() {
        return;
    }

    if let Err(error) = crate::services::disk_reconcile::emit::emit_internal_disk_reconcile(
        app,
        pool,
        game_id,
        changed_paths,
    )
    .await
    {
        log::warn!("Post-switch disk reconcile failed: {error}");
    }
}

pub async fn execute_switch(
    app: &tauri::AppHandle,
    input: WorkspaceSwitchInput,
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    watcher_state: &WatcherState,
    op_guard: &crate::services::fs_utils::operation_lock::OpGuard,
) -> Result<WorkspaceSwitchResult, AppError> {
    // Workspace Switch owns explicit enable/disable actions.
    // Object targets must use object-switch semantics, never the mod-toggle service.
    if matches!(input.target.kind, WorkspaceSwitchTargetKind::ObjectId) {
        let outcome = crate::services::mods::object_switch::toggle_object_root_service(
            config,
            pool,
            watcher_state,
            op_guard,
            &input.game_id,
            &input.target.value,
            input.desired_enabled,
        )
        .await?;

        let status = if outcome.next_path == outcome.original_path {
            WorkspaceSwitchStatus::Noop
        } else {
            WorkspaceSwitchStatus::Applied
        };
        let next_path = outcome.next_path.clone();
        let original_path = outcome.original_path.clone();
        let object_id = outcome.object_id.clone();

        if status == WorkspaceSwitchStatus::Applied {
            reconcile_after_switch(
                app,
                pool,
                &input.game_id,
                vec![original_path.clone(), next_path.clone()],
            )
            .await;
        }

        return Ok(WorkspaceSwitchResult {
            status,
            primary_path: Some(next_path.clone()),
            changed_folder_paths: vec![next_path.clone()],
            changed_object_ids: vec![object_id.clone()],
            duplicates: Vec::new(),
            impact: build_switch_impact(
                Some(&original_path),
                Some(&next_path),
                std::slice::from_ref(&next_path),
                std::slice::from_ref(&object_id),
            ),
        });
    }

    let (target_path, changed_object_ids) = resolve_mod_target_path(
        pool,
        &input.game_id,
        &input.target.value,
        input.desired_enabled,
    )
    .await?;

    if matches!(input.resolution, WorkspaceSwitchResolution::EnableOnlyThis) {
        let result = run_enable_only_this(
            config,
            pool,
            watcher_state,
            op_guard,
            target_path,
            &input.game_id,
            changed_object_ids,
        )
        .await?;
        reconcile_after_switch(
            app,
            pool,
            &input.game_id,
            result.changed_folder_paths.clone(),
        )
        .await;
        return Ok(result);
    }

    // Derivation site: `target_path` was resolved from the client's target
    // value plus the DB, so containment is proven here rather than passed in.
    let validated_target =
        crate::services::fs_utils::guard::validate_path(config, &input.game_id, &target_path)?;
    let result = crate::services::mods::core_ops::toggle_mod_inner_service_with_duplicate_policy(
        config,
        pool,
        watcher_state,
        op_guard,
        &validated_target,
        input.desired_enabled,
        &input.game_id,
        matches!(input.resolution, WorkspaceSwitchResolution::ForceEnable),
    )
    .await;

    let next_path = match result {
        Ok(path) => path,
        Err(AppError::DuplicateConflict(duplicates)) => {
            return Ok(WorkspaceSwitchResult {
                status: WorkspaceSwitchStatus::RequiresDuplicateResolution,
                primary_path: None,
                changed_folder_paths: Vec::new(),
                changed_object_ids: changed_object_ids.clone(),
                duplicates: map_duplicates(duplicates),
                impact: build_switch_impact(None, None, &[], &changed_object_ids),
            });
        }
        Err(error) => return Err(error),
    };

    let status = if next_path == target_path {
        WorkspaceSwitchStatus::Noop
    } else {
        WorkspaceSwitchStatus::Applied
    };

    if status == WorkspaceSwitchStatus::Applied {
        reconcile_after_switch(
            app,
            pool,
            &input.game_id,
            vec![target_path.clone(), next_path.clone()],
        )
        .await;
    }

    Ok(WorkspaceSwitchResult {
        status,
        primary_path: Some(next_path.clone()),
        changed_folder_paths: vec![next_path.clone()],
        changed_object_ids: changed_object_ids.clone(),
        duplicates: Vec::new(),
        impact: build_switch_impact(
            Some(&input.target.value),
            Some(&next_path),
            std::slice::from_ref(&next_path),
            &changed_object_ids,
        ),
    })
}

#[cfg(test)]
#[path = "tests/workspace_switch_service_tests.rs"]
mod tests;
