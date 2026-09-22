//! Workspace switch orchestration.
//! Moved out of `commands::app::workspace_cmds` so the command layer stays a
//! thin State-extraction wrapper over this service.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sha2::{Digest, Sha256};

use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::modules::workspace::domain::workspace::{
    WorkspaceImpact, WorkspaceParentEnableImpact, WorkspaceParentEnableParent,
    WorkspaceParentEnableRequirement, WorkspacePathRewrite, WorkspaceRefreshScope,
    WorkspaceSwitchDuplicate, WorkspaceSwitchInput, WorkspaceSwitchResolution,
    WorkspaceSwitchResult, WorkspaceSwitchStatus, WorkspaceSwitchTargetKind,
};
use crate::shared::errors::AppError;

#[cfg(test)]
#[path = "tests/prepared_switch_tests.rs"]
mod prepared_switch_tests;

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
    final_target_path: String,
    changed_object_ids: Vec<String>,
    duplicates: Vec<WorkspaceSwitchDuplicate>,
    batches: Vec<crate::modules::library::application::mods::bulk::PreparedBulkToggle>,
    logical_rewrites: Vec<WorkspacePathRewrite>,
}

#[derive(Debug, Clone)]
pub enum PreparedWorkspaceSwitch {
    Immediate(Box<WorkspaceSwitchResult>),
    Object(crate::modules::library::application::mods::object_switch::PreparedObjectSwitch),
    Objects(Vec<crate::modules::library::application::mods::object_switch::PreparedObjectSwitch>),
    Mod(PreparedModSwitch),
}

#[derive(Debug, Clone)]
pub struct WorkspaceMutationRename {
    pub sequence: u32,
    pub old_path: PathBuf,
    pub new_path: PathBuf,
    pub identity_path: PathBuf,
    pub expected_identity: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceMutationScope {
    pub renames: Vec<WorkspaceMutationRename>,
    pub changed_paths: Vec<String>,
    pub owning_roots: Vec<String>,
    pub touched_object_ids: Vec<String>,
}

impl WorkspaceMutationScope {
    pub fn has_trusted_identities(&self) -> bool {
        !self.renames.is_empty()
            && self
                .renames
                .iter()
                .all(|rename| rename.expected_identity.is_some())
    }

    pub fn validate_identities(&self) -> Result<(), AppError> {
        for rename in &self.renames {
            let Some(expected) = rename.expected_identity.as_deref() else {
                continue;
            };
            let actual = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(
                &rename.identity_path,
            );
            if actual.as_deref() != Some(expected) {
                return Err(AppError::Io(format!(
                    "Folder changed while preparing the switch: {}",
                    rename.identity_path.display()
                )));
            }
        }
        Ok(())
    }
}

struct ResolvedModTarget {
    path: String,
    mod_id: Option<String>,
    changed_object_ids: Vec<String>,
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
            Self::Objects(prepared) => {
                let mut sequence = 0_u32;
                prepared
                    .iter()
                    .filter_map(|object| {
                        let step = object.journal_step(sequence);
                        if step.is_some() {
                            sequence += 1;
                        }
                        step
                    })
                    .collect()
            }
            Self::Mod(prepared) => prepared
                .batches
                .iter()
                .flat_map(|batch| batch.planned_steps())
                .collect(),
        }
    }

    pub fn immediate_result(&self) -> Option<WorkspaceSwitchResult> {
        match self {
            Self::Immediate(result) => Some(result.as_ref().clone()),
            Self::Object(_) | Self::Objects(_) | Self::Mod(_) => None,
        }
    }

    pub fn mutation_scope(&self, mods_root: &Path) -> Result<WorkspaceMutationScope, AppError> {
        let steps = self.journal_steps();
        let mut prior_rewrites = Vec::<(PathBuf, PathBuf)>::new();
        let mut renames = Vec::with_capacity(steps.len());
        let mut changed_paths = Vec::with_capacity(steps.len() * 2);
        let mut owning_roots = BTreeSet::new();

        for (sequence, old_path, new_path) in steps {
            let physical_source = reverse_rebase_path(old_path.clone(), &prior_rewrites);
            let expected_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(
                &physical_source,
            )
            .ok_or_else(|| {
                AppError::Io(format!(
                    "Could not establish filesystem identity for {}",
                    physical_source.display()
                ))
            })?;
            for path in [&old_path, &new_path] {
                changed_paths.push(path.to_string_lossy().into_owned());
                if let Ok(relative) = path.strip_prefix(mods_root) {
                    if let Some(root) = relative.components().next() {
                        owning_roots.insert(root.as_os_str().to_string_lossy().into_owned());
                    }
                }
            }
            prior_rewrites.push((old_path.clone(), new_path.clone()));
            renames.push(WorkspaceMutationRename {
                sequence,
                old_path,
                new_path,
                identity_path: physical_source,
                expected_identity: Some(expected_identity),
            });
        }

        let touched_object_ids = match self {
            Self::Immediate(result) => result.changed_object_ids.clone(),
            Self::Object(prepared) => vec![prepared.object_id().to_string()],
            Self::Objects(prepared) => prepared
                .iter()
                .map(|object| object.object_id().to_string())
                .collect(),
            Self::Mod(prepared) => prepared.changed_object_ids.clone(),
        };
        Ok(WorkspaceMutationScope {
            renames,
            changed_paths,
            owning_roots: owning_roots.into_iter().collect(),
            touched_object_ids,
        })
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
            Self::Immediate(result) => Ok(result.as_ref().clone()),
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
                    parent_enable_requirement: None,
                    impact: build_switch_impact(
                        Some(&outcome.original_path),
                        Some(&outcome.next_path),
                        &changed_folder_paths,
                        std::slice::from_ref(&outcome.object_id),
                    ),
                    sync_warning: None,
                    runtime_sync_generation: None,
                })
            }
            Self::Objects(prepared_objects) => {
                let mut applied = Vec::new();
                let mut outcomes = Vec::new();
                for prepared in prepared_objects {
                    match prepared.execute(watcher) {
                        Ok(outcome) => {
                            applied.push(prepared);
                            outcomes.push(outcome);
                        }
                        Err(error) => {
                            for completed in applied.into_iter().rev() {
                                if let Err(rollback_error) = completed.rollback(watcher) {
                                    return Err(PreparedWorkspaceExecutionError::Compensation(
                                        rollback_error,
                                    ));
                                }
                            }
                            return Err(PreparedWorkspaceExecutionError::Apply(error));
                        }
                    }
                }
                let mut rewrites = Vec::new();
                let mut changed_folder_paths = Vec::new();
                let mut seen_paths = HashSet::new();
                let mut changed_object_ids = Vec::with_capacity(outcomes.len());
                for outcome in outcomes {
                    if outcome.original_path != outcome.next_path {
                        changed_object_ids.push(outcome.object_id);
                        for path in [&outcome.original_path, &outcome.next_path] {
                            if seen_paths.insert(path.clone()) {
                                changed_folder_paths.push(path.clone());
                            }
                        }
                        rewrites.push(WorkspacePathRewrite {
                            old_path: outcome.original_path,
                            new_path: outcome.next_path,
                        });
                    }
                }
                let status = if rewrites.is_empty() {
                    WorkspaceSwitchStatus::Noop
                } else {
                    WorkspaceSwitchStatus::Applied
                };
                let mut impact =
                    build_switch_impact(None, None, &changed_folder_paths, &changed_object_ids);
                impact.rewrites = rewrites;
                Ok(WorkspaceSwitchResult {
                    status,
                    primary_path: None,
                    changed_folder_paths,
                    changed_object_ids,
                    duplicates: Vec::new(),
                    parent_enable_requirement: None,
                    impact,
                    sync_warning: None,
                    runtime_sync_generation: None,
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
                        "workspace-switch",
                        false,
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

                rewrites.extend(prepared.logical_rewrites.iter().cloned());
                let primary_path = Some(prepared.final_target_path.clone());
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
                    duplicates: prepared.duplicates.clone(),
                    parent_enable_requirement: None,
                    impact,
                    sync_warning: None,
                    runtime_sync_generation: None,
                })
            }
        }
    }

    pub fn rollback(&self, watcher: &WatcherState) -> Result<(), AppError> {
        match self {
            Self::Immediate(_) => Ok(()),
            Self::Object(prepared) => prepared.rollback(watcher),
            Self::Objects(prepared) => {
                for object in prepared.iter().rev() {
                    object.rollback(watcher)?;
                }
                Ok(())
            }
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

pub async fn prepare_object_batch_switch(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_ids: &[String],
    desired_enabled: bool,
) -> Result<PreparedWorkspaceSwitch, AppError> {
    let mut seen_ids = HashSet::new();
    let mut unique_ids = Vec::new();
    for object_id in object_ids {
        if !seen_ids.insert(object_id.clone()) {
            continue;
        }
        unique_ids.push(object_id.clone());
    }
    if unique_ids.is_empty() {
        return Err(AppError::Validation(
            "Object bulk switch requires at least one object".to_string(),
        ));
    }
    const MAX_OBJECT_BATCH_SIZE: usize = 10_000;
    if unique_ids.len() > MAX_OBJECT_BATCH_SIZE {
        return Err(AppError::Validation(format!(
            "Object bulk switch supports at most {MAX_OBJECT_BATCH_SIZE} objects"
        )));
    }
    let prepared =
        crate::modules::library::application::mods::object_switch::prepare_object_root_switches(
            pool,
            game_id,
            &unique_ids,
            desired_enabled,
        )
        .await?;
    let mut mutation_paths = prepared
        .iter()
        .filter_map(|object| object.journal_step(0).map(|(_, old_path, _)| old_path))
        .collect::<Vec<_>>();
    mutation_paths.sort();
    for (index, path) in mutation_paths.iter().enumerate() {
        if mutation_paths
            .iter()
            .skip(index + 1)
            .any(|candidate| candidate.starts_with(path) || path.starts_with(candidate))
        {
            return Err(AppError::Validation(
                "Object bulk switch cannot include overlapping object roots".to_string(),
            ));
        }
    }
    Ok(PreparedWorkspaceSwitch::Objects(prepared))
}

fn reverse_rebase_path(mut path: PathBuf, rewrites: &[(PathBuf, PathBuf)]) -> PathBuf {
    for (old_path, new_path) in rewrites.iter().rev() {
        if let Ok(suffix) = path.strip_prefix(new_path) {
            path = old_path.join(suffix);
        }
    }
    path
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

    let resolved_target = resolve_mod_target_path(
        pool,
        &input.game_id,
        &input.target.value,
        input.desired_enabled,
    )
    .await?;
    let validated_target =
        crate::platform::fs::guard::validate_path(config, &input.game_id, &resolved_target.path)?;
    let configured_mods_root = config
        .mods_root_for(&input.game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    let mods_root = crate::platform::fs::guard::validate_mods_root(
        config,
        &input.game_id,
        &configured_mods_root.to_string_lossy(),
    )?;
    let disabled_parents = if input.desired_enabled {
        crate::modules::workspace::application::scanner::conflict::disabled_ancestor_paths(
            validated_target.as_ref(),
            &mods_root,
        )
    } else {
        Vec::new()
    };
    if !disabled_parents.is_empty() {
        let requirement = build_parent_enable_requirement(
            pool,
            &input.game_id,
            &mods_root,
            validated_target.as_ref(),
            &disabled_parents,
        )
        .await?;
        let confirmation_matches = input.enable_disabled_ancestors
            && input.parent_enable_confirmation.as_deref()
                == Some(requirement.confirmation_token.as_str());
        if !confirmation_matches {
            return Ok(PreparedWorkspaceSwitch::Immediate(Box::new(
                WorkspaceSwitchResult {
                    status: WorkspaceSwitchStatus::RequiresParentEnable,
                    primary_path: None,
                    changed_folder_paths: Vec::new(),
                    changed_object_ids: resolved_target.changed_object_ids.clone(),
                    duplicates: Vec::new(),
                    parent_enable_requirement: Some(requirement),
                    impact: build_switch_impact(
                        None,
                        None,
                        &[],
                        &resolved_target.changed_object_ids,
                    ),
                    sync_warning: None,
                    runtime_sync_generation: None,
                },
            )));
        }
    }
    let mut disable_paths = Vec::new();
    let mut duplicates = Vec::new();
    if input.desired_enabled && input.resolution != WorkspaceSwitchResolution::ForceEnable {
        let target_rel =
            crate::shared::path_key::relative_to_root(validated_target.original(), &mods_root);
        if matches!(input.resolution, WorkspaceSwitchResolution::EnableOnlyThis) {
            for object_id in &resolved_target.changed_object_ids {
                let siblings = if input.enable_disabled_ancestors {
                    crate::modules::library::adapters::sqlite::mods::get_object_mod_paths(
                        pool,
                        &input.game_id,
                        object_id,
                        resolved_target.mod_id.as_deref(),
                    )
                    .await?
                } else {
                    crate::modules::library::adapters::sqlite::mods::get_enabled_siblings_paths(
                        pool,
                        object_id,
                        &input.game_id,
                        resolved_target.mod_id.as_deref(),
                    )
                    .await?
                };
                disable_paths.extend(siblings.into_iter().map(|path| mods_root.join(path)));
            }
        } else {
            let detected_duplicates = crate::modules::workspace::application::scanner::conflict::get_duplicates_for_mod_service(
                pool, &target_rel, &input.game_id,
            ).await?;
            if !detected_duplicates.is_empty() {
                duplicates = map_duplicates(detected_duplicates);
            }
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
    let (parent_batches, mut target_batch, final_target_path, logical_rewrites) =
        prepare_target_activation_batches(
            validated_target.as_ref(),
            input.desired_enabled,
            &disabled_parents,
        )?;
    for mut batch in parent_batches {
        next_sequence = batch.resequence(next_sequence);
        batches.push(batch);
    }
    target_batch.resequence(next_sequence);
    batches.push(target_batch);
    Ok(PreparedWorkspaceSwitch::Mod(PreparedModSwitch {
        target_path: validated_target.to_string_lossy().into_owned(),
        final_target_path: final_target_path.to_string_lossy().into_owned(),
        changed_object_ids: resolved_target.changed_object_ids,
        duplicates,
        batches,
        logical_rewrites,
    }))
}

fn rebase_path(mut path: PathBuf, rewrites: &[(PathBuf, PathBuf)]) -> PathBuf {
    for (old_path, new_path) in rewrites {
        if let Ok(suffix) = path.strip_prefix(old_path) {
            path = new_path.join(suffix);
        }
    }
    path
}

fn has_disabled_component(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(component, std::path::Component::Normal(name) if crate::modules::workspace::domain::normalizer::is_disabled_folder(&name.to_string_lossy()))
    })
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| {
            crate::modules::workspace::domain::normalizer::normalize_display_name(
                &name.to_string_lossy(),
            )
            .into_owned()
        })
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn proposed_parent_activation_rewrites(
    target_path: &Path,
    disabled_parents: &[PathBuf],
) -> Vec<(PathBuf, PathBuf)> {
    let mut rewrites = Vec::with_capacity(disabled_parents.len() + 1);
    for parent_path in disabled_parents {
        let current_path = rebase_path(parent_path.clone(), &rewrites);
        let next_path = current_path.with_file_name(
            current_path
                .file_name()
                .map(|name| {
                    crate::modules::library::application::mods::core_ops::standardize_prefix(
                        &name.to_string_lossy(),
                        true,
                    )
                })
                .unwrap_or_default(),
        );
        rewrites.push((current_path, next_path));
    }

    let current_target = rebase_path(target_path.to_path_buf(), &rewrites);
    let next_target = current_target.with_file_name(
        current_target
            .file_name()
            .map(|name| {
                crate::modules::library::application::mods::core_ops::standardize_prefix(
                    &name.to_string_lossy(),
                    true,
                )
            })
            .unwrap_or_default(),
    );
    if current_target != next_target {
        rewrites.push((current_target, next_target));
    }
    rewrites
}

async fn build_parent_enable_requirement(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_root: &Path,
    target_path: &Path,
    disabled_parents: &[PathBuf],
) -> Result<WorkspaceParentEnableRequirement, AppError> {
    let rewrites = proposed_parent_activation_rewrites(target_path, disabled_parents);
    let Some(outer_parent) = disabled_parents.first() else {
        return Err(AppError::Internal(
            "Parent-enable requirement must contain a disabled parent".to_string(),
        ));
    };
    let root_string = mods_root.to_string_lossy();
    let parent_key = crate::shared::path_key::folder_path_key(
        &outer_parent.to_string_lossy(),
        Some(&root_string),
    );
    let subtree = crate::modules::library::adapters::sqlite::mods::get_mods_for_folder_subtree(
        pool,
        game_id,
        &parent_key,
    )
    .await
    .map_err(|error| AppError::Db(format!("Could not load parent-enable impact: {error}")))?;

    let mut will_activate = Vec::new();
    let mut stay_disabled = Vec::new();
    for entry in subtree {
        let current_path = mods_root.join(&entry.folder_path);
        let next_path = rebase_path(current_path.clone(), &rewrites);
        if current_path == next_path {
            continue;
        }
        let impact = WorkspaceParentEnableImpact {
            path: next_path.to_string_lossy().into_owned(),
            name: entry.actual_name,
        };
        if has_disabled_component(&next_path) {
            stay_disabled.push(impact);
        } else {
            will_activate.push(impact);
        }
    }

    let parents = disabled_parents
        .iter()
        .map(|path| WorkspaceParentEnableParent {
            path: path.to_string_lossy().into_owned(),
            name: display_name(path),
        })
        .collect::<Vec<_>>();
    let confirmation_token = parent_enable_confirmation_token(
        game_id,
        mods_root,
        target_path,
        disabled_parents,
        &will_activate,
        &stay_disabled,
    )?;
    Ok(WorkspaceParentEnableRequirement {
        confirmation_token,
        requested_target: WorkspaceParentEnableImpact {
            path: target_path.to_string_lossy().into_owned(),
            name: display_name(target_path),
        },
        parents,
        will_activate,
        stay_disabled,
    })
}

fn parent_enable_confirmation_token(
    game_id: &str,
    mods_root: &Path,
    target_path: &Path,
    disabled_parents: &[PathBuf],
    will_activate: &[WorkspaceParentEnableImpact],
    stay_disabled: &[WorkspaceParentEnableImpact],
) -> Result<String, AppError> {
    let outer_parent = disabled_parents.first().ok_or_else(|| {
        AppError::Internal("Parent-enable confirmation requires a disabled parent".to_string())
    })?;
    let mut pending = vec![outer_parent.clone()];
    let mut directories = Vec::new();
    while let Some(directory) = pending.pop() {
        let relative = directory.strip_prefix(mods_root).map_err(|_| {
            AppError::Validation(format!(
                "Parent-enable directory is outside the Mods root: {}",
                directory.display()
            ))
        })?;
        let identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&directory)
            .ok_or_else(|| {
                AppError::Io(format!(
                    "Could not establish filesystem identity for {}",
                    directory.display()
                ))
            })?;
        directories.push((
            crate::shared::path_key::canonical_path_key_for_path(relative),
            identity,
        ));
        let entries = std::fs::read_dir(&directory).map_err(|error| {
            AppError::Io(format!(
                "Could not inspect parent-enable subtree '{}': {error}",
                directory.display()
            ))
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| AppError::Io(error.to_string()))?;
            if entry
                .file_type()
                .map_err(|error| AppError::Io(error.to_string()))?
                .is_dir()
            {
                pending.push(entry.path());
            }
        }
    }
    directories.sort();

    let mut hasher = Sha256::new();
    for value in [game_id, &target_path.to_string_lossy()] {
        hash_confirmation_component(&mut hasher, value.as_bytes());
    }
    for parent in disabled_parents {
        hash_confirmation_component(&mut hasher, parent.to_string_lossy().as_bytes());
    }
    for (path, identity) in directories {
        hash_confirmation_component(&mut hasher, path.as_bytes());
        hash_confirmation_component(&mut hasher, identity.as_bytes());
    }
    for impact in will_activate {
        hash_confirmation_component(&mut hasher, b"active");
        hash_confirmation_component(&mut hasher, impact.path.as_bytes());
    }
    for impact in stay_disabled {
        hash_confirmation_component(&mut hasher, b"disabled");
        hash_confirmation_component(&mut hasher, impact.path.as_bytes());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_confirmation_component(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value);
}

fn prepare_target_activation_batches(
    target_path: &Path,
    desired_enabled: bool,
    disabled_parents: &[PathBuf],
) -> Result<
    (
        Vec<crate::modules::library::application::mods::bulk::PreparedBulkToggle>,
        crate::modules::library::application::mods::bulk::PreparedBulkToggle,
        PathBuf,
        Vec<WorkspacePathRewrite>,
    ),
    AppError,
> {
    let mut parent_batches = Vec::with_capacity(disabled_parents.len());
    let mut parent_rewrites = Vec::with_capacity(disabled_parents.len());
    for parent_path in disabled_parents {
        let mut parent_batch =
            crate::modules::library::application::mods::bulk::prepare_bulk_toggle(
                std::slice::from_ref(parent_path),
                true,
            );
        parent_batch.rebase_paths(&parent_rewrites);
        let Some((old_path, new_path)) = parent_batch.single_planned_step()? else {
            return Err(AppError::Internal(format!(
                "Disabled parent unexpectedly had no activation step: {}",
                parent_path.display()
            )));
        };
        parent_rewrites.push((old_path, new_path));
        parent_batches.push(parent_batch);
    }

    let target_path_buf = target_path.to_path_buf();
    let mut target_batch = crate::modules::library::application::mods::bulk::prepare_bulk_toggle(
        std::slice::from_ref(&target_path_buf),
        desired_enabled,
    );
    target_batch.rebase_paths(&parent_rewrites);
    let final_target_path = target_batch
        .single_planned_step()?
        .map(|(_, new_path)| new_path)
        .unwrap_or_else(|| rebase_path(target_path.to_path_buf(), &parent_rewrites));
    let logical_rewrites = if target_path != final_target_path {
        vec![WorkspacePathRewrite {
            old_path: target_path.to_string_lossy().into_owned(),
            new_path: final_target_path.to_string_lossy().into_owned(),
        }]
    } else {
        Vec::new()
    };

    Ok((
        parent_batches,
        target_batch,
        final_target_path,
        logical_rewrites,
    ))
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
        let resolved_target = resolve_mod_target_path(pool, game_id, target_value, true).await?;
        let validated_target =
            crate::platform::fs::guard::validate_path(config, game_id, &resolved_target.path)?;
        for object_id in resolved_target.changed_object_ids {
            if exclusive_object_ids.contains(&object_id) {
                let sibling_paths =
                    crate::modules::library::adapters::sqlite::mods::get_enabled_siblings_paths(
                        pool,
                        &object_id,
                        game_id,
                        resolved_target.mod_id.as_deref(),
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
                        resolved_target.mod_id.as_deref(),
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
        primary_target.get_or_insert(resolved_target.path);
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

    let primary_target = primary_target.expect("non-empty target values produce a target");
    Ok(PreparedWorkspaceSwitch::Mod(PreparedModSwitch {
        final_target_path: primary_target.clone(),
        target_path: primary_target,
        changed_object_ids,
        duplicates: Vec::new(),
        batches,
        logical_rewrites: Vec::new(),
    }))
}

async fn resolve_mod_target_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    target_value: &str,
    desired_enabled: bool,
) -> Result<ResolvedModTarget, AppError> {
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

    let mut mod_id = None;
    let mut changed_object_ids = Vec::new();
    if let Some((id, Some(object_id), _)) =
        crate::modules::library::adapters::sqlite::mods::get_mod_id_and_status_by_path(
            pool,
            &relative_path,
            game_id,
        )
        .await?
    {
        mod_id = Some(id);
        changed_object_ids.push(object_id);
    }

    Ok(ResolvedModTarget {
        path: resolved_target.to_string_lossy().to_string(),
        mod_id,
        changed_object_ids,
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

#[cfg(test)]
#[path = "tests/switch_tests.rs"]
mod tests;
