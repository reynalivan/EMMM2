//! Workspace switch orchestration.
//! Moved out of `commands::app::workspace_cmds` so the command layer stays a
//! thin State-extraction wrapper over this service.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::modules::workspace::domain::workspace::{
    WorkspaceImpact, WorkspacePathRewrite, WorkspaceRefreshScope, WorkspaceSwitchDuplicate,
    WorkspaceSwitchInput, WorkspaceSwitchResolution, WorkspaceSwitchResult, WorkspaceSwitchStatus,
    WorkspaceSwitchTargetKind,
};
use crate::shared::errors::AppError;

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

#[derive(Debug, Clone)]
pub struct PreparedModSwitch {
    target_path: String,
    changed_object_ids: Vec<String>,
    batches: Vec<crate::modules::library::application::mods::bulk::PreparedBulkToggle>,
}

#[derive(Debug, Clone)]
pub enum PreparedWorkspaceSwitch {
    Immediate(WorkspaceSwitchResult),
    Object(crate::modules::library::application::mods::object_switch::PreparedObjectSwitch),
    Mod(PreparedModSwitch),
}

/// Distinguishes a failed mutation whose compensation completed from a failed
/// compensation. Callers with durable journals may only close a rollback after
/// the latter has been ruled out.
#[derive(Debug)]
pub(crate) enum PreparedWorkspaceExecutionError {
    Apply(AppError),
    Compensation(AppError),
}

impl PreparedWorkspaceExecutionError {
    pub(crate) fn into_app_error(self) -> AppError {
        match self {
            Self::Apply(error) | Self::Compensation(error) => error,
        }
    }
}

impl From<AppError> for PreparedWorkspaceExecutionError {
    fn from(error: AppError) -> Self {
        Self::Apply(error)
    }
}

impl PreparedWorkspaceSwitch {
    pub fn journal_steps(&self) -> Vec<(u32, PathBuf, PathBuf)> {
        match self {
            Self::Immediate(_) => Vec::new(),
            Self::Object(prepared) => prepared.journal_steps(),
            Self::Mod(prepared) => prepared
                .batches
                .iter()
                .flat_map(|batch| batch.planned_steps())
                .collect(),
        }
    }

    pub fn immediate_result(&self) -> Option<WorkspaceSwitchResult> {
        match self {
            Self::Immediate(result) => Some(result.clone()),
            Self::Object(_) | Self::Mod(_) => None,
        }
    }

    pub fn execute(
        &self,
        app: &tauri::AppHandle,
        watcher: &WatcherState,
    ) -> Result<WorkspaceSwitchResult, AppError> {
        self.execute_with_outcome(app, watcher)
            .map_err(PreparedWorkspaceExecutionError::into_app_error)
    }

    pub(crate) fn execute_with_outcome(
        &self,
        app: &tauri::AppHandle,
        watcher: &WatcherState,
    ) -> Result<WorkspaceSwitchResult, PreparedWorkspaceExecutionError> {
        match self {
            Self::Immediate(result) => Ok(result.clone()),
            Self::Object(prepared) => {
                let outcome = prepared.execute(watcher)?;
                let changed_folder_paths = if outcome.original_path == outcome.next_path {
                    vec![outcome.next_path.clone()]
                } else {
                    vec![outcome.original_path.clone(), outcome.next_path.clone()]
                };
                let status = if outcome.original_path == outcome.next_path {
                    WorkspaceSwitchStatus::Noop
                } else {
                    WorkspaceSwitchStatus::Applied
                };
                Ok(WorkspaceSwitchResult {
                    status,
                    primary_path: Some(outcome.next_path.clone()),
                    changed_folder_paths: changed_folder_paths.clone(),
                    changed_object_ids: vec![outcome.object_id.clone()],
                    duplicates: Vec::new(),
                    impact: build_switch_impact(
                        Some(&outcome.original_path),
                        Some(&outcome.next_path),
                        &changed_folder_paths,
                        std::slice::from_ref(&outcome.object_id),
                    ),
                    sync_warning: None,
                })
            }
            Self::Mod(prepared) => {
                let cancel = AtomicBool::new(false);
                let mut executions: Vec<(
                    &crate::modules::library::application::mods::bulk::PreparedBulkToggle,
                    Vec<u32>,
                )> = Vec::new();
                let mut success = Vec::new();
                let mut rewrites = Vec::new();
                for batch in &prepared.batches {
                    let execution = crate::modules::library::application::mods::bulk::execute_prepared_bulk_toggle(
                        app,
                        watcher,
                        batch,
                        &cancel,
                    );
                    if !execution.result.failures.is_empty() {
                        for (rollback_batch, rollback_sequences) in executions.iter().rev() {
                            if let Err(error) = crate::modules::library::application::mods::bulk::rollback_prepared_bulk_toggle(
                                watcher,
                                rollback_batch,
                                rollback_sequences,
                            ) {
                                return Err(PreparedWorkspaceExecutionError::Compensation(error));
                            }
                        }
                        if let Err(error) = crate::modules::library::application::mods::bulk::rollback_prepared_bulk_toggle(
                            watcher,
                            batch,
                            &execution.applied_sequences,
                        ) {
                            return Err(PreparedWorkspaceExecutionError::Compensation(error));
                        }
                        return Err(PreparedWorkspaceExecutionError::Apply(
                            execution.result.failures[0].error.clone(),
                        ));
                    }
                    success.extend(execution.result.success.iter().cloned());
                    rewrites.extend(execution.result.path_rewrites.iter().cloned());
                    executions.push((batch, execution.applied_sequences));
                }

                let primary_path = rewrites
                    .iter()
                    .find(|rewrite| rewrite.old_path == prepared.target_path)
                    .map(|rewrite| rewrite.new_path.clone())
                    .or_else(|| Some(prepared.target_path.clone()));
                let mut seen = HashSet::new();
                let changed_folder_paths = rewrites
                    .iter()
                    .flat_map(|rewrite| [rewrite.old_path.clone(), rewrite.new_path.clone()])
                    .chain(success)
                    .filter(|path| seen.insert(path.clone()))
                    .collect::<Vec<_>>();
                let status = if rewrites.is_empty() {
                    WorkspaceSwitchStatus::Noop
                } else {
                    WorkspaceSwitchStatus::Applied
                };
                let mut impact = build_switch_impact(
                    Some(&prepared.target_path),
                    primary_path.as_deref(),
                    &changed_folder_paths,
                    &prepared.changed_object_ids,
                );
                impact.rewrites = rewrites;
                Ok(WorkspaceSwitchResult {
                    status,
                    primary_path,
                    changed_folder_paths,
                    changed_object_ids: prepared.changed_object_ids.clone(),
                    duplicates: Vec::new(),
                    impact,
                    sync_warning: None,
                })
            }
        }
    }

    pub fn rollback(&self, watcher: &WatcherState) -> Result<(), AppError> {
        match self {
            Self::Immediate(_) => Ok(()),
            Self::Object(prepared) => prepared.rollback(watcher),
            Self::Mod(prepared) => {
                for batch in prepared.batches.iter().rev() {
                    crate::modules::library::application::mods::bulk::rollback_prepared_bulk_toggle(
                        watcher,
                        batch,
                        &batch.planned_sequences(),
                    )?;
                }
                Ok(())
            }
        }
    }
}

pub async fn prepare_switch(
    input: &WorkspaceSwitchInput,
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
) -> Result<PreparedWorkspaceSwitch, AppError> {
    if matches!(input.target.kind, WorkspaceSwitchTargetKind::ObjectId) {
        return Ok(PreparedWorkspaceSwitch::Object(
            crate::modules::library::application::mods::object_switch::prepare_object_root_switch(
                pool,
                &input.game_id,
                &input.target.value,
                input.desired_enabled,
            )
            .await?,
        ));
    }

    let (target_path, changed_object_ids) = resolve_mod_target_path(
        pool,
        &input.game_id,
        &input.target.value,
        input.desired_enabled,
    )
    .await?;
    let validated_target =
        crate::platform::fs::guard::validate_path(config, &input.game_id, &target_path)?;
    let mut disable_paths = Vec::new();
    if input.desired_enabled {
        let mods_root = config
            .mods_root_for(&input.game_id)
            .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
        let target_rel = Path::new(validated_target.original())
            .strip_prefix(&mods_root)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|_| validated_target.original().to_string());
        let duplicates = crate::modules::workspace::application::scanner::conflict::get_duplicates_for_mod_service(
            pool,
            &target_rel,
            &input.game_id,
        )
        .await?;
        if !duplicates.is_empty()
            && matches!(
                input.resolution,
                WorkspaceSwitchResolution::Normal
                    | WorkspaceSwitchResolution::EnableParentThenContinue
            )
        {
            return Ok(PreparedWorkspaceSwitch::Immediate(WorkspaceSwitchResult {
                status: WorkspaceSwitchStatus::RequiresDuplicateResolution,
                primary_path: None,
                changed_folder_paths: Vec::new(),
                changed_object_ids: changed_object_ids.clone(),
                duplicates: map_duplicates(duplicates),
                impact: build_switch_impact(None, None, &[], &changed_object_ids),
                sync_warning: None,
            }));
        }
        if matches!(
            input.resolution,
            WorkspaceSwitchResolution::ForceEnable | WorkspaceSwitchResolution::EnableOnlyThis
        ) {
            disable_paths.extend(
                duplicates
                    .into_iter()
                    .map(|duplicate| mods_root.join(duplicate.folder_path)),
            );
        }
    }

    let mut batches = Vec::new();
    let mut next_sequence = 0;
    if !disable_paths.is_empty() {
        let mut batch = crate::modules::library::application::mods::bulk::prepare_bulk_toggle(
            &disable_paths,
            false,
        );
        next_sequence = batch.resequence(next_sequence);
        batches.push(batch);
    }
    let mut target_batch = crate::modules::library::application::mods::bulk::prepare_bulk_toggle(
        &[validated_target.to_path_buf()],
        input.desired_enabled,
    );
    target_batch.resequence(next_sequence);
    batches.push(target_batch);
    Ok(PreparedWorkspaceSwitch::Mod(PreparedModSwitch {
        target_path,
        changed_object_ids,
        batches,
    }))
}

/// Prepares one all-or-nothing randomizer loadout. Every selected target is
/// enabled while all currently effective siblings of its Object are disabled
/// first. A disabled Object ancestor is the actual activation target, because
/// renaming only its already-enabled child would otherwise be a no-op.
pub async fn prepare_randomized_loadout_switch(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    target_values: &[String],
    exclusive_object_ids: &HashSet<String>,
) -> Result<PreparedWorkspaceSwitch, AppError> {
    if target_values.is_empty() {
        return Err(AppError::Validation(
            "A randomized loadout requires at least one mod".to_string(),
        ));
    }

    let mods_root = config
        .mods_root_for(game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    let mut disable_paths = Vec::<PathBuf>::new();
    let mut enable_paths = Vec::<PathBuf>::new();
    let mut changed_object_ids = Vec::<String>::new();
    let mut seen_objects = HashSet::<String>::new();
    let mut seen_disable = HashSet::<String>::new();
    let mut seen_enable = HashSet::<String>::new();
    let mut primary_target = None;

    for target_value in target_values {
        let (target_path, object_ids) =
            resolve_mod_target_path(pool, game_id, target_value, true).await?;
        let validated_target =
            crate::platform::fs::guard::validate_path(config, game_id, &target_path)?;
        let target_rel = Path::new(validated_target.original())
            .strip_prefix(&mods_root)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|_| validated_target.original().to_string());
        for object_id in object_ids {
            if exclusive_object_ids.contains(&object_id) {
                let sibling_paths =
                    crate::modules::library::adapters::sqlite::mods::get_enabled_siblings_paths(
                        pool,
                        &object_id,
                        game_id,
                        &target_rel,
                    )
                    .await?;
                for sibling_path in sibling_paths {
                    let path = mods_root.join(sibling_path);
                    let key = path.to_string_lossy().to_ascii_lowercase();
                    if seen_disable.insert(key) {
                        disable_paths.push(path);
                    }
                }
                let activation_path = crate::modules::workspace::application::scanner::conflict::activation_path_for_disabled_ancestor(
                    validated_target.as_ref(),
                    &mods_root,
                );
                let all_sibling_paths =
                    crate::modules::library::adapters::sqlite::mods::get_object_mod_paths(
                        pool,
                        game_id,
                        &object_id,
                        &target_rel,
                    )
                    .await?;
                for sibling_path in all_sibling_paths {
                    let path = mods_root.join(&sibling_path);
                    let Ok(relative_to_activation) = path.strip_prefix(&activation_path) else {
                        continue;
                    };
                    let remains_disabled = relative_to_activation.components().any(|component| {
                        crate::modules::workspace::domain::normalizer::is_disabled_folder(
                            &component.as_os_str().to_string_lossy(),
                        )
                    });
                    if remains_disabled || !path.exists() {
                        continue;
                    }
                    let key = path.to_string_lossy().to_ascii_lowercase();
                    if seen_disable.insert(key) {
                        disable_paths.push(path);
                    }
                }
            }
            if seen_objects.insert(object_id.clone()) {
                changed_object_ids.push(object_id);
            }
        }

        let activation_path = crate::modules::workspace::application::scanner::conflict::activation_path_for_disabled_ancestor(
            validated_target.as_ref(),
            &mods_root,
        );
        let activation_key = activation_path.to_string_lossy().to_ascii_lowercase();
        if seen_enable.insert(activation_key) {
            enable_paths.push(activation_path);
        }
        primary_target.get_or_insert(target_path);
    }

    let mut batches = Vec::new();
    let mut next_sequence = 0;
    if !disable_paths.is_empty() {
        let mut batch = crate::modules::library::application::mods::bulk::prepare_bulk_toggle(
            &disable_paths,
            false,
        );
        next_sequence = batch.resequence(next_sequence);
        batches.push(batch);
    }
    let mut enable_batch =
        crate::modules::library::application::mods::bulk::prepare_bulk_toggle(&enable_paths, true);
    enable_batch.resequence(next_sequence);
    batches.push(enable_batch);

    Ok(PreparedWorkspaceSwitch::Mod(PreparedModSwitch {
        target_path: primary_target.expect("non-empty target values produce a target"),
        changed_object_ids,
        batches,
    }))
}

async fn resolve_mod_target_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    target_value: &str,
    desired_enabled: bool,
) -> Result<(String, Vec<String>), AppError> {
    let mods_path = crate::modules::games::adapters::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let mods_root = Path::new(&mods_path);
    let target_path = Path::new(target_value);
    let absolute_target = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        mods_root.join(target_path)
    };
    let resolved_target =
        crate::modules::library::application::mods::core_ops::resolve_existing_runtime_variant(
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
        crate::modules::library::adapters::sqlite::mods::get_mod_id_and_status_by_path(
            pool,
            &relative_path,
            game_id,
        )
        .await?
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
    let result =
        crate::modules::workspace::application::scanner::conflict::enable_only_this_service(
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
        let outcome =
            crate::modules::library::application::mods::object_switch::toggle_object_root_service(
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
