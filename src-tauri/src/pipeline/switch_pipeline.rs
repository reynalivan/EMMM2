use std::path::PathBuf;
use std::sync::Arc;

use sqlx::SqlitePool;

use crate::domain::collection::CollectionMod;
use crate::domain::corridor::SwitchResult;
use crate::domain::errors::CorridorError;
use crate::repo::corridor_repo;
use crate::services::app::post_apply::PostApplyContext;
use crate::services::config::AppSettings;
use crate::services::corridor_constants::DISABLED_REASON_SYSTEM;
use crate::services::runtime_mutation_engine::{
    toggle_mods_mixed, RuntimeToggleBatchRequest, RuntimeToggleOperation, RuntimeToggleTarget,
};
use crate::services::scanner::watcher::{WatcherState, WatcherSuppressor};

// ---------------------------------------------------------------------------
// SwitchPipeline — Composable corridor switch operation
// ---------------------------------------------------------------------------

/// Context passed through all pipeline steps during a corridor switch.
pub struct SwitchContext {
    // Inputs
    pub pool: SqlitePool,
    pub game_id: String,
    pub target_safe: bool,
    pub mods_path: PathBuf,
    pub suppressor: Arc<WatcherSuppressor>,
    pub settings: AppSettings,

    // State during execution
    pub mods_disabled: usize,
    pub mods_restored: usize,
    pub new_signature: String,
    pub restored_collection_id: Option<String>,
    pub warnings: Vec<String>,
}

impl SwitchContext {
    pub fn new(
        pool: SqlitePool,
        game_id: String,
        target_safe: bool,
        mods_path: PathBuf,
        suppressor: Arc<WatcherSuppressor>,
        settings: AppSettings,
    ) -> Self {
        Self {
            pool,
            game_id,
            target_safe,
            mods_path,
            suppressor,
            settings,
            mods_disabled: 0,
            mods_restored: 0,
            new_signature: String::new(),
            restored_collection_id: None,
            warnings: Vec::new(),
        }
    }
}

/// Execute the full corridor switch pipeline.
/// `watcher_state` is passed by reference since WatcherState is not Clone.
/// This is an intentional physical-rename path. Disk Reconcile must stay
/// read/projection-only and never perform corridor switch renames.
pub async fn execute(
    ctx: &mut SwitchContext,
    watcher_state: &WatcherState,
) -> Result<SwitchResult, CorridorError> {
    let task_id = uuid::Uuid::new_v4().to_string();
    let target_str = ctx.target_safe.to_string();
    crate::repo::task_repo::create_task(
        &ctx.pool,
        &task_id,
        &ctx.game_id,
        "switch_corridor",
        Some(&target_str),
    )
    .await
    .map_err(|e| CorridorError::Db(e.to_string()))?;

    let result = execute_inner(ctx, watcher_state).await;

    let status = if result.is_ok() {
        crate::domain::task::TaskStatus::Completed
    } else {
        crate::domain::task::TaskStatus::Failed
    };
    let _ = crate::repo::task_repo::update_status(&ctx.pool, &task_id, status).await;

    result
}

async fn execute_inner(
    ctx: &mut SwitchContext,
    _watcher_state: &WatcherState,
) -> Result<SwitchResult, CorridorError> {
    let _guard = crate::services::scanner::watcher::SuppressionGuard::new(&ctx.suppressor);
    corridor_repo::ensure_exists(&ctx.pool, &ctx.game_id, ctx.target_safe).await?;
    corridor_repo::ensure_exists(&ctx.pool, &ctx.game_id, !ctx.target_safe).await?;
    let leaving_snapshot = crate::services::corridor_service::get_corridor_state(
        &ctx.pool,
        &ctx.game_id,
        !ctx.target_safe,
    )
    .await?;
    if leaving_snapshot.is_dirty {
        crate::services::collection_service::persist_corridor_runtime_snapshot(
            &ctx.pool,
            &ctx.game_id,
            !ctx.target_safe,
        )
        .await
        .map_err(|error| CorridorError::Db(error.to_string()))?;
    }
    let leaving_state = corridor_repo::get(&ctx.pool, &ctx.game_id, !ctx.target_safe).await?;
    corridor_repo::update_pointers(
        &ctx.pool,
        &ctx.game_id,
        !ctx.target_safe,
        leaving_state
            .as_ref()
            .and_then(|state| state.active_collection_id.as_deref()),
        None,
    )
    .await?;

    let plan = build_switch_mutation_plan(ctx).await?;
    ctx.restored_collection_id = plan.restored_collection_id;
    ctx.mods_disabled = plan.leaving_disable_count;
    ctx.warnings.extend(plan.warnings);
    let result = toggle_mods_mixed(
        &ctx.pool,
        RuntimeToggleBatchRequest {
            game_id: ctx.game_id.clone(),
            mods_path: ctx.mods_path.clone(),
            operations: plan.operations,
        },
    )
    .await
    .map_err(|error| CorridorError::Db(error.to_string()))?;
    ctx.mods_restored = result.enabled_count;
    ctx.warnings.extend(
        result
            .warnings
            .into_iter()
            .map(|warning| format!("switch-stage: {warning}")),
    );

    // Step 1: Update corridor state + recompute signature
    update_state(ctx).await?;

    // Step 2: Run post-apply tasks (KeyViewer, Conflicts, Status)
    let post_ctx = PostApplyContext {
        pool: ctx.pool.clone(),
        game_id: ctx.game_id.clone(),
        is_safe: ctx.target_safe,
        mods_path: ctx.mods_path.clone(),
        suppressor: ctx.suppressor.clone(),
        settings: ctx.settings.clone(),
        status_fields: None,
    };
    let _ = crate::services::app::post_apply::run_post_apply_tasks(post_ctx).await;

    Ok(SwitchResult {
        success: true,
        active_safe: ctx.target_safe,
        mods_disabled: ctx.mods_disabled,
        mods_restored: ctx.mods_restored,
        new_signature: ctx.new_signature.clone(),
        warnings: ctx.warnings.clone(),
        restored_collection_id: ctx.restored_collection_id.clone(),
    })
}

// ---------------------------------------------------------------------------
// Individual switch steps
// ---------------------------------------------------------------------------

struct SwitchMutationPlan {
    operations: Vec<RuntimeToggleOperation>,
    leaving_disable_count: usize,
    restored_collection_id: Option<String>,
    warnings: Vec<String>,
}

async fn build_switch_mutation_plan(
    ctx: &SwitchContext,
) -> Result<SwitchMutationPlan, CorridorError> {
    let mut operations = Vec::new();
    let mut warnings = Vec::new();
    let leaving_operations = collect_leaving_disable_operations(ctx).await?;
    let leaving_disable_count = leaving_operations.len();
    operations.extend(leaving_operations);

    let resolved_target = crate::services::corridor_service::resolve_restore_collection(
        &ctx.pool,
        &ctx.game_id,
        ctx.target_safe,
    )
    .await?;

    let Some((collection, kind_label)) = resolved_target else {
        log::info!(
            "switch_pipeline: target corridor (safe={}) has no saved state, falling back to SYSTEM reason",
            ctx.target_safe
        );
        operations.extend(collect_system_restore_operations(ctx).await?);
        return Ok(SwitchMutationPlan {
            operations,
            leaving_disable_count,
            restored_collection_id: None,
            warnings,
        });
    };

    let collection_id = collection.id.clone();

    log::info!(
        "switch_pipeline: restoring target corridor (safe={}) via {} collection '{}'",
        ctx.target_safe,
        kind_label,
        collection_id
    );

    let collection_operations =
        collect_collection_restore_operations(ctx, &collection, &mut warnings).await?;
    operations.extend(collection_operations);

    Ok(SwitchMutationPlan {
        operations,
        leaving_disable_count,
        restored_collection_id: Some(collection_id),
        warnings,
    })
}

async fn collect_leaving_disable_operations(
    ctx: &SwitchContext,
) -> Result<Vec<RuntimeToggleOperation>, CorridorError> {
    let leaving_safe = !ctx.target_safe;
    let is_safe_i32 = if leaving_safe { 1i32 } else { 0i32 };
    let rows: Vec<(String, String)> = sqlx::query_as(
        r#"SELECT id, folder_path FROM mods
        WHERE game_id = ? AND is_safe = ? AND status = 1
        AND object_id IS NOT NULL"#,
    )
    .bind(&ctx.game_id)
    .bind(is_safe_i32)
    .fetch_all(&ctx.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, folder_path)| RuntimeToggleOperation {
            id,
            folder_path,
            target_enabled: false,
            disabled_reason: Some(DISABLED_REASON_SYSTEM.to_string()),
        })
        .collect())
}

async fn collect_collection_restore_operations(
    ctx: &SwitchContext,
    collection: &crate::domain::collection::Collection,
    warnings: &mut Vec<String>,
) -> Result<Vec<RuntimeToggleOperation>, CorridorError> {
    let mods_path = ctx.mods_path.to_string_lossy().to_string();
    let snapshot = crate::services::collection_service::load_projected_collection_state(
        &ctx.pool,
        collection,
        Some(mods_path.as_str()),
    )
    .await
    .map_err(CorridorError::from)?;
    let target_mods = crate::services::projected_state_service::mods_from_projected_state(
        &collection.id,
        &snapshot,
    );
    let valid_target_mods = filter_existing_target_mods(ctx, &target_mods, warnings);
    let target_keys = valid_target_mods
        .iter()
        .filter_map(|member| {
            member
                .mod_path_key
                .clone()
                .or_else(|| Some(member.mod_path.clone()))
        })
        .collect::<std::collections::HashSet<_>>();
    let (current_mods, _) = crate::services::collection_service::load_live_corridor_state(
        &ctx.pool,
        &ctx.game_id,
        ctx.target_safe,
    )
    .await
    .map_err(CorridorError::from)?;
    let current_keys = current_mods
        .iter()
        .filter_map(|member| {
            member
                .mod_path_key
                .clone()
                .or_else(|| Some(member.mod_path.clone()))
        })
        .collect::<std::collections::HashSet<_>>();
    let to_enable = target_keys
        .difference(&current_keys)
        .cloned()
        .collect::<Vec<_>>();
    let to_disable = current_keys
        .difference(&target_keys)
        .cloned()
        .collect::<Vec<_>>();
    let enable_targets = load_targets_for_keys(ctx, ctx.target_safe, &to_enable).await?;
    let disable_targets = load_targets_for_keys(ctx, ctx.target_safe, &to_disable).await?;
    let mut operations = Vec::with_capacity(enable_targets.len() + disable_targets.len());
    operations.extend(
        enable_targets
            .into_iter()
            .map(|target| RuntimeToggleOperation {
                id: target.id,
                folder_path: target.folder_path,
                target_enabled: true,
                disabled_reason: None,
            }),
    );
    operations.extend(
        disable_targets
            .into_iter()
            .map(|target| RuntimeToggleOperation {
                id: target.id,
                folder_path: target.folder_path,
                target_enabled: false,
                disabled_reason: Some("COLLECTION".to_string()),
            }),
    );
    Ok(operations)
}

fn filter_existing_target_mods(
    ctx: &SwitchContext,
    target_mods: &[CollectionMod],
    warnings: &mut Vec<String>,
) -> Vec<CollectionMod> {
    target_mods
        .iter()
        .filter_map(|member| {
            if target_mod_exists(&ctx.mods_path, &member.mod_path) {
                return Some(member.clone());
            }

            warnings.push(format!(
                "restore-stage: Missing mod on disk: {}",
                member.mod_path
            ));
            None
        })
        .collect()
}

fn target_mod_exists(mods_path: &std::path::Path, mod_path: &str) -> bool {
    let candidate = mods_path.join(mod_path);
    if candidate.exists() {
        return true;
    }

    let file_name = std::path::Path::new(mod_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let parent = std::path::Path::new(mod_path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""));
    let disabled_name = format!("{}{}", crate::DISABLED_PREFIX, file_name);
    mods_path.join(parent).join(disabled_name).exists()
}

async fn collect_system_restore_operations(
    ctx: &SwitchContext,
) -> Result<Vec<RuntimeToggleOperation>, CorridorError> {
    let is_safe_i32 = if ctx.target_safe { 1i32 } else { 0i32 };
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, folder_path FROM mods WHERE game_id = ? AND is_safe = ? AND status = 0 AND disabled_reason = ?",
    )
    .bind(&ctx.game_id)
    .bind(is_safe_i32)
    .bind(DISABLED_REASON_SYSTEM)
    .fetch_all(&ctx.pool)
    .await
    .map_err(|error| CorridorError::Db(error.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|(id, folder_path)| RuntimeToggleOperation {
            id,
            folder_path,
            target_enabled: true,
            disabled_reason: None,
        })
        .collect())
}

async fn load_targets_for_keys(
    ctx: &SwitchContext,
    is_safe: bool,
    keys: &[String],
) -> Result<Vec<RuntimeToggleTarget>, CorridorError> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let rows = sqlx::query(
        r#"
        SELECT id, folder_path, folder_path_key
        FROM mods
        WHERE game_id = ? AND is_safe = ?
        "#,
    )
    .bind(&ctx.game_id)
    .bind(if is_safe { 1i32 } else { 0i32 })
    .fetch_all(&ctx.pool)
    .await?;
    let desired = keys
        .iter()
        .map(|key| key.to_lowercase())
        .collect::<std::collections::HashSet<_>>();
    let mods_path = ctx.mods_path.to_string_lossy().to_string();
    let mut targets = Vec::new();

    for row in rows {
        use sqlx::Row;

        let id: String = row.get("id");
        let folder_path: String = row.get("folder_path");
        let folder_path_key: Option<String> = row.try_get("folder_path_key").ok();
        let normalized_key = normalized_enabled_key(&folder_path, Some(&mods_path));
        let matches = folder_path_key
            .as_deref()
            .is_some_and(|key| desired.contains(&key.to_lowercase()))
            || desired.contains(&normalized_key);

        if matches {
            targets.push(RuntimeToggleTarget { id, folder_path });
        }
    }

    Ok(targets)
}

fn normalized_enabled_key(path: &str, mods_path: Option<&str>) -> String {
    let clean_path = path
        .split(['/', '\\'])
        .map(|segment| crate::services::mods::core_ops::standardize_prefix(segment, true))
        .collect::<Vec<_>>()
        .join("/");
    crate::services::path_key::folder_path_key(&clean_path, mods_path).to_lowercase()
}

/// Step 3: Recompute signature from newly-enabled mods and record switch timestamp.
async fn update_state(ctx: &mut SwitchContext) -> Result<(), CorridorError> {
    // Recompute signature for the target corridor
    ctx.new_signature = crate::services::corridor_service::recompute_signature(
        &ctx.pool,
        &ctx.game_id,
        ctx.target_safe,
    )
    .await?;

    // Record the switch timestamp
    corridor_repo::record_switch(&ctx.pool, &ctx.game_id, ctx.target_safe).await?;

    log::info!(
        "switch_pipeline: corridor switch complete for game '{}' → safe={}, sig='{}'",
        ctx.game_id,
        ctx.target_safe,
        &ctx.new_signature[..8.min(ctx.new_signature.len())]
    );

    Ok(())
}
