use std::collections::{HashMap, HashSet};

use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::services::runtime_mutation_engine::{
    toggle_mods_mixed, RuntimeToggleBatchRequest, RuntimeToggleOperation, RuntimeToggleTarget,
};

/// Batch rename mod folders and persist DB projection.
pub async fn rename(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let _guard = crate::services::scanner::watcher::SuppressionGuard::new(&ctx.suppressor);
    // One pass over the game's mods, indexed by key — the enable and disable
    // lists then resolve from memory instead of re-reading the table.
    let by_key = load_targets_by_key(ctx).await?;
    let to_enable = pick_targets(&by_key, &ctx.to_enable);
    let to_disable = pick_targets(&by_key, &ctx.to_disable);
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
            if !failure.rollback_warnings.is_empty() {
                recover_after_incomplete_rollback(ctx, &failure.rollback_warnings).await;
            }
            return Err(failure.error);
        }
    };

    ctx.mods_enabled = result.enabled_count;
    ctx.mods_disabled = result.disabled_count;
    ctx.runtime_path_rewrites.extend(result.path_rewrites);
    ctx.warnings.extend(result.warnings);

    // Single-writer: the renames above changed disk only. Converge the mods
    // rows now — the later pipeline steps (corridor, post-apply harvest) read
    // `status` and must see the new state. Watcher events for these paths are
    // blanket-suppressed for the whole apply, and the orchestrator's per-game
    // lock below keeps this inline pass from interleaving with a queued
    // reconcile (WindowRefocused etc.) for the same game.
    if !result.changed_paths.is_empty() {
        let _reconcile_lock = match ctx.reconcile_lock.as_ref() {
            Some(lock) => Some(lock.clone().lock_owned().await),
            None => None,
        };
        crate::services::disk_reconcile::reconcile::reconcile_disk_projection(
            crate::services::disk_reconcile::reconcile::ReconcileDiskProjectionRequest {
                pool: &ctx.pool,
                game_id: &ctx.game_id,
                mods_path: &ctx.mods_path,
                safe_mode_keywords: &ctx.settings.safe_mode.keywords,
                reason:
                    &crate::services::disk_reconcile::types::DiskReconcileReason::InternalMutation,
                changed_paths: &result.changed_paths,
                force_full: false,
                watcher_events: None,
            },
        )
        .await
        .map_err(|error| {
            CollectionError::Db(format!("Post-rename disk reconcile failed: {error}"))
        })?;
    }

    log::info!(
        "apply_pipeline[batch_rename]: {} enabled, {} disabled",
        ctx.mods_enabled,
        ctx.mods_disabled
    );

    Ok(())
}

async fn recover_after_incomplete_rollback(ctx: &mut ApplyContext, warnings: &[String]) {
    ctx.warnings.extend(warnings.iter().cloned());
    let _reconcile_lock = match ctx.reconcile_lock.as_ref() {
        Some(lock) => Some(lock.clone().lock_owned().await),
        None => None,
    };
    let outcome = crate::services::disk_reconcile::reconcile::reconcile_disk_projection(
        crate::services::disk_reconcile::reconcile::ReconcileDiskProjectionRequest {
            pool: &ctx.pool,
            game_id: &ctx.game_id,
            mods_path: &ctx.mods_path,
            safe_mode_keywords: &ctx.settings.safe_mode.keywords,
            reason: &crate::services::disk_reconcile::types::DiskReconcileReason::InternalMutation,
            changed_paths: &[],
            force_full: true,
            watcher_events: None,
        },
    )
    .await;

    let recovery_message = match outcome {
        Ok(_) => "Full disk reconcile completed after incomplete rollback".to_string(),
        Err(error) => format!("Full disk reconcile failed after incomplete rollback: {error}"),
    };
    ctx.warnings.push(recovery_message);
    crate::services::apply_progress_service::set_warnings(
        &ctx.game_id,
        ctx.is_safe,
        ctx.warnings.clone(),
    );
}

/// Every mod row for the game, reachable by both key spellings it may be
/// addressed under: its stored `folder_path_key`, and the key its path yields
/// once the `DISABLED ` prefix is normalized away.
async fn load_targets_by_key(
    ctx: &ApplyContext,
) -> Result<HashMap<String, RuntimeToggleTarget>, CollectionError> {
    let mut conn = ctx.pool.acquire().await?;
    let rows = crate::repo::mod_repo::get_rows_for_reconcile(&mut conn, &ctx.game_id).await?;
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
        .map(|segment| crate::services::mods::core_ops::standardize_prefix(segment, true))
        .collect::<Vec<_>>()
        .join("/");
    crate::common::path_key::folder_path_key(&clean_path, mods_path).to_lowercase()
}
