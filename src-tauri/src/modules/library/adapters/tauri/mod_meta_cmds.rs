use crate::modules::library::application::mods::{info_json, metadata};
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::reconciliation::application::disk_reconcile::emit::{
    require_applied_reconcile, settle_committed_reconcile,
};
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::guard::validate_path;
use crate::shared::errors::AppError;
use tauri::Manager;

async fn object_absolute_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: &str,
) -> Result<String, AppError> {
    let object =
        crate::modules::catalog::adapters::sqlite::object::get_game_object_by_id(pool, object_id)
            .await?
            .filter(|object| object.game_id == game_id)
            .ok_or_else(|| AppError::NotFound(format!("Object not found: {object_id}")))?;
    let mods_path = crate::modules::games::adapters::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    Ok(std::path::Path::new(&mods_path)
        .join(object.folder_path)
        .to_string_lossy()
        .to_string())
}

fn restore_info_json(path: &std::path::Path, previous: Option<&[u8]>) -> Result<(), AppError> {
    match previous {
        Some(bytes) => crate::platform::fs::atomic_file::atomic_write(path, bytes),
        None if path.exists() => std::fs::remove_file(path).map_err(AppError::from),
        None => Ok(()),
    }
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the mutation payload.
pub async fn toggle_mod_safe(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    op_lock: tauri::State<'_, MutationCoordinator>,
    game_id: String,
    folder_path: String,
    safe: bool,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationResult,
    AppError,
> {
    let diagnostics_enabled = config.get_settings().diagnostics.telemetry_enabled;
    let telemetry = app
        .state::<crate::modules::system::application::telemetry::TelemetryStore>()
        .inner()
        .clone();
    let started_at = std::time::Instant::now();
    let folder = validate_path(&config, &game_id, &folder_path)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_initial_recovery_allows_mutation(
        &app,
        &game_id,
    )?;
    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    let suppression = watcher.suppressor.suppress_paths([folder.as_ref()]);
    metadata::toggle_mod_safe(pool.inner(), &game_id, &folder, safe).await?;
    drop(suppression);
    drop(lock);
    let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
        crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
            &app,
            pool.inner(),
            &game_id,
            vec![folder.join("info.json").to_string_lossy().to_string()],
        )
        .await,
    );
    let result =
        crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationResult {
            sync_warning: settlement.sync_warning,
        };
    if diagnostics_enabled {
        let event = crate::modules::system::application::telemetry::TelemetryEvent::new(
            crate::modules::system::application::telemetry::TelemetryOperation::Toggle,
            crate::modules::system::application::telemetry::TelemetryOutcome::Success,
            crate::modules::system::application::telemetry::TelemetryErrorCode::None,
        )
        .with_duration(started_at.elapsed());
        let _ = telemetry
            .record_rollup(env!("CARGO_PKG_VERSION"), event, chrono::Utc::now())
            .await;
    }
    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub async fn suggest_random_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    input: metadata::SuggestRandomModsInput,
) -> Result<Vec<metadata::RandomModProposal>, AppError> {
    metadata::suggest_random_mods(pool.inner(), &input).await
}

#[specta::specta]
#[tauri::command]
pub async fn preview_randomized_loadout(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    input: metadata::PreviewRandomizedLoadoutInput,
) -> Result<metadata::RandomizedLoadoutPreview, AppError> {
    metadata::preview_randomized_loadout(config.inner(), pool.inner(), &input).await
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected runtime services plus the batch payload.
pub async fn apply_randomized_loadout(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    disk_reconcile_state: tauri::State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: tauri::State<'_, MutationCoordinator>,
    input: metadata::ApplyRandomizedLoadoutInput,
) -> Result<metadata::ApplyRandomizedLoadoutResult, AppError> {
    let preflight = crate::modules::reconciliation::application::disk_reconcile::emit::mutation_preflight_report_for_paths(
        &app,
        pool.inner(),
        &input.game_id,
        None,
    )
    .await?;
    let game_guard = disk_reconcile_state
        .game_lock(&input.game_id)
        .lock_owned()
        .await;
    let preview_input = metadata::PreviewRandomizedLoadoutInput {
        game_id: input.game_id.clone(),
        mod_ids: input.mod_ids.clone(),
        safety_filter: input.safety_filter,
        scope: input.scope.clone(),
    };
    let preview =
        metadata::preview_randomized_loadout(config.inner(), pool.inner(), &preview_input).await?;
    if preview.fingerprint != input.preview_fingerprint {
        return Err(AppError::Validation(
            "The randomizer review is stale. Review changes again before applying.".to_string(),
        ));
    }
    let target_paths = metadata::validate_randomized_loadout(pool.inner(), &input).await?;
    let exclusive_object_ids = preview
        .items
        .iter()
        .filter(|item| item.mode == metadata::RandomizerLoadoutMode::Exclusive)
        .map(|item| item.object_id.clone())
        .collect::<std::collections::HashSet<_>>();

    let backup = if let Some(backup) = input.backup.as_ref() {
        let name = backup.collection_name.trim();
        if name.is_empty() {
            return Err(AppError::Validation(
                "A backup collection name is required".to_string(),
            ));
        }
        let _backup_guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::CollectionMetadata,
            )
            .await?;
        let snapshot =
            crate::modules::collections::application::collection::snapshot_live_state_passively(
                pool.inner(),
                &input.game_id,
                name,
            )
            .await
            .map_err(|error| AppError::Validation(error.to_string()))?;
        Some(metadata::RandomizedLoadoutBackupResult {
            collection_id: snapshot.collection_id,
            collection_name: snapshot.collection_name,
            reused: !snapshot.created,
        })
    } else {
        None
    };

    let prepared = crate::modules::workspace::application::workspace::switch::prepare_randomized_loadout_switch(
        config.inner(),
        pool.inner(),
        &input.game_id,
        &target_paths,
        &exclusive_object_ids,
    )
    .await?;
    let planned_renames = prepared.journal_steps();
    let preflight_paths = planned_renames
        .iter()
        .flat_map(|(_, old_path, new_path)| {
            [
                old_path.to_string_lossy().into_owned(),
                new_path.to_string_lossy().into_owned(),
            ]
        })
        .collect::<Vec<_>>();
    if crate::modules::reconciliation::application::disk_reconcile::emit::conflicts_intersect_paths(
        &preflight.folder_conflicts,
        &preflight_paths,
    ) {
        return Err(
            crate::modules::reconciliation::application::disk_reconcile::emit::folder_conflict_mutation_error(),
        );
    }
    let journal_steps = planned_renames
        .into_iter()
        .map(|(sequence, old_path, new_path)| {
            crate::modules::mutation::api::PlannedStep::rename(sequence, old_path, new_path)
        })
        .collect::<Vec<_>>();
    if journal_steps.is_empty() {
        let _guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::WorkspaceConfiguration,
            )
            .await?;
        let result = prepared.execute(&app, &watcher)?;
        let history_warning =
            crate::modules::library::adapters::sqlite::mods::record_randomizer_history(
                pool.inner(),
                &input.game_id,
                &preview
                    .items
                    .iter()
                    .map(|item| (item.object_id.clone(), item.selected_mod_id.clone()))
                    .collect::<Vec<_>>(),
            )
            .await
            .err()
            .map(|error| {
                format!("Loadout was applied, but anti-repeat history was not saved: {error}")
            });
        return Ok(metadata::ApplyRandomizedLoadoutResult {
            impact: result.impact,
            backup,
            sync_warning: result.sync_warning,
            history_warning,
        });
    }

    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "randomized-loadout",
            input.game_id.clone(),
            journal_steps,
        ))
        .await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    let result = match prepared.execute_with_outcome(&app, &watcher) {
        Ok(result) => result,
        Err(
            crate::modules::workspace::application::workspace::switch::PreparedWorkspaceExecutionError::Apply(error),
        ) => {
            for (sequence, _, _) in prepared.journal_steps() {
                mutation_lease.mark_step_rolled_back(sequence)?;
            }
            mutation_lease.begin_rollback()?;
            mutation_lease.finish_rollback()?;
            return Err(error);
        }
        Err(
            crate::modules::workspace::application::workspace::switch::PreparedWorkspaceExecutionError::Compensation(error),
        ) => {
            mutation_lease.fail(format!(
                "Randomized loadout compensation failed; workspace requires recovery: {error}"
            ))?;
            return Err(error);
        }
    };
    for (sequence, _, _) in prepared.journal_steps() {
        mutation_lease.mark_step_applied(sequence)?;
    }
    match crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
        &app,
        pool.inner(),
        &input.game_id,
        &mutation_lease,
    )
    .await
    .and_then(require_applied_reconcile)
    {
        Ok(reconcile) => {
            mutation_lease.mark_db_committed()?;
            mutation_lease.commit()?;
            let history_warning = crate::modules::library::adapters::sqlite::mods::record_randomizer_history(
                pool.inner(),
                &input.game_id,
                &preview.items.iter().map(|item| (item.object_id.clone(), item.selected_mod_id.clone())).collect::<Vec<_>>(),
            ).await.err().map(|error| format!("Loadout was applied, but anti-repeat history was not saved: {error}"));
            Ok(metadata::ApplyRandomizedLoadoutResult {
                impact: result.impact,
                backup,
                sync_warning: settle_committed_reconcile(Ok(reconcile)).sync_warning,
                history_warning,
            })
        }
        Err(error) => {
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) = prepared.rollback(&watcher) {
                let combined = format!("{error}; randomized loadout rollback failed: {rollback_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            for (sequence, _, _) in prepared.journal_steps() {
                mutation_lease.mark_step_rolled_back(sequence)?;
            }
            if let Err(rollback_reconcile_error) = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                &app,
                pool.inner(),
                &input.game_id,
                &mutation_lease,
            )
            .await.and_then(require_applied_reconcile) {
                let combined = format!(
                    "{error}; randomized loadout rollback projection failed: {rollback_reconcile_error}"
                );
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            mutation_lease.finish_rollback()?;
            Err(error)
        }
    }
}

#[specta::specta]
#[tauri::command]
pub async fn get_active_mod_conflicts(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<crate::modules::workspace::application::scanner::conflict::ConflictInfo>, AppError>
{
    metadata::get_active_mod_conflicts(pool.inner(), &game_id).await
}

#[specta::specta]
#[tauri::command]
pub async fn read_mod_info(
    config: tauri::State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<Option<info_json::ModInfo>, AppError> {
    let path = validate_path(&config, &game_id, &folder_path)?;
    Ok(info_json::read_info_json(&path)?)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the mutation payload.
pub async fn update_mod_info(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: tauri::State<'_, MutationCoordinator>,
    game_id: String,
    folder_path: String,
    update: info_json::ModInfoUpdate,
) -> Result<info_json::ModInfo, AppError> {
    if update.is_safe.is_some() {
        return Err(AppError::Validation(
            "Safety changes must use toggle_mod_safe".to_string(),
        ));
    }
    let path = validate_path(&config, &game_id, &folder_path)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_initial_recovery_allows_mutation(
        &app,
        &game_id,
    )?;
    let info_path = path.join("info.json");
    let changed_path = info_path.to_string_lossy().to_string();
    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    let guard = state.suppressor.suppress_paths([path.as_ref()]);
    let previous = match std::fs::read(&info_path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let info = info_json::update_info_json(&path, &update)?;
    drop(guard);
    drop(lock);
    let reconcile = crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
        &app,
        pool.inner(),
        &game_id,
        vec![changed_path],
    )
    .await;
    let failure = match reconcile {
        Ok(result) if result.status.applied() => None,
        Ok(result) => Some(AppError::Io(result.error_message.unwrap_or_else(|| {
            format!(
                "Metadata reconcile was blocked with status {:?}",
                result.status
            )
        }))),
        Err(error) => Some(error),
    };
    if let Some(failure) = failure {
        let rollback_lock = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata,
            )
            .await?;
        let rollback_guard = state.suppressor.suppress_paths([path.as_ref()]);
        let rollback = restore_info_json(&info_path, previous.as_deref());
        drop(rollback_guard);
        drop(rollback_lock);
        let repair = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile(
            &app,
            pool.inner(),
            &game_id,
        )
        .await;
        return match (rollback, repair) {
            (Ok(()), Ok(_)) => Err(failure),
            (rollback, repair) => Err(AppError::Io(format!(
                "{failure}; rollback result: {}; recovery reconcile result: {}",
                rollback
                    .err()
                    .map_or_else(|| "ok".to_string(), |error| error.to_string()),
                repair
                    .err()
                    .map_or_else(|| "ok".to_string(), |error| error.to_string())
            ))),
        };
    }

    Ok(info)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the mutation payload.
pub async fn set_mod_category(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: tauri::State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: tauri::State<'_, MutationCoordinator>,
    game_id: String,
    folder_path: String,
    category: String,
) -> Result<(), AppError> {
    let folder = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [folder.to_string_lossy().to_string()];
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let game_lock = disk_reconcile_state.game_lock(&game_id);
    let game_guard = game_lock.lock().await;
    let operation_guard = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    metadata::set_mod_category(&pool, &game_id, &folder, &category).await?;
    drop(operation_guard);
    drop(game_guard);
    let runtime_effects = crate::modules::system::application::app::runtime_effects::settle_committed_runtime_effects(
        disk_reconcile_state.inner(),
        crate::modules::system::application::app::runtime_effects::RuntimeSideEffects {
            pool: &pool,
            config: &config,
            game_id: &game_id,
            collections_dirty: false,
            overlay_refresh: true,
            overlay_cause: crate::modules::system::application::app::post_apply::OverlaySyncCause::EffectiveModsChanged,
        },
    )
    .await;
    if let Some(warning) = runtime_effects.warning {
        log::warn!("Metadata category update committed with pending runtime effects: {warning}");
    }

    Ok(())
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the mutation payload.
pub async fn set_object_mods_category(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: tauri::State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: tauri::State<'_, MutationCoordinator>,
    game_id: String,
    object_id: String,
    category: String,
) -> Result<usize, AppError> {
    let preflight_paths = [object_absolute_path(pool.inner(), &game_id, &object_id).await?];
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let game_lock = disk_reconcile_state.game_lock(&game_id);
    let game_guard = game_lock.lock().await;
    let operation_guard = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    let updated =
        crate::modules::catalog::application::objects::mutate::set_object_and_mods_category(
            pool.inner(),
            &game_id,
            &object_id,
            &category,
        )
        .await?;
    drop(operation_guard);
    drop(game_guard);

    let runtime_effects = crate::modules::system::application::app::runtime_effects::settle_committed_runtime_effects(
        disk_reconcile_state.inner(),
        crate::modules::system::application::app::runtime_effects::RuntimeSideEffects {
            pool: pool.inner(),
            config: &config,
            game_id: &game_id,
            collections_dirty: true,
            overlay_refresh: true,
            overlay_cause: crate::modules::system::application::app::post_apply::OverlaySyncCause::EffectiveModsChanged,
        },
    )
    .await;
    if let Some(warning) = runtime_effects.warning {
        log::warn!("Object metadata update committed with pending runtime effects: {warning}");
    }

    Ok(updated)
}

#[derive(serde::Deserialize, specta::Type)]
pub struct MoveModsToObjectInput {
    pub game_id: String,
    pub folder_paths: Vec<String>,
    pub target_object_id: String,
    pub target_subpath: Option<String>,
    pub status: Option<String>,
}

#[specta::specta]
#[tauri::command]
pub async fn list_move_targets_for_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    object_id: String,
) -> Result<
    Vec<crate::modules::library::application::mods::organizer_ext::WorkspaceMoveTarget>,
    AppError,
> {
    crate::modules::library::application::mods::organizer_ext::list_move_targets_for_object_service(
        pool.inner(),
        &game_id,
        &object_id,
    )
    .await
}

#[specta::specta]
#[tauri::command]
pub async fn move_mods_to_object(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    op_lock: tauri::State<'_, MutationCoordinator>,
    disk_reconcile_state: tauri::State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    watcher: tauri::State<'_, WatcherState>,
    input: MoveModsToObjectInput,
) -> Result<crate::modules::library::application::mods::bulk::BulkResult, AppError> {
    let folders =
        crate::platform::fs::guard::validate_paths(&config, &input.game_id, &input.folder_paths)?;
    let mut preflight_paths = folders
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    preflight_paths
        .push(object_absolute_path(pool.inner(), &input.game_id, &input.target_object_id).await?);
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &input.game_id,
        Some(&preflight_paths),
    )
    .await?;
    let game_guard = disk_reconcile_state
        .game_lock(&input.game_id)
        .lock_owned()
        .await;
    let prepared =
        crate::modules::library::application::mods::organizer_move::prepare_move_mods_to_object(
            pool.inner(),
            crate::modules::library::application::mods::organizer_ext::MoveModsToObjectParams {
                game_id: &input.game_id,
                folder_paths: &folders,
                target_object_id: &input.target_object_id,
                target_subpath: input.target_subpath.as_deref(),
                status: input.status.as_deref(),
            },
        )
        .await?;
    let journal_steps = prepared
        .journal_steps()
        .into_iter()
        .map(|(sequence, old_path, new_path)| {
            crate::modules::mutation::api::PlannedStep::rename(sequence, old_path, new_path)
        })
        .collect::<Vec<_>>();
    if journal_steps.is_empty() {
        let _guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata,
            )
            .await?;
        return Ok(
            crate::modules::library::application::mods::organizer_move::execute_prepared_move(
                &watcher, &prepared,
            )?
            .result,
        );
    }
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "organizer-move",
            input.game_id.clone(),
            journal_steps,
        ))
        .await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    let organizer =
        match crate::modules::library::application::mods::organizer_move::execute_prepared_move(
            &watcher, &prepared,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                for (sequence, _, _) in prepared.journal_steps() {
                    mutation_lease.mark_step_rolled_back(sequence)?;
                }
                mutation_lease.begin_rollback()?;
                mutation_lease.finish_rollback()?;
                return Err(error);
            }
        };
    for (sequence, _, _) in prepared.journal_steps() {
        mutation_lease.mark_step_applied(sequence)?;
    }
    let mut result = organizer.result;

    // Convergence: reconcile source and destination roots after the move.
    // The target root is included explicitly: a partial failure can leave a
    // folder already renamed under the target while its path is absent from
    // `success`, and reconciling only the sources would prune its row.
    let mut changed_paths = prepared.changed_paths();
    if let Some(target_obj) =
        crate::modules::catalog::adapters::sqlite::object::get_game_object_by_id(
            pool.inner(),
            &input.target_object_id,
        )
        .await?
    {
        if let Some(mods_path) = crate::modules::games::adapters::sqlite::game::get_mod_path(
            pool.inner(),
            &input.game_id,
        )
        .await?
        {
            changed_paths.push(
                std::path::Path::new(&mods_path)
                    .join(&target_obj.folder_path)
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }
    // Quiet: the move's caller publishes its own refresh from the result.
    let reconcile = crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile_with_path_hints_under_lease(
            &app,
            pool.inner(),
            &input.game_id,
            changed_paths,
            organizer.path_hints,
            &mutation_lease,
        )
        .await;
    let settlement = match reconcile {
        Ok(reconcile) if reconcile.status.applied() => {
            mutation_lease.mark_db_committed()?;
            mutation_lease.commit()?;
            crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(Ok(reconcile))
        }
        Ok(reconcile) => {
            let error = AppError::Io(format!(
                "Organizer reconcile requires attention: {:?}",
                reconcile.status
            ));
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) =
                crate::modules::library::application::mods::organizer_move::rollback_prepared_move(
                    &watcher, &prepared,
                )
            {
                let combined = format!("{error}; organizer rollback failed: {rollback_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            for (sequence, _, _) in prepared.journal_steps() {
                mutation_lease.mark_step_rolled_back(sequence)?;
            }
            crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                &app,
                pool.inner(),
                &input.game_id,
                &mutation_lease,
            )
            .await?;
            mutation_lease.finish_rollback()?;
            return Err(error);
        }
        Err(error) => {
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) =
                crate::modules::library::application::mods::organizer_move::rollback_prepared_move(
                    &watcher, &prepared,
                )
            {
                let combined = format!("{error}; organizer rollback failed: {rollback_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            for (sequence, _, _) in prepared.journal_steps() {
                mutation_lease.mark_step_rolled_back(sequence)?;
            }
            crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                &app,
                pool.inner(),
                &input.game_id,
                &mutation_lease,
            )
            .await?;
            mutation_lease.finish_rollback()?;
            return Err(error);
        }
    };
    if let Some(reconcile) = settlement.reconcile {
        result
            .collection_impact
            .merge(reconcile.collection_reference_impact);
        for update in reconcile.path_updates {
            if !result.path_rewrites.iter().any(|rewrite| {
                rewrite.old_path.eq_ignore_ascii_case(&update.from)
                    && rewrite.new_path.eq_ignore_ascii_case(&update.to)
            }) {
                result.path_rewrites.push(
                    crate::modules::workspace::domain::workspace::WorkspacePathRewrite {
                        old_path: update.from,
                        new_path: update.to,
                    },
                );
            }
        }
    }
    result.sync_warning = settlement.sync_warning;

    Ok(result)
}

#[cfg(test)]
#[path = "tests/mod_meta_cmds_tests.rs"]
mod tests;
