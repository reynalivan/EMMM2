use std::path::Path;

use tauri::State;

use crate::domain::errors::AppError;
use crate::domain::workspace::{
    WorkspaceImpact, WorkspacePathRewrite, WorkspaceRefreshScope, WorkspaceSwitchDuplicate,
    WorkspaceSwitchInput, WorkspaceSwitchResolution, WorkspaceSwitchResult, WorkspaceSwitchStatus,
    WorkspaceSwitchTargetKind, WorkspaceViewModel, WorkspaceViewModelInput,
};
use crate::services::config::ConfigService;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::WatcherState;
use crate::types::errors::CommandResult;

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_view_model(
    input: WorkspaceViewModelInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<WorkspaceViewModel> {
    crate::services::workspace_service::get_workspace_view_model(pool.inner(), input)
        .await
        .map_err(crate::types::errors::CommandError::App)
}

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
) -> Result<(String, Vec<String>), AppError> {
    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let mods_root = Path::new(&mods_path);
    let relative_path = Path::new(target_value)
        .strip_prefix(mods_root)
        .ok()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| target_value.to_string());

    let mut changed_object_ids = Vec::new();
    if let Some((_, Some(object_id), _)) =
        crate::repo::mod_repo::get_mod_id_and_status_by_path_any(pool, &relative_path, game_id)
            .await?
    {
        changed_object_ids.push(object_id);
    }

    Ok((target_value.to_string(), changed_object_ids))
}

async fn resolve_switch_target_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    target_kind: WorkspaceSwitchTargetKind,
    target_value: &str,
) -> Result<(String, Vec<String>), AppError> {
    if matches!(target_kind, WorkspaceSwitchTargetKind::ModPath) {
        return resolve_mod_target_path(pool, game_id, target_value).await;
    }
    Ok((target_value.to_string(), vec![target_value.to_string()]))
}

async fn run_enable_only_this(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    watcher_state: &State<'_, WatcherState>,
    op_lock: &OperationLock,
    target_path: String,
    game_id: &str,
    changed_object_ids: Vec<String>,
) -> Result<WorkspaceSwitchResult, AppError> {
    let _lock = op_lock
        .acquire()
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

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

    Ok(WorkspaceSwitchResult {
        status: WorkspaceSwitchStatus::Applied,
        primary_path: primary_path.clone(),
        changed_folder_paths: changed_folder_paths.clone(),
        changed_object_ids: changed_object_ids.clone(),
        duplicates: Vec::new(),
        impact: build_switch_impact(
            None,
            primary_path.as_deref(),
            &changed_folder_paths,
            &changed_object_ids,
            false,
        ),
    })
}

fn default_switch_refresh_scopes() -> Vec<WorkspaceRefreshScope> {
    vec![
        WorkspaceRefreshScope::WorkspaceChanged,
        WorkspaceRefreshScope::FolderStructureChanged,
        WorkspaceRefreshScope::ObjectRowsChanged,
        WorkspaceRefreshScope::CorridorChanged,
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
    projection_dirty: bool,
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
        cleared_targets: Vec::new(),
        changed_object_ids: changed_object_ids.to_vec(),
        changed_folder_paths: changed_folder_paths.to_vec(),
        refresh_scopes: default_switch_refresh_scopes(),
        projection_dirty,
        warnings: Vec::new(),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn execute_workspace_switch(
    input: WorkspaceSwitchInput,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher_state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
) -> Result<WorkspaceSwitchResult, AppError> {
    // Workspace Switch owns explicit enable/disable actions.
    // Object targets must use object-switch semantics, never the mod-toggle service.
    if matches!(input.target.kind, WorkspaceSwitchTargetKind::ObjectId) {
        let outcome = crate::services::mods::object_switch::toggle_object_root_service(
            config.inner(),
            pool.inner(),
            watcher_state.inner(),
            op_lock.inner(),
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
                false,
            ),
        });
    }

    let (target_path, changed_object_ids) = resolve_switch_target_path(
        pool.inner(),
        &input.game_id,
        input.target.kind,
        &input.target.value,
    )
    .await?;

    if matches!(input.resolution, WorkspaceSwitchResolution::EnableOnlyThis) {
        return run_enable_only_this(
            config.inner(),
            pool.inner(),
            &watcher_state,
            op_lock.inner(),
            target_path,
            &input.game_id,
            changed_object_ids,
        )
        .await;
    }

    let result = crate::services::mods::core_ops::toggle_mod_inner_service_with_duplicate_policy(
        config.inner(),
        pool.inner(),
        watcher_state.inner(),
        op_lock.inner(),
        target_path.clone(),
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
                impact: build_switch_impact(None, None, &[], &changed_object_ids, false),
            });
        }
        Err(error) => return Err(error),
    };

    let status = if next_path == target_path {
        WorkspaceSwitchStatus::Noop
    } else {
        WorkspaceSwitchStatus::Applied
    };

    Ok(WorkspaceSwitchResult {
        status,
        primary_path: Some(next_path.clone()),
        changed_folder_paths: vec![next_path.clone()],
        changed_object_ids: changed_object_ids.clone(),
        duplicates: Vec::new(),
        impact: build_switch_impact(
            Some(&target_path),
            Some(&next_path),
            std::slice::from_ref(&next_path),
            &changed_object_ids,
            false,
        ),
    })
}
