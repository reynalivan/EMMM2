use std::collections::HashSet;
use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::domain::collection::{ApplyResult, CollectionMod, CollectionObject};
use crate::domain::errors::CollectionError;
use crate::services::app::post_apply::PostApplyContext;
use crate::services::config::AppSettings;

// ---------------------------------------------------------------------------
// ApplyPipeline — Composable collection apply operation
// ---------------------------------------------------------------------------

/// Context passed through all pipeline steps during a collection apply.
pub struct ApplyContext {
    // Inputs
    pub pool: SqlitePool,
    pub game_id: String,
    pub collection_id: String,
    pub is_safe: bool,
    pub mods_path: PathBuf,
    pub suppressor: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub ignore_missing: bool,
    pub settings: AppSettings,

    // Resolved during pipeline execution
    pub target_mods: Vec<CollectionMod>,
    pub target_objects: Vec<CollectionObject>,
    pub currently_enabled_path_keys: HashSet<String>,
    pub to_enable: Vec<String>,  // path_keys to enable
    pub to_disable: Vec<String>, // path_keys to disable
    pub undo_snapshot_id: Option<String>,
    pub new_signature: String,

    // Stats
    pub mods_enabled: usize,
    pub mods_disabled: usize,
    pub objects_toggled: usize,
}

impl ApplyContext {
    pub fn new(
        pool: SqlitePool,
        game_id: String,
        collection_id: String,
        is_safe: bool,
        mods_path: PathBuf,
        suppressor: std::sync::Arc<std::sync::atomic::AtomicBool>,
        ignore_missing: bool,
        settings: AppSettings,
    ) -> Self {
        Self {
            pool,
            game_id,
            collection_id,
            is_safe,
            mods_path,
            suppressor,
            ignore_missing,
            settings,
            target_mods: Vec::new(),
            target_objects: Vec::new(),
            currently_enabled_path_keys: HashSet::new(),
            to_enable: Vec::new(),
            to_disable: Vec::new(),
            undo_snapshot_id: None,
            new_signature: String::new(),
            mods_enabled: 0,
            mods_disabled: 0,
            objects_toggled: 0,
        }
    }
}

/// Execute the full apply pipeline.
///
/// Each step is a standalone function that operates on `ApplyContext`.
/// Steps run sequentially — each step can read/write the context.
pub async fn execute(ctx: &mut ApplyContext) -> Result<ApplyResult, CollectionError> {
    let task_id = uuid::Uuid::new_v4().to_string();
    crate::repo::task_repo::create_task(
        &ctx.pool,
        &task_id,
        &ctx.game_id,
        "apply_collection",
        Some(&ctx.collection_id),
    )
    .await
    .map_err(|e| CollectionError::Db(e.to_string()))?;

    let result = execute_inner(ctx).await;

    let status = if result.is_ok() {
        crate::domain::task::TaskStatus::Completed
    } else {
        crate::domain::task::TaskStatus::Failed
    };
    let _ = crate::repo::task_repo::update_status(&ctx.pool, &task_id, status).await;

    result
}

async fn execute_inner(ctx: &mut ApplyContext) -> Result<ApplyResult, CollectionError> {
    // Step 1: Validate corridor match
    super::steps::validate_corridor::validate(ctx).await?;

    // Step 2: Resolve target members from the collection
    super::steps::resolve_target::resolve(ctx).await?;

    // Step 3: Pre-apply disk validation (checks physical paths)
    super::steps::validate_paths::validate(ctx).await?;

    // Step 4: Resolve currently-enabled mod state
    super::steps::resolve_current_state::resolve(ctx).await?;

    // Step 5: Compute the diff (what to enable, what to disable)
    super::steps::compute_diff::compute(ctx).await?;

    // Step 5: Create undo snapshot
    super::steps::snapshot_state::snapshot(ctx).await?;

    // Step 6: Batch rename on filesystem
    super::steps::batch_rename::rename(ctx).await?;

    // Step 7: Batch update database
    super::steps::batch_db_update::update(ctx).await?;

    // Step 8: Update corridor pointers
    super::steps::update_corridor::update(ctx).await?;

    // Step 9: Run post-apply tasks (KeyViewer, Signature, Conflicts, Status)
    let post_ctx = PostApplyContext {
        pool: ctx.pool.clone(),
        game_id: ctx.game_id.clone(),
        is_safe: ctx.is_safe,
        mods_path: ctx.mods_path.clone(),
        suppressor: ctx.suppressor.clone(),
        settings: ctx.settings.clone(),
        status_fields: None,
    };
    let _ = crate::services::app::post_apply::run_post_apply_tasks(post_ctx).await;

    Ok(ApplyResult {
        success: true,
        mods_enabled: ctx.mods_enabled,
        mods_disabled: ctx.mods_disabled,
        objects_toggled: ctx.objects_toggled,
        undo_collection_id: ctx.undo_snapshot_id.clone(),
        new_signature: ctx.new_signature.clone(),
    })
}
