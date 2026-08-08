use std::collections::HashSet;
use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::domain::collection::{ApplyResult, Collection, CollectionMod, CollectionObject};
use crate::domain::errors::CollectionError;
use crate::domain::workspace::WorkspacePathRewrite;
use crate::services::app::post_apply::PostApplyContext;
use crate::services::collection_service::ApplyCollectionRequest;
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

    // Resolved during pipeline execution
    pub collection: Option<Collection>,
    pub target_mods: Vec<CollectionMod>,
    pub target_objects: Vec<CollectionObject>,
    pub currently_enabled_path_keys: HashSet<String>,
    pub to_enable: Vec<String>,  // path_keys to enable
    pub to_disable: Vec<String>, // path_keys to disable
    pub new_signature: String,
    pub warnings: Vec<String>,
    pub final_state_name: Option<String>,
    pub skipped_missing_paths: Vec<String>,
    pub final_state_is_dirty: bool,
    pub runtime_path_rewrites: Vec<WorkspacePathRewrite>,

    // Stats
    pub mods_enabled: usize,
    pub mods_disabled: usize,
    pub objects_toggled: usize,
}

/// Human-readable corridor label used in messages and apply results.
pub fn corridor_label(is_safe: bool) -> &'static str {
    if is_safe {
        "SAFE"
    } else {
        "UNSAFE"
    }
}

impl ApplyContext {
    /// The collection row loaded once by the `validate_corridor` step.
    pub fn collection(&self) -> Result<&Collection, CollectionError> {
        self.collection
            .as_ref()
            .ok_or_else(|| CollectionError::NotFound {
                id: self.collection_id.clone(),
            })
    }

    /// Seed a context from the caller's request. Takes the borrowed request
    /// type directly — an owned intermediate struct would just re-declare the
    /// same eight fields a third time.
    pub fn new(request: ApplyCollectionRequest<'_>) -> Self {
        Self {
            pool: request.pool.clone(),
            game_id: request.game_id.to_string(),
            collection_id: request.collection_id.to_string(),
            is_safe: request.is_safe,
            mods_path: request.mods_path,
            suppressor: request.suppressor,
            ignore_missing: request.ignore_missing,
            settings: request.settings,
            collection: None,
            target_mods: Vec::new(),
            target_objects: Vec::new(),
            currently_enabled_path_keys: HashSet::new(),
            to_enable: Vec::new(),
            to_disable: Vec::new(),
            new_signature: String::new(),
            warnings: Vec::new(),
            final_state_name: None,
            skipped_missing_paths: Vec::new(),
            final_state_is_dirty: false,
            runtime_path_rewrites: Vec::new(),
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
/// This is an intentional physical-rename path. Disk Reconcile must not perform
/// these collection apply renames on passive startup or watcher refresh.
pub async fn execute(ctx: &mut ApplyContext) -> Result<ApplyResult, CollectionError> {
    crate::services::apply_progress_service::start(&ctx.game_id, ctx.is_safe);
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
    if result.is_err() {
        crate::services::apply_progress_service::finish(
            &ctx.game_id,
            ctx.is_safe,
            ctx.final_state_name.clone(),
            Some(corridor_label(ctx.is_safe).to_string()),
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
    if !ctx.mods_path.exists() || !ctx.mods_path.is_dir() {
        return Err(CollectionError::Corridor(
            crate::domain::errors::CorridorError::NoModsPath {
                game_id: ctx.game_id.clone(),
            },
        ));
    }

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

    // Step 7: Verify database projection. The runtime mutation engine already wrote
    // the filesystem and the DB projection in one operation, so there is nothing to
    // do here beyond reporting progress.
    crate::services::apply_progress_service::update(
        &ctx.game_id,
        ctx.is_safe,
        "verifying",
        ctx.mods_enabled + ctx.mods_disabled,
        ctx.to_enable.len() + ctx.to_disable.len(),
        None,
    );

    // Step 8: Update corridor pointers
    super::steps::update_corridor::update(ctx).await?;

    // Step 9: Run post-apply tasks (KeyViewer, Signature, Conflicts, Status)
    let post_ctx = PostApplyContext {
        pool: ctx.pool.clone(),
        game_id: ctx.game_id.clone(),
        is_safe: ctx.is_safe,
        mods_path: ctx.mods_path.clone(),
        hotkeys: ctx.settings.hotkeys.clone(),
        // The pipeline already settled the corridor in step 8; passing it here
        // saves post-apply a second full live-state derivation.
        status_fields: (!ctx.final_state_is_dirty).then(|| {
            crate::services::keyviewer::generator::StatusFields {
                safe_mode: ctx.is_safe,
                preset_name: ctx.final_state_name.clone(),
                ..Default::default()
            }
        }),
    };
    if let Err(error) = crate::services::app::post_apply::run_post_apply_tasks(post_ctx).await {
        // Best-effort artifacts, but a silent failure here leaves the overlay
        // and the SQLite projection stale with no signal at all.
        log::warn!("apply_pipeline[post_apply]: {error}");
    }

    let result = ApplyResult {
        success: true,
        mods_enabled: ctx.mods_enabled,
        mods_disabled: ctx.mods_disabled,
        objects_toggled: ctx.objects_toggled,
        new_signature: ctx.new_signature.clone(),
        warnings: ctx.warnings.clone(),
        final_state_name: ctx.final_state_name.clone(),
        final_mode: Some(corridor_label(ctx.is_safe).to_string()),
        partial_apply: !ctx.skipped_missing_paths.is_empty(),
        skipped_missing_paths: ctx.skipped_missing_paths.clone(),
        final_state_is_dirty: ctx.final_state_is_dirty,
        runtime_path_rewrites: ctx.runtime_path_rewrites.clone(),
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
