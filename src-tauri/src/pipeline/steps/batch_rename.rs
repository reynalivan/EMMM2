use std::collections::{HashMap, HashSet};

use crate::shared::errors::{AppError, CollectionError};
use crate::modules::workspace::domain::workspace::WorkspacePathRewrite;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::modules::mutation::application::workspace_mutation::engine::{
    toggle_mods_mixed, RuntimeToggleBatchRequest, RuntimeToggleOperation, RuntimeToggleTarget,
};

struct ObjectTogglePlan {
    current_path: std::path::PathBuf,
    target_enabled: bool,
}

/// Batch rename mod folders and persist DB projection.
pub async fn rename(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let _guard = crate::modules::workspace::application::scanner::watcher::SuppressionGuard::new(&ctx.suppressor);
    // One pass over the game's mods, indexed by key — the enable and disable
    // lists then resolve from memory instead of re-reading the table.
    let by_key = load_targets_by_key(ctx).await?;
    let to_enable = pick_targets(&by_key, &ctx.to_enable);
    let to_disable = pick_targets(&by_key, &ctx.to_disable);
    let object_plans = load_object_plans(ctx).await?;
    let enable_count = to_enable.len();
    let disable_count = to_disable.len();
    let mut operations = Vec::with_capacity(enable_count + disable_count);
    operations.extend(to_enable.into_iter().map(|target| RuntimeToggleOperation {
        folder_path: target.folder_path,
        target_enabled: true,
    }));
    operations.extend(to_disable.into_iter().map(|target| RuntimeToggleOperation {
        folder_path: target.folder_path,
        target_enabled: false,
    }));

    let result = toggle_mods_mixed(RuntimeToggleBatchRequest {
        mods_path: ctx.mods_path.clone(),
        operations,
    })
    .await;
    let result = match result {
        Ok(result) => result,
        Err(failure) => {
            reconcile_after_mutation_failure(ctx, &failure.rollback_warnings).await;
            return Err(failure.error);
        }
    };

    ctx.mods_enabled = result.enabled_count;
    ctx.mods_disabled = result.disabled_count;
    ctx.runtime_path_rewrites.extend(result.path_rewrites);
    ctx.warnings.extend(result.warnings);
    let mut changed_paths = result.changed_paths;

    // Terminal mod state is independent from parent state. Apply every child
    // rename first, then rename parent roots so a parent move cannot invalidate
    // a child path before its terminal state is settled.
    for plan in object_plans {
        let original_path = plan.current_path.to_string_lossy().to_string();
        match crate::modules::library::application::mods::object_switch::toggle_object_root_on_disk(
            &plan.current_path,
            plan.target_enabled,
        ) {
            Ok(Some(next_path)) => {
                let next_path = next_path.to_string_lossy().to_string();
                changed_paths.extend([original_path.clone(), next_path.clone()]);
                ctx.runtime_path_rewrites.push(WorkspacePathRewrite {
                    old_path: original_path,
                    new_path: next_path,
                });
            }
            Ok(None) => changed_paths.push(original_path),
            Err(error) => {
                let error = object_toggle_error(error);
                reconcile_after_mutation_failure(ctx, &[]).await;
                return Err(error);
            }
        }
    }

    // Single-writer: the renames above changed disk only. Converge the mods
    // and object rows now — later pipeline steps read the projected runtime state.
    // Watcher events for these paths are blanket-suppressed for the whole apply.
    // The collection/hotkey entrypoint
    // retains its game-first mutation lease across this inline projection, so
    // it cannot interleave with a queued reconcile for the same game.
    if !changed_paths.is_empty() {
        let reconcile = crate::modules::reconciliation::application::disk_reconcile::reconcile::reconcile_disk_projection(
            crate::modules::reconciliation::application::disk_reconcile::reconcile::ReconcileDiskProjectionRequest {
                pool: &ctx.pool,
                game_id: &ctx.game_id,
                mods_path: &ctx.mods_path,
                safe_mode_keywords: &ctx.settings.safety.keywords,
                reason:
                    &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::InternalMutation,
                changed_paths: &changed_paths,
                force_full: false,
                watcher_events: None,
                path_hints: &[],
                progress_reporter: None,
            },
        )
        .await;
        if let Err(error) = reconcile {
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

async fn load_object_plans(ctx: &ApplyContext) -> Result<Vec<ObjectTogglePlan>, CollectionError> {
    let mut conn = ctx.pool.acquire().await?;
    let rows = crate::modules::catalog::adapters::outbound::sqlite::object::get_rows_for_reconcile(&mut conn, &ctx.game_id).await?;
    drop(conn);
    let by_id = rows
        .into_iter()
        .map(|row| (row.id.clone(), row))
        .collect::<HashMap<_, _>>();

    Ok(ctx
        .target_objects
        .iter()
        .filter_map(|target| {
            let current = by_id.get(&target.object_id)?;
            (current.status.is_enabled() != target.is_enabled).then(|| {
                let folder_path = std::path::PathBuf::from(&current.folder_path);
                ObjectTogglePlan {
                    current_path: if folder_path.is_absolute() {
                        folder_path
                    } else {
                        ctx.mods_path.join(folder_path)
                    },
                    target_enabled: target.is_enabled,
                }
            })
        })
        .collect())
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
        .map(
            |rewrite| crate::modules::workspace::application::scanner::watcher::ModWatchEvent::Renamed {
                from: rewrite.old_path.clone(),
                to: rewrite.new_path.clone(),
            },
        )
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
    crate::modules::library::application::apply_progress::set_warnings(&ctx.game_id, ctx.warnings.clone());
}

/// Every mod row for the game, reachable by both key spellings it may be
/// addressed under: its stored `folder_path_key`, and the key its path yields
/// once the `DISABLED ` prefix is normalized away.
async fn load_targets_by_key(
    ctx: &ApplyContext,
) -> Result<HashMap<String, RuntimeToggleTarget>, CollectionError> {
    let mut conn = ctx.pool.acquire().await?;
    let rows = crate::modules::library::adapters::outbound::sqlite::mods::get_rows_for_reconcile(&mut conn, &ctx.game_id).await?;
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
        .map(|segment| crate::modules::library::application::mods::core_ops::standardize_prefix(segment, true))
        .collect::<Vec<_>>()
        .join("/");
    crate::shared::path_key::folder_path_key(&clean_path, mods_path).to_lowercase()
}
