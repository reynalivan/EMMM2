use std::collections::HashSet;
use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::common::path_key::folder_path_key;
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
    pub pool: SqlitePool,
    pub game_id: String,
    pub collection_id: String,
    pub is_safe: bool,
    pub mods_path: PathBuf,
    pub suppressor: std::sync::Arc<WatcherSuppressor>,
    pub ignore_missing: bool,
    pub settings: AppSettings,
    /// See `ApplyCollectionRequest::reconcile_lock`.
    pub reconcile_lock: Option<std::sync::Arc<tokio::sync::Mutex<()>>>,

    pub collection: Option<Collection>,
    pub target_mods: Vec<CollectionMod>,
    pub target_objects: Vec<CollectionObject>,
    pub currently_enabled_path_keys: HashSet<String>,
    pub to_enable: Vec<String>,
    pub to_disable: Vec<String>,
    pub warnings: Vec<String>,
    pub final_state_name: Option<String>,
    pub skipped_missing_paths: Vec<String>,
    pub runtime_path_rewrites: Vec<WorkspacePathRewrite>,

    pub mods_enabled: usize,
    pub mods_disabled: usize,
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
            reconcile_lock: request.reconcile_lock,
            collection: None,
            target_mods: Vec::new(),
            target_objects: Vec::new(),
            currently_enabled_path_keys: HashSet::new(),
            to_enable: Vec::new(),
            to_disable: Vec::new(),
            warnings: Vec::new(),
            final_state_name: None,
            skipped_missing_paths: Vec::new(),
            runtime_path_rewrites: Vec::new(),
            mods_enabled: 0,
            mods_disabled: 0,
        }
    }
}

/// Disk Reconcile must not perform these physical collection renames during a
/// passive startup or watcher refresh.
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

    let apply_outcome = execute_inner(ctx).await;
    update_recovery_task(ctx, &task_id, apply_outcome.is_ok()).await?;
    if apply_outcome.is_err() {
        finish_failed_apply(ctx);
    }

    apply_outcome
}

async fn update_recovery_task(
    ctx: &ApplyContext,
    task_id: &str,
    apply_succeeded: bool,
) -> Result<(), CollectionError> {
    let status = if apply_succeeded {
        crate::domain::task::TaskStatus::Completed
    } else {
        crate::domain::task::TaskStatus::Failed
    };
    let Err(error) = crate::repo::task_repo::update_status(&ctx.pool, task_id, status).await else {
        return Ok(());
    };

    log::error!("apply_pipeline: failed to update task '{task_id}' status: {error}");
    if !apply_succeeded {
        return Ok(());
    }

    finish_failed_apply(ctx);
    Err(CollectionError::Db(format!(
        "Applied collection but failed to finalize recovery task '{task_id}': {error}"
    )))
}

fn finish_failed_apply(ctx: &ApplyContext) {
    crate::services::apply_progress_service::finish(
        &ctx.game_id,
        ctx.is_safe,
        ctx.final_state_name.clone(),
        Some(corridor_label(ctx.is_safe).to_string()),
        ctx.warnings.clone(),
        false,
    );
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

    super::steps::validate_corridor::validate(ctx).await?;

    crate::services::apply_progress_service::update(
        &ctx.game_id,
        ctx.is_safe,
        "diffing",
        0,
        0,
        None,
    );
    super::steps::resolve_target::resolve(ctx).await?;

    super::steps::validate_paths::validate(ctx).await?;
    crate::services::apply_progress_service::set_warnings(
        &ctx.game_id,
        ctx.is_safe,
        ctx.warnings.clone(),
    );

    super::steps::resolve_current_state::resolve(ctx).await?;
    compute_diff(ctx);

    crate::services::apply_progress_service::update(
        &ctx.game_id,
        ctx.is_safe,
        "renaming",
        0,
        ctx.to_enable.len() + ctx.to_disable.len(),
        None,
    );
    super::steps::batch_rename::rename(ctx).await?;

    crate::services::apply_progress_service::update(
        &ctx.game_id,
        ctx.is_safe,
        "verifying",
        ctx.mods_enabled + ctx.mods_disabled,
        ctx.to_enable.len() + ctx.to_disable.len(),
        None,
    );

    ctx.final_state_name = Some(ctx.collection()?.name.clone());

    let post_ctx = PostApplyContext {
        pool: ctx.pool.clone(),
        game_id: ctx.game_id.clone(),
        is_safe: ctx.is_safe,
        mods_path: ctx.mods_path.clone(),
        hotkeys: ctx.settings.hotkeys.clone(),
        status_fields: ctx.skipped_missing_paths.is_empty().then(|| {
            crate::services::keyviewer::generator::StatusFields {
                safe_mode: ctx.is_safe,
                preset_name: ctx.final_state_name.clone(),
                ..Default::default()
            }
        }),
    };
    if let Err(error) = crate::services::app::post_apply::run_post_apply_tasks(post_ctx).await {
        log::warn!("apply_pipeline[post_apply]: {error}");
        ctx.warnings
            .push(format!("Runtime artifacts were not refreshed: {error}"));
    }

    let apply_result = ApplyResult {
        mods_enabled: ctx.mods_enabled,
        mods_disabled: ctx.mods_disabled,
        warnings: ctx.warnings.clone(),
        final_state_name: ctx.final_state_name.clone(),
        final_mode: Some(corridor_label(ctx.is_safe).to_string()),
        partial_apply: !ctx.skipped_missing_paths.is_empty(),
        skipped_missing_paths: ctx.skipped_missing_paths.clone(),
        runtime_path_rewrites: ctx.runtime_path_rewrites.clone(),
    };
    crate::services::apply_progress_service::finish(
        &ctx.game_id,
        ctx.is_safe,
        apply_result.final_state_name.clone(),
        apply_result.final_mode.clone(),
        apply_result.warnings.clone(),
        true,
    );

    Ok(apply_result)
}

fn compute_diff(ctx: &mut ApplyContext) {
    let target_keys: HashSet<String> = ctx
        .target_mods
        .iter()
        .map(|member| {
            member
                .mod_path_key
                .clone()
                .unwrap_or_else(|| folder_path_key(&member.mod_path, None))
        })
        .collect();

    ctx.to_enable = target_keys
        .difference(&ctx.currently_enabled_path_keys)
        .cloned()
        .collect();
    ctx.to_disable = ctx
        .currently_enabled_path_keys
        .difference(&target_keys)
        .cloned()
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn successful_apply_surfaces_task_finalization_failure() {
        let test_db = crate::test_utils::init_test_db().await;
        let pool = test_db.pool.clone();
        pool.close().await;
        let mut apply_context = ApplyContext::new(ApplyCollectionRequest {
            pool: &pool,
            game_id: "game-1",
            collection_id: "collection-1",
            is_safe: true,
            mods_path: PathBuf::from("E:/Mods"),
            suppressor: std::sync::Arc::new(WatcherSuppressor::new(false)),
            ignore_missing: false,
            settings: AppSettings::default(),
            reconcile_lock: None,
        });
        apply_context.final_state_name = Some("Preset".to_string());

        let error = update_recovery_task(&apply_context, "task-1", true)
            .await
            .expect_err("task finalization failure must be returned");

        assert!(matches!(error, CollectionError::Db(_)));
    }
}
