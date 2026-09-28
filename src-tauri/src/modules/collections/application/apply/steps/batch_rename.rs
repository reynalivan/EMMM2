use std::collections::{HashMap, HashSet};

use crate::modules::collections::application::apply::apply_pipeline::ApplyContext;
use crate::modules::library::application::mods::core_ops::ToggleRenamePlan;
use crate::modules::mutation::application::workspace_mutation::engine::{
    plan_runtime_toggles, RuntimeRenamePlan, RuntimeToggleBatchRequest, RuntimeToggleOperation,
    RuntimeToggleTarget,
};
use crate::modules::mutation::journal::PlannedStep;
use crate::modules::workspace::domain::workspace::WorkspacePathRewrite;
use crate::platform::fs::rename::rename_no_replace;
use crate::shared::errors::{AppError, CollectionError};

pub(crate) struct PreparedCollectionRenames {
    mod_plans: Vec<RuntimeRenamePlan>,
    object_plans: Vec<ToggleRenamePlan>,
}

struct AppliedRename {
    sequence: u32,
    old_path: std::path::PathBuf,
    new_path: std::path::PathBuf,
    expected_identity: String,
}

pub async fn prepare(ctx: &mut ApplyContext) -> Result<Vec<PlannedStep>, CollectionError> {
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
    let mod_plans = plan_runtime_toggles(&RuntimeToggleBatchRequest {
        mods_path: ctx.mods_path.clone(),
        operations,
    })
    .map_err(|failure| failure.error)?;
    let object_plans = load_object_plans(ctx).await?;
    let mut steps = Vec::with_capacity(mod_plans.len() + object_plans.len());
    for plan in &mod_plans {
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
    });
    Ok(steps)
}

pub async fn rename(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let _guard = crate::modules::workspace::application::scanner::watcher::SuppressionGuard::new(
        &ctx.suppressor,
    );
    let prepared = ctx.prepared_renames.take().ok_or_else(|| {
        CollectionError::Validation("Collection renames were not prepared".to_string())
    })?;
    let planned_count = prepared.mod_plans.len() + prepared.object_plans.len();
    let mut applied = Vec::<AppliedRename>::new();
    let mut changed_paths = Vec::with_capacity(planned_count * 2);

    for plan in &prepared.mod_plans {
        if let Err(error) = plan.apply() {
            let rollback = rollback_applied(ctx, planned_count, &applied);
            reconcile_after_mutation_failure(ctx, &[]).await;
            return Err(with_rollback_error(
                CollectionError::Io(error.to_string()),
                rollback,
            ));
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
                let rollback = rollback_applied(ctx, planned_count, &applied);
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

    for plan in prepared.object_plans {
        if let Err(error) = plan.apply("object folder") {
            let rollback = rollback_applied(ctx, planned_count, &applied);
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
                let rollback = rollback_applied(ctx, planned_count, &applied);
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
                let rollback = rollback_applied(ctx, planned_count, &applied);
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
                let rollback = rollback_applied(ctx, planned_count, &applied);
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

fn rollback_applied(
    ctx: &mut ApplyContext,
    planned_count: usize,
    applied: &[AppliedRename],
) -> Result<(), CollectionError> {
    if let Some(guard) = ctx.mutation_guard.as_ref() {
        guard.begin_rollback().map_err(object_toggle_error)?;
    }
    rollback_renamed_paths(applied)?;
    let Some(guard) = ctx.mutation_guard.as_ref() else {
        ctx.mutation_started = false;
        ctx.runtime_path_rewrites.clear();
        return Ok(());
    };
    for sequence in 0..planned_count as u32 {
        guard
            .mark_step_rolled_back(sequence)
            .map_err(object_toggle_error)?;
    }
    if let Some(guard) = ctx.mutation_guard.take() {
        guard.finish_rollback().map_err(object_toggle_error)?;
    }
    ctx.mutation_started = false;
    ctx.runtime_path_rewrites.clear();
    Ok(())
}

fn rollback_renamed_paths(applied: &[AppliedRename]) -> Result<(), CollectionError> {
    for rename in applied.iter().rev() {
        let actual = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&rename.new_path);
        if actual.as_deref() != Some(rename.expected_identity.as_str()) {
            return Err(CollectionError::Io(format!(
                "Rollback folder changed after rename (step {}): {}",
                rename.sequence,
                rename.new_path.display()
            )));
        }
        match std::fs::symlink_metadata(&rename.old_path) {
            Ok(_) => {
                return Err(CollectionError::Io(format!(
                    "Rollback destination is occupied (step {}): {}",
                    rename.sequence,
                    rename.old_path.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(CollectionError::Io(format!(
                    "Could not inspect rollback destination (step {}): {}: {error}",
                    rename.sequence,
                    rename.old_path.display()
                )));
            }
        }
        rename_no_replace(&rename.new_path, &rename.old_path).map_err(|error| {
            CollectionError::Io(format!(
                "Rollback rename failed (step {}): {} to {}: {error}",
                rename.sequence,
                rename.new_path.display(),
                rename.old_path.display()
            ))
        })?;
    }
    Ok(())
}

fn with_rollback_error(
    original: CollectionError,
    rollback: Result<(), CollectionError>,
) -> CollectionError {
    match rollback {
        Ok(()) => original,
        Err(error) => CollectionError::Io(format!("{original}; rollback failed: {error}")),
    }
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

fn object_toggle_error(error: AppError) -> CollectionError {
    match error {
        AppError::FileInUse { path, processes } => CollectionError::FileInUse { path, processes },
        AppError::PathBusy { path } => CollectionError::PathBusy { path },
        other => CollectionError::Io(other.to_string()),
    }
}

async fn reconcile_after_mutation_failure(ctx: &mut ApplyContext, warnings: &[String]) {
    ctx.warnings.extend(warnings.iter().cloned());
    let rename_events = ctx
        .runtime_path_rewrites
        .iter()
        .map(|rewrite| {
            crate::modules::workspace::application::scanner::watcher::ModWatchEvent::Renamed {
                from: rewrite.old_path.clone(),
                to: rewrite.new_path.clone(),
            }
        })
        .collect::<Vec<_>>();
    let changed_paths = ctx
        .runtime_path_rewrites
        .iter()
        .flat_map(|rewrite| [rewrite.old_path.clone(), rewrite.new_path.clone()])
        .collect::<Vec<_>>();
    let outcome = crate::modules::reconciliation::application::disk_reconcile::reconcile::reconcile_disk_projection(
        crate::modules::reconciliation::application::disk_reconcile::reconcile::ReconcileDiskProjectionRequest {
            pool: &ctx.pool,
            game_id: &ctx.game_id,
            mods_path: &ctx.mods_path,
            safe_mode_keywords: &ctx.settings.safety.keywords,
            reason: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::InternalMutation,
            changed_paths: &changed_paths,
            force_full: true,
            watcher_events: (!rename_events.is_empty()).then_some(rename_events.as_slice()),
            path_hints: &[],
            trusted_mutation_scope: false,
            progress_reporter: None,
            precomputed_discovery: None,
        },
    )
    .await;
    let recovery_message = match outcome {
        Ok(_) => "Full disk reconcile completed after failed mutation".to_string(),
        Err(error) => format!("Full disk reconcile failed after failed mutation: {error}"),
    };
    ctx.warnings.push(recovery_message);
    crate::modules::library::application::apply_progress::set_warnings(
        &ctx.game_id,
        ctx.warnings.clone(),
    );
}

async fn load_targets_by_key(
    ctx: &ApplyContext,
) -> Result<HashMap<String, RuntimeToggleTarget>, CollectionError> {
    let root_keys = ctx
        .to_enable
        .iter()
        .chain(&ctx.to_disable)
        .map(|key| key.to_lowercase())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut conn = ctx.pool.acquire().await?;
    let rows = crate::modules::library::adapters::sqlite::mods::get_rows_for_reconcile_scope(
        &mut conn,
        &ctx.game_id,
        &root_keys,
        &[],
    )
    .await?;
    drop(conn);
    let mods_path = ctx.mods_path.to_string_lossy().to_string();
    let mut by_key = HashMap::with_capacity(rows.len() * 2);
    for row in rows {
        let target = RuntimeToggleTarget {
            id: row.id,
            folder_path: row.folder_path.clone(),
        };
        by_key.insert(
            normalized_enabled_key(&row.folder_path, Some(&mods_path)),
            target.clone(),
        );
        by_key.insert(row.folder_path_key.to_lowercase(), target);
    }
    Ok(by_key)
}

fn pick_targets(
    by_key: &HashMap<String, RuntimeToggleTarget>,
    keys: &[String],
) -> Vec<RuntimeToggleTarget> {
    let mut seen = HashSet::with_capacity(keys.len());
    keys.iter()
        .filter_map(|key| by_key.get(&key.to_lowercase()))
        .filter(|target| seen.insert(target.id.clone()))
        .cloned()
        .collect()
}

fn normalized_enabled_key(path: &str, mods_path: Option<&str>) -> String {
    let clean_path = path
        .split(['/', '\\'])
        .map(|segment| {
            crate::modules::library::application::mods::core_ops::standardize_prefix(segment, true)
        })
        .collect::<Vec<_>>()
        .join("/");
    crate::shared::path_key::folder_path_key(&clean_path, mods_path).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{rollback_renamed_paths, AppliedRename};
    use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity;

    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn atomic_rollback_rename_preserves_an_occupied_directory() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let destination_identity = filesystem_identity(&destination).unwrap();

        super::rename_no_replace(&source, &destination)
            .expect_err("destination must not be replaced");

        assert!(source.is_dir());
        assert_eq!(
            filesystem_identity(&destination),
            Some(destination_identity)
        );
    }

    #[test]
    fn rollback_rejects_replacement_renamed_folder() {
        let temp = tempfile::tempdir().unwrap();
        let old_path = temp.path().join("DISABLED Blue");
        let new_path = temp.path().join("Blue");
        let parked = temp.path().join("parked");
        std::fs::create_dir(&old_path).unwrap();
        let expected_identity = filesystem_identity(&old_path).unwrap();
        std::fs::rename(&old_path, &new_path).unwrap();
        std::fs::rename(&new_path, &parked).unwrap();
        std::fs::create_dir(&new_path).unwrap();

        let error = rollback_renamed_paths(&[AppliedRename {
            sequence: 0,
            old_path: old_path.clone(),
            new_path: new_path.clone(),
            expected_identity,
        }])
        .expect_err("replacement must not be moved");
        assert!(error.to_string().contains("changed"));
        assert!(!old_path.exists());
        assert!(new_path.exists());
        assert!(parked.exists());
    }

    #[test]
    fn rollback_rejects_occupied_original_path() {
        let temp = tempfile::tempdir().unwrap();
        let old_path = temp.path().join("DISABLED Blue");
        let new_path = temp.path().join("Blue");
        std::fs::create_dir(&old_path).unwrap();
        let expected_identity = filesystem_identity(&old_path).unwrap();
        std::fs::rename(&old_path, &new_path).unwrap();
        std::fs::create_dir(&old_path).unwrap();

        let error = rollback_renamed_paths(&[AppliedRename {
            sequence: 0,
            old_path: old_path.clone(),
            new_path: new_path.clone(),
            expected_identity,
        }])
        .expect_err("occupied original must not be replaced");
        assert!(error.to_string().contains("occupied"));
        assert!(old_path.exists());
        assert!(new_path.exists());
    }

    #[test]
    fn rollback_restores_matching_folder() {
        let temp = tempfile::tempdir().unwrap();
        let old_path = temp.path().join("DISABLED Blue");
        let new_path = temp.path().join("Blue");
        std::fs::create_dir(&old_path).unwrap();
        let expected_identity = filesystem_identity(&old_path).unwrap();
        std::fs::rename(&old_path, &new_path).unwrap();

        rollback_renamed_paths(&[AppliedRename {
            sequence: 0,
            old_path: old_path.clone(),
            new_path: new_path.clone(),
            expected_identity: expected_identity.clone(),
        }])
        .unwrap();
        assert_eq!(filesystem_identity(&old_path), Some(expected_identity));
        assert!(!new_path.exists());
    }
}
