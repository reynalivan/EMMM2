use std::collections::HashSet;
use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::domain::collection::{ApplyResult, CollectionMod, CollectionObject};
use crate::domain::errors::CollectionError;
use crate::services::app::post_apply::PostApplyContext;
use crate::services::config::AppSettings;
use crate::services::scanner::watcher::WatcherSuppressor;

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
    pub suppressor: std::sync::Arc<WatcherSuppressor>,
    pub ignore_missing: bool,
    pub settings: AppSettings,
    pub track_task: bool,

    // Resolved during pipeline execution
    pub target_mods: Vec<CollectionMod>,
    pub target_objects: Vec<CollectionObject>,
    pub currently_enabled_path_keys: HashSet<String>,
    pub to_enable: Vec<String>,  // path_keys to enable
    pub to_disable: Vec<String>, // path_keys to disable
    pub new_signature: String,
    pub warnings: Vec<String>,
    pub final_state_name: Option<String>,

    // Stats
    pub mods_enabled: usize,
    pub mods_disabled: usize,
    pub objects_toggled: usize,
}

pub struct ApplyContextInput {
    pub pool: SqlitePool,
    pub game_id: String,
    pub collection_id: String,
    pub is_safe: bool,
    pub mods_path: PathBuf,
    pub suppressor: std::sync::Arc<WatcherSuppressor>,
    pub ignore_missing: bool,
    pub settings: AppSettings,
}

impl ApplyContext {
    pub fn new(input: ApplyContextInput) -> Self {
        Self {
            pool: input.pool,
            game_id: input.game_id,
            collection_id: input.collection_id,
            is_safe: input.is_safe,
            mods_path: input.mods_path,
            suppressor: input.suppressor,
            ignore_missing: input.ignore_missing,
            settings: input.settings,
            track_task: true,
            target_mods: Vec::new(),
            target_objects: Vec::new(),
            currently_enabled_path_keys: HashSet::new(),
            to_enable: Vec::new(),
            to_disable: Vec::new(),
            new_signature: String::new(),
            warnings: Vec::new(),
            final_state_name: None,
            mods_enabled: 0,
            mods_disabled: 0,
            objects_toggled: 0,
        }
    }

    pub fn without_task(mut self) -> Self {
        self.track_task = false;
        self
    }
}

/// Execute the full apply pipeline.
///
/// Each step is a standalone function that operates on `ApplyContext`.
/// Steps run sequentially — each step can read/write the context.
/// This is an intentional physical-rename path. Disk Reconcile must not perform
/// these collection apply renames on passive startup or watcher refresh.
pub async fn execute(ctx: &mut ApplyContext) -> Result<ApplyResult, CollectionError> {
    crate::services::apply_progress_service::start(&ctx.game_id, ctx.is_safe);
    let task_id = if ctx.track_task {
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
        Some(task_id)
    } else {
        None
    };

    let result = execute_inner(ctx).await;

    if let Some(task_id) = task_id.as_deref() {
        let status = if result.is_ok() {
            crate::domain::task::TaskStatus::Completed
        } else {
            crate::domain::task::TaskStatus::Failed
        };
        let _ = crate::repo::task_repo::update_status(&ctx.pool, task_id, status).await;
    }
    if result.is_err() {
        crate::services::apply_progress_service::finish(
            &ctx.game_id,
            ctx.is_safe,
            ctx.final_state_name.clone(),
            Some(if ctx.is_safe {
                "SAFE".to_string()
            } else {
                "UNSAFE".to_string()
            }),
            ctx.warnings.clone(),
            false,
        );
    }

    result
}

async fn execute_inner(ctx: &mut ApplyContext) -> Result<ApplyResult, CollectionError> {
    crate::services::apply_progress_service::update(
        &ctx.game_id,
        ctx.is_safe,
        "preparing",
        0,
        0,
        None,
    );
    // Step 1: Validate corridor match
    super::steps::validate_corridor::validate(ctx).await?;

    // Step 2: Resolve target members from the collection
    crate::services::apply_progress_service::update(
        &ctx.game_id,
        ctx.is_safe,
        "diffing",
        0,
        0,
        None,
    );
    super::steps::resolve_target::resolve(ctx).await?;

    // Step 3: Pre-apply disk validation (checks physical paths)
    super::steps::validate_paths::validate(ctx).await?;
    crate::services::apply_progress_service::set_warnings(
        &ctx.game_id,
        ctx.is_safe,
        ctx.warnings.clone(),
    );

    // Step 4: Resolve currently-enabled mod state
    super::steps::resolve_current_state::resolve(ctx).await?;

    // Step 5: Compute the diff (what to enable, what to disable)
    super::steps::compute_diff::compute(ctx).await?;

    // Step 6: Batch rename on filesystem
    crate::services::apply_progress_service::update(
        &ctx.game_id,
        ctx.is_safe,
        "renaming",
        0,
        ctx.to_enable.len() + ctx.to_disable.len(),
        None,
    );
    super::steps::batch_rename::rename(ctx).await?;

    // Step 7: Batch update database
    crate::services::apply_progress_service::update(
        &ctx.game_id,
        ctx.is_safe,
        "verifying",
        ctx.mods_enabled + ctx.mods_disabled,
        ctx.to_enable.len() + ctx.to_disable.len(),
        None,
    );
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

    let result = ApplyResult {
        success: true,
        mods_enabled: ctx.mods_enabled,
        mods_disabled: ctx.mods_disabled,
        objects_toggled: ctx.objects_toggled,
        undo_collection_id: None,
        new_signature: ctx.new_signature.clone(),
        warnings: ctx.warnings.clone(),
        final_state_name: ctx.final_state_name.clone(),
        final_mode: Some(if ctx.is_safe {
            "SAFE".to_string()
        } else {
            "UNSAFE".to_string()
        }),
    };
    crate::services::apply_progress_service::finish(
        &ctx.game_id,
        ctx.is_safe,
        result.final_state_name.clone(),
        result.final_mode.clone(),
        result.warnings.clone(),
        true,
    );

    Ok(result)
}
