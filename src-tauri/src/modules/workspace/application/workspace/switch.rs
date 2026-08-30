//! Workspace switch orchestration.
//! Moved out of `commands::app::workspace_cmds` so the command layer stays a
//! thin State-extraction wrapper over this service.

use std::path::Path;

use crate::shared::errors::AppError;
use crate::modules::workspace::domain::workspace::{
    WorkspaceImpact, WorkspacePathRewrite, WorkspaceRefreshScope, WorkspaceSwitchDuplicate,
    WorkspaceSwitchInput, WorkspaceSwitchResolution, WorkspaceSwitchResult, WorkspaceSwitchStatus,
    WorkspaceSwitchTargetKind,
};
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;

fn map_duplicates(
    duplicates: Vec<crate::modules::library::domain::mods::DuplicateModInfo>,
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
    let mods_path = crate::modules::games::adapters::outbound::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let mods_root = Path::new(&mods_path);
    let target_path = Path::new(target_value);
    let absolute_target = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        mods_root.join(target_path)
    };
    let resolved_target = crate::modules::library::application::mods::core_ops::resolve_existing_runtime_variant(
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
        crate::modules::library::adapters::outbound::sqlite::mods::get_mod_id_and_status_by_path(pool, &relative_path, game_id).await?
    {
        changed_object_ids.push(object_id);
    }

    Ok((
        resolved_target.to_string_lossy().to_string(),
        changed_object_ids,
    ))
}

async fn run_enable_only_this(
    pool: &sqlx::SqlitePool,
    watcher_state: &WatcherState,
    _op_guard: &crate::platform::fs::operation_lock::OpGuard,
    target_path: String,
    game_id: &str,
    changed_object_ids: Vec<String>,
) -> Result<WorkspaceSwitchResult, AppError> {
    let result = crate::modules::workspace::application::scanner::conflict::enable_only_this_service(
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
        sync_warning: None,
    })
}

fn default_switch_refresh_scopes() -> Vec<WorkspaceRefreshScope> {
    vec![
        WorkspaceRefreshScope::WorkspaceChanged,
        WorkspaceRefreshScope::FolderStructureChanged,
        WorkspaceRefreshScope::ObjectRowsChanged,
        WorkspaceRefreshScope::RuntimeStateChanged,
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

pub async fn execute_switch(
    input: WorkspaceSwitchInput,
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    watcher_state: &WatcherState,
    op_guard: &crate::platform::fs::operation_lock::OpGuard,
) -> Result<WorkspaceSwitchResult, AppError> {
    // Workspace Switch owns explicit enable/disable actions.
    // Object targets must use object-switch semantics, never the mod-toggle service.
    if matches!(input.target.kind, WorkspaceSwitchTargetKind::ObjectId) {
        let outcome = crate::modules::library::application::mods::object_switch::toggle_object_root_service(
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

        let changed_folder_paths = if original_path == next_path {
            vec![next_path.clone()]
        } else {
            vec![original_path.clone(), next_path.clone()]
        };

        return Ok(WorkspaceSwitchResult {
            status,
            primary_path: Some(next_path.clone()),
            changed_folder_paths: changed_folder_paths.clone(),
            changed_object_ids: vec![object_id.clone()],
            duplicates: Vec::new(),
            impact: build_switch_impact(
                Some(&original_path),
                Some(&next_path),
                &changed_folder_paths,
                std::slice::from_ref(&object_id),
            ),
            sync_warning: None,
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
            pool,
            watcher_state,
            op_guard,
            target_path,
            &input.game_id,
            changed_object_ids,
        )
        .await?;
        return Ok(result);
    }

    // Derivation site: `target_path` was resolved from the client's target
    // value plus the DB, so containment is proven here rather than passed in.
    let validated_target =
        crate::platform::fs::guard::validate_path(config, &input.game_id, &target_path)?;
    let result = crate::modules::library::application::mods::core_ops::toggle_mod_inner_service_with_duplicate_policy(
        pool,
        watcher_state,
        op_guard,
        &validated_target,
        input.desired_enabled,
        &input.game_id,
        matches!(input.resolution, WorkspaceSwitchResolution::ForceEnable),
    )
    .await;

    let outcome = match result {
        Ok(outcome) => outcome,
        Err(AppError::DuplicateConflict(duplicates)) => {
            return Ok(WorkspaceSwitchResult {
                status: WorkspaceSwitchStatus::RequiresDuplicateResolution,
                primary_path: None,
                changed_folder_paths: Vec::new(),
                changed_object_ids: changed_object_ids.clone(),
                duplicates: map_duplicates(duplicates),
                impact: build_switch_impact(None, None, &[], &changed_object_ids),
                sync_warning: None,
            });
        }
        Err(error) => return Err(error),
    };
    let next_path = outcome.new_absolute_path;

    let status = if next_path == target_path {
        WorkspaceSwitchStatus::Noop
    } else {
        WorkspaceSwitchStatus::Applied
    };

    // Unconditional: the reconcile IS the DB write for this mutation (single
    // writer), and on a no-op it heals any drift the toggle found. Implicit
    // swap can disable variants under OTHER object roots, so their paths are
    // part of the scope too.
    let mut reconcile_paths = vec![target_path.clone(), next_path.clone()];
    reconcile_paths.extend(outcome.swapped_paths);
    Ok(WorkspaceSwitchResult {
        status,
        primary_path: Some(next_path.clone()),
        changed_folder_paths: reconcile_paths.clone(),
        changed_object_ids: changed_object_ids.clone(),
        duplicates: Vec::new(),
        impact: build_switch_impact(
            Some(&input.target.value),
            Some(&next_path),
            &reconcile_paths,
            &changed_object_ids,
        ),
        sync_warning: None,
    })
}

#[cfg(test)]
#[path = "tests/switch_tests.rs"]
mod tests;
