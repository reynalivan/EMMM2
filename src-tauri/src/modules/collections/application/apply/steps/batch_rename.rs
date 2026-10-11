use super::object_toggle_error;
use super::targets::{load_targets_by_key, pick_targets};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::rollback::{
    reconcile_after_mutation_failure, rollback_applied, with_rollback_error, AppliedRename,
};
use crate::modules::collections::application::apply::apply_pipeline::ApplyContext;
use crate::modules::library::application::mods::core_ops::ToggleRenamePlan;
use crate::modules::mutation::application::workspace_mutation::engine::{
    plan_runtime_toggles, RuntimeRenamePlan, RuntimeToggleBatchRequest, RuntimeToggleOperation,
};
use crate::modules::mutation::journal::PlannedStep;
use crate::modules::workspace::domain::workspace::WorkspacePathRewrite;
use crate::platform::fs::file_utils::{FilesystemIdentityProof, FilesystemNamespaceProof};
use crate::platform::fs::rename::PinnedDirectory;
use crate::platform::fs::retry::{blocking_error, RetryBudget};
use crate::shared::errors::{AppError, CollectionError};

pub(crate) struct PreparedCollectionRenames {
    mod_plans: Vec<RuntimeRenamePlan>,
    object_plans: Vec<ToggleRenamePlan>,
    namespace_proof: Arc<FilesystemNamespaceProof>,
}

#[cfg(test)]
impl PreparedCollectionRenames {
    pub(crate) fn first_mod_plan(&self) -> Option<RuntimeRenamePlan> {
        self.mod_plans.first().cloned()
    }

    pub(crate) fn first_object_plan(&self) -> Option<ToggleRenamePlan> {
        self.object_plans.first().cloned()
    }
}

pub async fn prepare(ctx: &mut ApplyContext) -> Result<Vec<PlannedStep>, CollectionError> {
    let source_root = match ctx.source_root_proof.as_ref() {
        Some(proof) => proof.clone(),
        None => FilesystemIdentityProof::capture(&ctx.mods_path)
            .map_err(|error| CollectionError::Io(error.to_string()))?,
    };
    if ctx.source_root_proof.is_none() {
        ctx.source_root_proof = Some(source_root.clone());
    }
    source_root
        .validate(&ctx.mods_path)
        .map_err(|error| CollectionError::Io(error.to_string()))?;
    let by_key = load_targets_by_key(ctx).await?;
    let to_enable = pick_targets(&by_key, &ctx.to_enable);
    let to_disable = pick_targets(&by_key, &ctx.to_disable);
    let mut operations = Vec::with_capacity(to_enable.len() + to_disable.len());
    operations.extend(to_enable.into_iter().map(|target| RuntimeToggleOperation {
        folder_path: target.folder_path,
        target_enabled: true,
    }));
    operations.extend(to_disable.into_iter().map(|target| RuntimeToggleOperation {
        folder_path: target.folder_path,
        target_enabled: false,
    }));
    source_root
        .validate(&ctx.mods_path)
        .map_err(|error| CollectionError::Io(error.to_string()))?;
    let mut mod_plans = plan_runtime_toggles(&RuntimeToggleBatchRequest {
        mods_path: ctx.mods_path.clone(),
        operations,
    })
    .map_err(|failure| failure.error)?;
    let mut object_plans = load_object_plans(ctx).await?;
    source_root
        .validate(&ctx.mods_path)
        .map_err(|error| CollectionError::Io(error.to_string()))?;
    let paths = mod_plans
        .iter()
        .flat_map(|plan| [plan.old_path().to_path_buf(), plan.new_path().to_path_buf()])
        .chain(
            object_plans
                .iter()
                .flat_map(|plan| [plan.old_path().to_path_buf(), plan.new_path().to_path_buf()]),
        )
        .collect::<Vec<_>>();
    let namespace_proof = Arc::new(
        FilesystemNamespaceProof::capture(&ctx.mods_path, &paths)
            .map_err(|error| CollectionError::Io(error.to_string()))?,
    );
    source_root
        .validate(&ctx.mods_path)
        .map_err(|error| CollectionError::Io(error.to_string()))?;
    for plan in &mut mod_plans {
        plan.set_namespace_proof(namespace_proof.clone());
    }
    for plan in &mut object_plans {
        plan.set_namespace_proof(namespace_proof.clone());
    }
    let mut steps = Vec::with_capacity(mod_plans.len() + object_plans.len());
    for plan in mod_plans
        .iter()
        .filter(|plan| plan.old_path() != plan.new_path())
    {
        steps.push(
            PlannedStep::rename(
                steps.len() as u32,
                plan.old_path().to_path_buf(),
                plan.new_path().to_path_buf(),
            )
            .with_expected_identity(Some(plan.expected_identity().to_string())),
        );
    }
    for plan in &object_plans {
        steps.push(
            PlannedStep::rename(
                steps.len() as u32,
                plan.old_path().to_path_buf(),
                plan.new_path().to_path_buf(),
            )
            .with_expected_identity(Some(plan.expected_identity().to_string())),
        );
    }
    ctx.prepared_renames = Some(PreparedCollectionRenames {
        mod_plans,
        object_plans,
        namespace_proof,
    });
    Ok(steps)
}

pub async fn rename(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    rename_with_hook(ctx, || {}).await
}

#[cfg(test)]
pub(crate) async fn rename_with_test_hook(
    ctx: &mut ApplyContext,
    before_reconcile: impl FnOnce(),
) -> Result<(), CollectionError> {
    rename_with_hook(ctx, before_reconcile).await
}

async fn rename_with_hook(
    ctx: &mut ApplyContext,
    before_reconcile: impl FnOnce(),
) -> Result<(), CollectionError> {
    let _guard = crate::modules::workspace::application::scanner::watcher::SuppressionGuard::new(
        &ctx.suppressor,
    );
    let prepared = ctx.prepared_renames.take().ok_or_else(|| {
        CollectionError::Validation("Collection renames were not prepared".to_string())
    })?;
    if let Some(proof) = ctx.source_root_proof.as_ref() {
        proof
            .validate(&ctx.mods_path)
            .map_err(|error| CollectionError::Io(error.to_string()))?;
    }
    let _root_pin = crate::platform::fs::rename::PinnedDirectory::open(&ctx.mods_path)
        .map_err(|error| CollectionError::Io(error.to_string()))?;
    if ctx
        .source_root_proof
        .as_ref()
        .is_some_and(|proof| _root_pin.identity() != proof.identity())
    {
        return Err(CollectionError::Io(
            "Collection Mods root changed while pinning it".into(),
        ));
    }
    let planned_count = prepared
        .mod_plans
        .iter()
        .filter(|plan| plan.old_path() != plan.new_path())
        .count()
        + prepared.object_plans.len();
    let mut applied = Vec::<AppliedRename>::new();
    let mut runtime_projection_participants = Vec::<(std::path::PathBuf, String)>::new();
    let mut changed_paths = Vec::with_capacity(planned_count * 2);
    let mut retry_budget = crate::platform::fs::retry::RetryBudget::default();

    for plan in &prepared.mod_plans {
        if let Err(error) = plan.apply_async(&mut retry_budget).await {
            let rollback = rollback_applied(
                ctx,
                planned_count,
                &applied,
                prepared.namespace_proof.clone(),
            )
            .await;
            reconcile_after_mutation_failure(ctx, &[]).await;
            return Err(with_rollback_error(
                crate::modules::mutation::application::workspace_mutation::engine::classify_rename_failure_with_identity(plan.old_path(), plan.expected_identity(), error),
                rollback,
            ));
        }
        runtime_projection_participants.push((
            plan.new_path().to_path_buf(),
            plan.expected_identity().to_string(),
        ));
        if plan.old_path() == plan.new_path() {
            changed_paths.extend([
                plan.requested_path().to_string_lossy().to_string(),
                plan.new_path().to_string_lossy().to_string(),
            ]);
            ctx.runtime_path_rewrites.push(WorkspacePathRewrite {
                old_path: plan.requested_path().to_string_lossy().to_string(),
                new_path: plan.new_path().to_string_lossy().to_string(),
            });
            continue;
        }
        let sequence = applied.len() as u32;
        applied.push(AppliedRename {
            sequence,
            old_path: plan.old_path().to_path_buf(),
            new_path: plan.new_path().to_path_buf(),
            expected_identity: plan.expected_identity().to_string(),
        });
        if let Some(guard) = ctx.mutation_guard.as_ref() {
            if let Err(error) = guard.mark_step_applied(sequence) {
                let rollback = rollback_applied(
                    ctx,
                    planned_count,
                    &applied,
                    prepared.namespace_proof.clone(),
                )
                .await;
                reconcile_after_mutation_failure(ctx, &[]).await;
                return Err(with_rollback_error(object_toggle_error(error), rollback));
            }
        }
        changed_paths.extend([
            plan.old_path().to_string_lossy().to_string(),
            plan.new_path().to_string_lossy().to_string(),
        ]);
        if plan.requested_path() != plan.new_path() {
            ctx.runtime_path_rewrites.push(WorkspacePathRewrite {
                old_path: plan.requested_path().to_string_lossy().to_string(),
                new_path: plan.new_path().to_string_lossy().to_string(),
            });
        }
        if plan.target_enabled() {
            ctx.mods_enabled += 1;
        } else {
            ctx.mods_disabled += 1;
        }
    }

    let object_applied_start = applied.len();
    for plan in &prepared.object_plans {
        let object_result =
            apply_object_plan(plan, prepared.namespace_proof.clone(), &mut retry_budget).await;
        if let Err(error) = object_result {
            let rollback = rollback_applied(
                ctx,
                planned_count,
                &applied,
                prepared.namespace_proof.clone(),
            )
            .await;
            reconcile_after_mutation_failure(ctx, &[]).await;
            return Err(with_rollback_error(object_toggle_error(error), rollback));
        }
        let sequence = applied.len() as u32;
        applied.push(AppliedRename {
            sequence,
            old_path: plan.old_path().to_path_buf(),
            new_path: plan.new_path().to_path_buf(),
            expected_identity: plan.expected_identity().to_string(),
        });
        if let Some(guard) = ctx.mutation_guard.as_ref() {
            if let Err(error) = guard.mark_step_applied(sequence) {
                let rollback = rollback_applied(
                    ctx,
                    planned_count,
                    &applied,
                    prepared.namespace_proof.clone(),
                )
                .await;
                reconcile_after_mutation_failure(ctx, &[]).await;
                return Err(with_rollback_error(object_toggle_error(error), rollback));
            }
        }
        let original_path = plan.old_path().to_string_lossy().to_string();
        let next_path = plan.new_path().to_string_lossy().to_string();
        changed_paths.extend([original_path.clone(), next_path.clone()]);
        ctx.runtime_path_rewrites.push(WorkspacePathRewrite {
            old_path: original_path,
            new_path: next_path,
        });
    }

    if !changed_paths.is_empty() {
        if let Some(proof) = ctx.source_root_proof.as_ref() {
            if let Err(error) = proof.validate(&ctx.mods_path) {
                let rollback = rollback_applied(
                    ctx,
                    planned_count,
                    &applied,
                    prepared.namespace_proof.clone(),
                )
                .await;
                return Err(with_rollback_error(
                    CollectionError::Io(error.to_string()),
                    rollback,
                ));
            }
        }
        let object_sources = prepared
            .object_plans
            .iter()
            .map(|plan| plan.old_path())
            .collect::<Vec<_>>();
        let unchanged_ancestry_paths =
            prepared
                .mod_plans
                .iter()
                .filter(|plan| {
                    !object_sources.iter().any(|source| {
                        plan.old_path().starts_with(source) || plan.new_path().starts_with(source)
                    })
                })
                .flat_map(|plan| [plan.old_path().to_path_buf(), plan.new_path().to_path_buf()])
                .chain(prepared.object_plans.iter().flat_map(|plan| {
                    [plan.old_path().to_path_buf(), plan.new_path().to_path_buf()]
                }))
                .collect::<Vec<_>>();
        if let Err(error) = prepared
            .namespace_proof
            .validate_paths(&unchanged_ancestry_paths)
        {
            let rollback = rollback_applied(
                ctx,
                planned_count,
                &applied,
                prepared.namespace_proof.clone(),
            )
            .await;
            return Err(with_rollback_error(
                CollectionError::Io(error.to_string()),
                rollback,
            ));
        }
        let object_rewrites = prepared
            .object_plans
            .iter()
            .map(|plan| (plan.old_path().to_path_buf(), plan.new_path().to_path_buf()))
            .collect::<Vec<_>>();
        let projection_pins = match pin_trusted_projection_scope(
            &prepared.namespace_proof,
            &unchanged_ancestry_paths,
            &runtime_projection_participants,
            &applied[object_applied_start..],
            &object_rewrites,
            &ctx.mods_path,
            before_reconcile,
        ) {
            Ok(pins) => pins,
            Err(error) => {
                let rollback = rollback_applied(
                    ctx,
                    planned_count,
                    &applied,
                    prepared.namespace_proof.clone(),
                )
                .await;
                return Err(with_rollback_error(error, rollback));
            }
        };
        let reconcile = crate::modules::reconciliation::application::disk_reconcile::reconcile::reconcile_disk_projection(
            crate::modules::reconciliation::application::disk_reconcile::reconcile::ReconcileDiskProjectionRequest {
                pool: &ctx.pool,
                game_id: &ctx.game_id,
                mods_path: &ctx.mods_path,
                safe_mode_keywords: &ctx.settings.safety.keywords,
                reason: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::InternalMutation,
                changed_paths: &changed_paths,
                force_full: false,
                watcher_events: None,
                path_hints: &[],
                // Every plan was identity-checked immediately before a
                // same-parent enable/disable rename and is journaled with that
                // identity. No unrelated root can participate in this write.
                trusted_mutation_scope: true,
                progress_reporter: None,
                precomputed_discovery: None,
            },
        )
        .await;
        drop(projection_pins);
        match reconcile {
            Ok(outcome) if outcome.status.applied() => {
                log::debug!(
                    "apply_pipeline[batch_rename]: scan_scope={:?} full_scan_count={}",
                    outcome.scan_scope,
                    usize::from(
                        outcome.scan_scope
                            == crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileScanScope::Full
                    )
                );
            }
            Ok(outcome) => {
                let rollback = rollback_applied(
                    ctx,
                    planned_count,
                    &applied,
                    prepared.namespace_proof.clone(),
                )
                .await;
                reconcile_after_mutation_failure(ctx, &[]).await;
                return Err(with_rollback_error(
                    CollectionError::Db(format!(
                        "Post-rename disk reconcile did not apply: {:?}",
                        outcome.status
                    )),
                    rollback,
                ));
            }
            Err(error) => {
                let rollback = rollback_applied(
                    ctx,
                    planned_count,
                    &applied,
                    prepared.namespace_proof.clone(),
                )
                .await;
                reconcile_after_mutation_failure(ctx, &[]).await;
                return Err(with_rollback_error(
                    CollectionError::Db(format!("Post-rename disk reconcile failed: {error}")),
                    rollback,
                ));
            }
        }
    }
    log::info!(
        "apply_pipeline[batch_rename]: {} enabled, {} disabled",
        ctx.mods_enabled,
        ctx.mods_disabled
    );
    Ok(())
}

struct TrustedProjectionPins {
    _original_parents: Vec<PinnedDirectory>,
    _current_parents: Vec<PinnedDirectory>,
    _destinations: Vec<PinnedDirectory>,
}

fn pin_trusted_projection_scope(
    original_namespace: &FilesystemNamespaceProof,
    unchanged_ancestry_paths: &[std::path::PathBuf],
    runtime_participants: &[(std::path::PathBuf, String)],
    applied_objects: &[AppliedRename],
    object_rewrites: &[(std::path::PathBuf, std::path::PathBuf)],
    mods_path: &std::path::Path,
    before_reconcile: impl FnOnce(),
) -> Result<TrustedProjectionPins, CollectionError> {
    let original_parents = original_namespace
        .pin_paths(unchanged_ancestry_paths)
        .map_err(|error| CollectionError::Io(error.to_string()))?;
    let mut seen = HashSet::new();
    let destinations = runtime_participants
        .iter()
        .map(|(path, identity)| (path.clone(), identity.clone()))
        .chain(
            applied_objects
                .iter()
                .map(|rename| (rename.new_path.clone(), rename.expected_identity.clone())),
        )
        .filter_map(|(mut path, identity)| {
            for (old, new) in object_rewrites {
                if let Ok(suffix) = path.strip_prefix(old) {
                    path = new.join(suffix);
                }
            }
            seen.insert((path.clone(), identity.clone()))
                .then_some((path, identity))
        })
        .collect::<Vec<_>>();
    let destination_paths = destinations
        .iter()
        .map(|(path, _)| path.clone())
        .collect::<Vec<_>>();
    let current_namespace = FilesystemNamespaceProof::capture(mods_path, &destination_paths)
        .map_err(|error| CollectionError::Io(error.to_string()))?;
    let current_parents = current_namespace
        .pin_paths(&destination_paths)
        .map_err(|error| CollectionError::Io(error.to_string()))?;
    let mut destination_pins = Vec::with_capacity(destinations.len());
    for (path, expected_identity) in destinations {
        let pin =
            PinnedDirectory::open(&path).map_err(|error| CollectionError::Io(error.to_string()))?;
        if pin.identity() != expected_identity {
            return Err(CollectionError::Io(format!(
                "Collection destination changed before trusted projection: {}",
                path.display()
            )));
        }
        destination_pins.push(pin);
    }
    before_reconcile();
    Ok(TrustedProjectionPins {
        _original_parents: original_parents,
        _current_parents: current_parents,
        _destinations: destination_pins,
    })
}

async fn apply_object_plan(
    plan: &ToggleRenamePlan,
    namespace: Arc<FilesystemNamespaceProof>,
    budget: &mut RetryBudget,
) -> Result<(), AppError> {
    let source = plan.old_path().to_path_buf();
    let destination = plan.new_path().to_path_buf();
    let expected = plan.expected_identity().to_string();
    let paths = [source.clone(), destination.clone()];
    let result = budget
        .run(|| {
            let namespace = namespace.clone();
            let source = source.clone();
            let destination = destination.clone();
            let expected = expected.clone();
            async move {
                tokio::task::spawn_blocking(move || {
                    rename_object_once(&namespace, &source, &destination, &expected, || {})
                })
                .await
                .map_err(blocking_error)?
            }
        })
        .await;
    result.map_err(|error| {
        if let Err(namespace_error) = namespace.validate_paths(&paths) {
            return namespace_error;
        }
        crate::platform::fs::retry::map_file_error_with_identity(
            &source,
            &expected,
            "rename object folder",
            error,
        )
    })
}

fn rename_object_once(
    namespace: &FilesystemNamespaceProof,
    source: &std::path::Path,
    destination: &std::path::Path,
    expected: &str,
    before_publication: impl FnOnce(),
) -> std::io::Result<()> {
    let parent = source
        .parent()
        .ok_or_else(|| std::io::Error::other("Object rename source has no parent"))?;
    if destination.parent() != Some(parent) {
        return Err(std::io::Error::other(
            "Object rename must remain within one parent",
        ));
    }
    let paths = [source.to_path_buf(), destination.to_path_buf()];
    let pins = namespace
        .pin_paths(&paths)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let parent = crate::shared::path_key::physical_namespace_path(parent)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let parent_pin = pins
        .iter()
        .find(|pin| pin.path() == parent.as_path())
        .ok_or_else(|| std::io::Error::other("Object rename parent is not pinned"))?;
    let source_name = source
        .file_name()
        .ok_or_else(|| std::io::Error::other("Object rename source has no name"))?;
    let destination_name = destination
        .file_name()
        .ok_or_else(|| std::io::Error::other("Object rename destination has no name"))?;
    let owned_source = crate::platform::fs::rename::PinnedDirectory::open_for_rename(
        &parent_pin.stable_path().join(source_name),
    )?;
    if owned_source.identity() != expected {
        return Err(std::io::Error::other(
            "Object rename source changed after namespace pinning",
        ));
    }
    before_publication();
    owned_source.rename_into(parent_pin, destination_name)
}

async fn load_object_plans(ctx: &ApplyContext) -> Result<Vec<ToggleRenamePlan>, CollectionError> {
    let object_ids = ctx
        .target_objects
        .iter()
        .map(|target| target.object_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let rows = crate::modules::catalog::adapters::sqlite::object::get_game_objects_by_ids(
        &ctx.pool,
        &ctx.game_id,
        &object_ids,
    )
    .await?;
    let by_id = rows
        .into_iter()
        .map(|row| (row.id.clone(), row))
        .collect::<HashMap<_, _>>();
    let mut plans = Vec::new();
    for target in &ctx.target_objects {
        let Some(current) = by_id.get(&target.object_id) else {
            continue;
        };
        if current.status.is_enabled() == target.is_enabled {
            continue;
        }
        let folder_path = std::path::PathBuf::from(&current.folder_path);
        let current_path = if folder_path.is_absolute() {
            folder_path
        } else {
            ctx.mods_path.join(folder_path)
        };
        let Some(plan) = crate::modules::library::application::mods::core_ops::plan_toggle_rename(
            &current_path,
            target.is_enabled,
        )
        .map_err(object_toggle_error)?
        else {
            continue;
        };
        plans.push(plan);
    }
    Ok(plans)
}

#[cfg(all(test, windows))]
mod tests {
    use super::rename_object_once;
    use crate::platform::fs::file_utils::{filesystem_identity, FilesystemNamespaceProof};

    #[test]
    fn owned_object_source_cannot_be_swapped_before_publication() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("Mods");
        let source = root.join("Group/Object");
        let destination = root.join("Group/DISABLED Object");
        let parked = root.join("Group/ParkedObject");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("asset.bin"), b"original").unwrap();
        let expected = filesystem_identity(&source).unwrap();
        let namespace =
            FilesystemNamespaceProof::capture(&root, &[source.clone(), destination.clone()])
                .unwrap();

        rename_object_once(&namespace, &source, &destination, &expected, || {
            std::fs::rename(&source, &parked)
                .expect_err("owned source handle must deny a competing leaf rename");
        })
        .unwrap();

        assert_eq!(
            filesystem_identity(&destination).as_deref(),
            Some(expected.as_str())
        );
        assert_eq!(
            std::fs::read(destination.join("asset.bin")).unwrap(),
            b"original"
        );
        assert!(!source.exists());
        assert!(!parked.exists());
    }
}
