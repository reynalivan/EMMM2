use std::collections::HashSet;
use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::shared::path_key::folder_path_key;
use crate::modules::collections::domain::collection::{ApplyResult, Collection, CollectionMod, CollectionObject};
use crate::shared::errors::CollectionError;
use crate::modules::workspace::domain::task::TaskStatus;
use crate::modules::workspace::domain::workspace::WorkspacePathRewrite;
use crate::modules::system::application::app::post_apply::PostApplyContext;
use crate::modules::collections::application::collection::ApplyCollectionRequest;
use crate::modules::settings::application::config::AppSettings;
use crate::modules::workspace::application::scanner::watcher::WatcherSuppressor;

// ---------------------------------------------------------------------------
// ApplyPipeline — Composable collection apply operation
// ---------------------------------------------------------------------------

/// Context passed through all pipeline steps during a collection apply.
pub struct ApplyContext {
    pub pool: SqlitePool,
    pub game_id: String,
    pub collection_id: String,
    pub mods_path: PathBuf,
    pub suppressor: std::sync::Arc<WatcherSuppressor>,
    pub ignore_missing: bool,
    pub settings: AppSettings,
    pub finalize_active_collection: bool,
    pub final_active_collection_id: Option<String>,
    pub mutation_started: bool,
    pub rollback_collection_id: Option<String>,
    pub rollback_active_collection_id: Option<String>,

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

impl ApplyContext {
    /// The collection row loaded once by the validation step.
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
            mods_path: request.mods_path,
            suppressor: request.suppressor,
            ignore_missing: request.ignore_missing,
            settings: request.settings,
            finalize_active_collection: request.capture_last_changes,
            final_active_collection_id: request
                .capture_last_changes
                .then(|| request.collection_id.to_string()),
            mutation_started: false,
            rollback_collection_id: None,
            rollback_active_collection_id: None,
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
pub async fn execute(
    ctx: &mut ApplyContext,
    task_id: &str,
    expected_status: TaskStatus,
    settle_normal_failure: bool,
) -> Result<ApplyResult, CollectionError> {
    crate::modules::library::application::apply_progress::start(&ctx.game_id);

    match execute_inner(ctx).await {
        Ok(result) => {
            if let Err(error) = finalize_apply(ctx, task_id, expected_status).await {
                if settle_normal_failure {
                    settle_normal_apply_failure(ctx, task_id).await;
                }
                finish_failed_apply(ctx);
                return Err(error);
            }
            crate::modules::library::application::apply_progress::finish(
                &ctx.game_id,
                result.final_state_name.clone(),
                result.warnings.clone(),
                true,
            );
            Ok(result)
        }
        Err(error) => {
            if settle_normal_failure {
                settle_normal_apply_failure(ctx, task_id).await;
            }
            finish_failed_apply(ctx);
            Err(error)
        }
    }
}

async fn finalize_apply(
    ctx: &ApplyContext,
    task_id: &str,
    expected_status: TaskStatus,
) -> Result<(), CollectionError> {
    let outcome: Result<(), sqlx::Error> = async {
        let mut tx = ctx.pool.begin().await?;
        if ctx.finalize_active_collection {
            crate::modules::collections::adapters::outbound::sqlite::runtime::set_active_tx(
                &mut tx,
                &ctx.game_id,
                ctx.final_active_collection_id.as_deref(),
            )
            .await?;
        }
        let completed = crate::modules::workspace::adapters::outbound::sqlite::task::compare_and_set_status_tx(
            &mut tx,
            task_id,
            expected_status,
            TaskStatus::Completed,
        )
        .await?;
        if !completed {
            return Err(sqlx::Error::RowNotFound);
        }
        tx.commit().await
    }
    .await;

    outcome.map_err(|error| {
        log::error!("apply_pipeline: failed to finalize task '{task_id}': {error}");
        CollectionError::Db(format!(
            "Applied collection but failed to finalize recovery task '{task_id}': {error}"
        ))
    })
}

async fn settle_normal_apply_failure(ctx: &ApplyContext, task_id: &str) {
    let status = if ctx.mutation_started {
        TaskStatus::Pending
    } else {
        TaskStatus::Failed
    };
    if let Err(error) = crate::modules::workspace::adapters::outbound::sqlite::task::compare_and_set_status(
        &ctx.pool,
        task_id,
        TaskStatus::Running,
        status,
    )
    .await
    {
        log::error!("apply_pipeline: failed to mark task '{task_id}' as failed: {error}");
    }
}

fn finish_failed_apply(ctx: &ApplyContext) {
    crate::modules::library::application::apply_progress::finish(
        &ctx.game_id,
        ctx.final_state_name.clone(),
        ctx.warnings.clone(),
        false,
    );
}

async fn execute_inner(ctx: &mut ApplyContext) -> Result<ApplyResult, CollectionError> {
    crate::modules::library::application::apply_progress::update(&ctx.game_id, "preparing", 0, 0, None);
    if !ctx.mods_path.exists() || !ctx.mods_path.is_dir() {
        return Err(CollectionError::RuntimeState(
            crate::shared::errors::RuntimeStateError::NoModsPath {
                game_id: ctx.game_id.clone(),
            },
        ));
    }

    super::steps::validate_collection::validate(ctx).await?;

    crate::modules::library::application::apply_progress::update(&ctx.game_id, "diffing", 0, 0, None);
    super::steps::resolve_target::resolve(ctx).await?;

    super::steps::validate_paths::validate(ctx).await?;
    crate::modules::library::application::apply_progress::set_warnings(&ctx.game_id, ctx.warnings.clone());

    super::steps::resolve_current_state::resolve(ctx).await?;
    compute_diff(ctx);
    ctx.mutation_started = true;

    crate::modules::library::application::apply_progress::update(
        &ctx.game_id,
        "renaming",
        0,
        ctx.to_enable.len() + ctx.to_disable.len(),
        None,
    );
    super::steps::batch_rename::rename(ctx).await?;

    crate::modules::library::application::apply_progress::update(
        &ctx.game_id,
        "verifying",
        ctx.mods_enabled + ctx.mods_disabled,
        ctx.to_enable.len() + ctx.to_disable.len(),
        None,
    );

    ctx.final_state_name = Some(ctx.collection()?.name.clone());

    let post_ctx = PostApplyContext {
        pool: ctx.pool.clone(),
        game_id: ctx.game_id.clone(),
        mods_path: ctx.mods_path.clone(),
        hotkeys: ctx.settings.hotkeys.clone(),
        status_fields: ctx.skipped_missing_paths.is_empty().then(|| {
            crate::modules::automation::application::keyviewer::generator::StatusFields {
                preset_name: ctx.final_state_name.clone(),
                ..Default::default()
            }
        }),
    };
    if let Err(error) = crate::modules::system::application::app::post_apply::run_post_apply_tasks(post_ctx).await {
        log::warn!("apply_pipeline[post_apply]: {error}");
        ctx.warnings
            .push(format!("Runtime artifacts were not refreshed: {error}"));
    }

    let apply_result = ApplyResult {
        mods_enabled: ctx.mods_enabled,
        mods_disabled: ctx.mods_disabled,
        warnings: ctx.warnings.clone(),
        final_state_name: ctx.final_state_name.clone(),
        partial_apply: !ctx.skipped_missing_paths.is_empty(),
        skipped_missing_paths: ctx.skipped_missing_paths.clone(),
        runtime_path_rewrites: ctx.runtime_path_rewrites.clone(),
        sync_warning: None,
    };
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
            capture_last_changes: false,
            mods_path: PathBuf::from("E:/Mods"),
            suppressor: std::sync::Arc::new(WatcherSuppressor::new(false)),
            ignore_missing: false,
            settings: AppSettings::default(),
        });
        apply_context.final_state_name = Some("Preset".to_string());

        let error = finalize_apply(&apply_context, "task-1", TaskStatus::Pending)
            .await
            .expect_err("task finalization failure must be returned");

        assert!(matches!(error, CollectionError::Db(_)));
    }

    #[tokio::test]
    async fn finalization_failure_rolls_back_active_baseline_and_task_completion_together() {
        let test_db = crate::test_utils::init_test_db().await;
        crate::test_utils::insert_test_game(
            &test_db.pool,
            &crate::test_utils::TestGameFixture {
                id: "game-atomic-finalize",
                name: "Atomic finalize",
                game_type: crate::modules::games::domain::models::GameType::GIMI,
                path: "E:/Games/Atomic",
                mods_path: Some("E:/Mods/Atomic"),
            },
        )
        .await
        .expect("seed game");
        for (id, name) in [("baseline-before", "Before"), ("baseline-after", "After")] {
            crate::modules::collections::adapters::outbound::sqlite::create(
                &test_db.pool,
                id,
                "game-atomic-finalize",
                name,
                true,
                false,
            )
            .await
            .expect("seed collection");
        }
        crate::modules::collections::adapters::outbound::sqlite::runtime::set_active(
            &test_db.pool,
            "game-atomic-finalize",
            Some("baseline-before"),
        )
        .await
        .expect("seed active baseline");
        crate::modules::workspace::adapters::outbound::sqlite::task::create_claimed_task(
            &test_db.pool,
            "task-atomic-finalize",
            "game-atomic-finalize",
            crate::modules::workspace::domain::task::TASK_TYPE_APPLY_COLLECTION,
            Some("baseline-after"),
        )
        .await
        .expect("seed running task");
        sqlx::query(
            "CREATE TRIGGER fail_task_completion BEFORE UPDATE OF status ON tasks \
             WHEN NEW.id = 'task-atomic-finalize' AND NEW.status = 'COMPLETED' \
             BEGIN SELECT RAISE(FAIL, 'injected task completion failure'); END",
        )
        .execute(&test_db.pool)
        .await
        .expect("install failure trigger");

        let mut apply_context = ApplyContext::new(ApplyCollectionRequest {
            pool: &test_db.pool,
            game_id: "game-atomic-finalize",
            collection_id: "baseline-after",
            capture_last_changes: false,
            mods_path: PathBuf::from("E:/Mods/Atomic"),
            suppressor: std::sync::Arc::new(WatcherSuppressor::new(false)),
            ignore_missing: false,
            settings: AppSettings::default(),
        });
        apply_context.finalize_active_collection = true;
        apply_context.final_active_collection_id = Some("baseline-after".to_string());

        let error = finalize_apply(&apply_context, "task-atomic-finalize", TaskStatus::Running)
            .await
            .expect_err("injected finalization failure");
        assert!(matches!(error, CollectionError::Db(_)));

        let runtime =
            crate::modules::collections::adapters::outbound::sqlite::runtime::get(&test_db.pool, "game-atomic-finalize")
                .await
                .expect("load runtime")
                .expect("runtime exists");
        assert_eq!(
            runtime.active_collection_id.as_deref(),
            Some("baseline-before"),
            "active baseline update must roll back with failed task completion"
        );
        let task = crate::modules::workspace::adapters::outbound::sqlite::task::get_task_by_id(&test_db.pool, "task-atomic-finalize")
            .await
            .expect("load task")
            .expect("task exists");
        assert_eq!(task.status, TaskStatus::Running);
    }
}
