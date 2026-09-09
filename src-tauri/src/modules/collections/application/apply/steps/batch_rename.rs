use std::collections::{HashMap, HashSet};

use crate::modules::collections::application::apply::apply_pipeline::ApplyContext;
use crate::modules::mutation::application::workspace_mutation::engine::{
    plan_runtime_toggles, RuntimeRenamePlan, RuntimeToggleBatchRequest, RuntimeToggleOperation,
    RuntimeToggleTarget,
};
use crate::modules::mutation::journal::PlannedStep;
use crate::modules::workspace::domain::workspace::WorkspacePathRewrite;
use crate::shared::errors::{AppError, CollectionError};

struct ObjectTogglePlan {
    current_path: std::path::PathBuf,
    target_path: std::path::PathBuf,
}

pub(crate) struct PreparedCollectionRenames {
    mod_plans: Vec<RuntimeRenamePlan>,
    object_plans: Vec<ObjectTogglePlan>,
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
        steps.push(PlannedStep::rename(
            steps.len() as u32,
            plan.old_path().to_path_buf(),
            plan.new_path().to_path_buf(),
        ));
    }
    for plan in &object_plans {
        steps.push(PlannedStep::rename(
            steps.len() as u32,
            plan.current_path.clone(),
            plan.target_path.clone(),
        ));
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
    let mut applied = Vec::<(u32, std::path::PathBuf, std::path::PathBuf)>::new();
    let mut changed_paths = Vec::with_capacity(planned_count * 2);

    for plan in &prepared.mod_plans {
        if let Err(error) = plan.apply() {
            rollback_applied(ctx, planned_count, &applied);
            reconcile_after_mutation_failure(ctx, &[]).await;
            return Err(CollectionError::Io(error.to_string()));
        }
        let sequence = applied.len() as u32;
        applied.push((
            sequence,
            plan.old_path().to_path_buf(),
            plan.new_path().to_path_buf(),
        ));
        if let Some(guard) = ctx.mutation_guard.as_ref() {
            if let Err(error) = guard.mark_step_applied(sequence) {
                rollback_applied(ctx, planned_count, &applied);
                return Err(object_toggle_error(error));
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
        if let Err(error) = std::fs::rename(&plan.current_path, &plan.target_path) {
            rollback_applied(ctx, planned_count, &applied);
            reconcile_after_mutation_failure(ctx, &[]).await;
            return Err(object_toggle_error(
                crate::modules::library::application::mods::core_ops::map_toggle_error(
                    &plan.current_path,
                    "object folder",
                    error,
                ),
            ));
        }
        let sequence = applied.len() as u32;
        applied.push((
            sequence,
            plan.current_path.clone(),
            plan.target_path.clone(),
        ));
        if let Some(guard) = ctx.mutation_guard.as_ref() {
            if let Err(error) = guard.mark_step_applied(sequence) {
                rollback_applied(ctx, planned_count, &applied);
                return Err(object_toggle_error(error));
            }
        }
        let original_path = plan.current_path.to_string_lossy().to_string();
        let next_path = plan.target_path.to_string_lossy().to_string();
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
                progress_reporter: None,
            },
        )
        .await;
        if let Err(error) = reconcile {
            rollback_applied(ctx, planned_count, &applied);
            reconcile_after_mutation_failure(ctx, &[]).await;
            return Err(CollectionError::Db(format!(
                "Post-rename disk reconcile failed: {error}"
            )));
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
    applied: &[(u32, std::path::PathBuf, std::path::PathBuf)],
) {
    if let Some(guard) = ctx.mutation_guard.as_ref() {
        if guard.begin_rollback().is_err() {
            return;
        }
    }
    for (_, old_path, new_path) in applied.iter().rev() {
        if std::fs::rename(new_path, old_path).is_err() {
            return;
        }
    }
    let Some(guard) = ctx.mutation_guard.as_ref() else {
        return;
    };
    for sequence in 0..planned_count as u32 {
        if guard.mark_step_rolled_back(sequence).is_err() {
            return;
        }
    }
    if let Some(guard) = ctx.mutation_guard.take() {
        if guard.finish_rollback().is_err() {
            return;
        }
    }
    ctx.mutation_started = false;
    ctx.runtime_path_rewrites.clear();
}

async fn load_object_plans(ctx: &ApplyContext) -> Result<Vec<ObjectTogglePlan>, CollectionError> {
    let mut conn = ctx.pool.acquire().await?;
    let rows = crate::modules::catalog::adapters::sqlite::object::get_rows_for_reconcile(
        &mut conn,
        &ctx.game_id,
    )
    .await?;
    drop(conn);
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
        plans.push(ObjectTogglePlan {
            current_path,
            target_path: plan.new_path().to_path_buf(),
        });
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
            progress_reporter: None,
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
    let mut conn = ctx.pool.acquire().await?;
    let rows = crate::modules::library::adapters::sqlite::mods::get_rows_for_reconcile(
        &mut conn,
        &ctx.game_id,
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
