//! Startup bootstrap steps extracted from the Tauri `.setup()` closure.
//!
//! Ordering is load-bearing: window recovery -> thumbnail cache -> database pool
//! (with corrupt-db recovery) -> config -> hotkeys -> task GC + boot reconcile.
//! `lib.rs` keeps the `.manage()` / plugin / command registration and calls these
//! in the same order.

use tauri::{Emitter, Manager};

const MUTATION_JOURNAL_HISTORY_LIMIT: usize = 256;

async fn initialize_mutation_coordinator(
    app: &tauri::AppHandle,
) -> Result<Vec<String>, crate::shared::errors::AppError> {
    let app_data_dir = app.path().app_data_dir()?;
    let journal_path = app_data_dir.join("mutation-journal.json");
    let journal = std::sync::Arc::new(crate::modules::mutation::journal::OperationJournal::open(
        journal_path,
        MUTATION_JOURNAL_HISTORY_LIMIT,
    )?);
    if super::readonly_audit::is_active() {
        super::readonly_audit::ensure_empty_journal(journal.entries().len())?;
    }
    let game_roots = app
        .state::<crate::modules::settings::application::config::ConfigService>()
        .with_settings(|settings| {
            settings
                .games
                .iter()
                .map(|game| (game.id.clone(), game.mod_path.clone()))
                .collect::<std::collections::HashMap<_, _>>()
        });
    let pool = app.state::<sqlx::SqlitePool>();
    let mut ready_to_move_roots = std::collections::HashMap::new();
    for game_id in game_roots.keys() {
        match crate::modules::ingestion::api::import_batch::ready_to_move::resolve_mod_inbox_root(
            app,
            pool.inner(),
            game_id,
            None,
        )
        .await
        {
            Ok(root) => {
                ready_to_move_roots.insert(game_id.clone(), root);
            }
            Err(error) => {
                log::warn!(
                    "startup: ReadyToMove recovery root for game {game_id} is unavailable: {error}"
                );
            }
        }
    }
    let roots = crate::modules::mutation::recovery::RecoveryRoots::new(
        game_roots.clone(),
        app_data_dir.join("import-staging"),
    )
    .with_ready_to_move_roots(ready_to_move_roots);
    let recovered = crate::modules::mutation::recovery::RecoveryRunner::new(journal.clone(), roots)
        .with_projection_pool(pool.inner().clone())
        .run_recovery()
        .await?;
    let recorded_quarantines = journal
        .entries()
        .into_iter()
        .filter(|operation| {
            matches!(
                operation.kind.as_str(),
                "delete-mod" | "bulk-delete" | "trash-folder-conflict-candidate"
            ) && operation.status == crate::modules::mutation::journal::OperationStatus::Completed
                && operation.database_projection_status
                    == crate::modules::mutation::journal::DatabaseProjectionStatus::Committed
        })
        .flat_map(|operation| {
            let game_root = game_roots.get(&operation.game_id).cloned();
            let expected_root_identity = operation.source_epoch;
            operation.steps.into_iter().filter_map(move |step| {
                let path = (step.status == crate::modules::mutation::journal::StepStatus::Applied)
                    .then_some(step.new_path)
                    .flatten()?;
                Some(
                    crate::modules::library::api::mods::trash::RecordedQuarantine {
                        path,
                        source_path: step.old_path,
                        game_root: game_root.as_ref()?.clone(),
                        expected_root_identity: expected_root_identity.clone(),
                        expected_identity: step.expected_identity,
                    },
                )
            })
        })
        .collect::<Vec<_>>();
    for warning in crate::modules::library::api::mods::trash::finalize_recorded_quarantines(
        recorded_quarantines,
    ) {
        log::warn!("Pending trash quarantine cleanup failed: {warning}");
    }
    app.state::<crate::modules::mutation::coordinator::MutationCoordinator>()
        .configure(journal)?;
    Ok(recovered)
}

/// Re-centers the main window when it was restored onto a monitor that no longer
/// exists (disconnected display).
pub fn center_window_if_offscreen(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        if let Ok(outer_pos) = window.outer_position() {
            let mut is_visible = false;
            if let Ok(monitors) = window.available_monitors() {
                for monitor in monitors {
                    let m_pos = monitor.position();
                    let m_size = monitor.size();
                    if outer_pos.x >= m_pos.x
                        && outer_pos.x < m_pos.x + m_size.width as i32
                        && outer_pos.y >= m_pos.y
                        && outer_pos.y < m_pos.y + m_size.height as i32
                    {
                        is_visible = true;
                        break;
                    }
                }
            }
            if !is_visible {
                log::warn!(
                    "Window spawned off-screen (disconnected monitor). Centering on primary."
                );
                let _ = window.center();
            }
        }
    }
}

/// Opens the SQLite pool and runs migrations. A corrupt database is renamed to
/// `app_corrupt_<unix_ts>.db` and re-created from scratch. Stable-id and
/// unicode-key backfills run afterwards and are best-effort.
#[cfg(desktop)]
pub fn init_pool(app_data_dir: &std::path::Path) -> sqlx::SqlitePool {
    use tauri::async_runtime::block_on;

    let db_path = app_data_dir.join("app.db");
    if !app_data_dir.exists() {
        let _ = std::fs::create_dir_all(app_data_dir);
    }

    block_on(async {
        use sqlx::sqlite::{
            SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
        };
        let try_init = || async {
            // sqlx leaves these unset, so SQLite falls back to `DELETE` + `FULL`
            // — roughly three fsyncs per autocommit statement, paid per row by
            // every bulk toggle, delete and projection refresh.
            //
            // `NORMAL` under WAL can lose the last transactions on power loss,
            // which is the right trade here: the filesystem is the source of
            // truth and a lost index write is repaired by the next reconcile.
            let opts = SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal);

            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect_with(opts)
                .await?;

            sqlx::migrate!("./migrations").run(&pool).await?;

            Ok::<sqlx::SqlitePool, sqlx::Error>(pool)
        };

        let p = match try_init().await {
            Ok(pool) => pool,
            Err(error) if super::app_service::database_is_corrupt(&db_path).await => {
                log::error!("Database integrity check failed after initialization error: {error}. Attempting recovery...");
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let backup_path = app_data_dir.join(format!("app_corrupt_{}.db", timestamp));
                super::app_service::quarantine_database_files(&db_path, &backup_path)
                    .expect("Failed to preserve corrupt database files during recovery");
                try_init()
                    .await
                    .expect("Failed to initialize database after recovery")
            }
            Err(error) => {
                panic!("Database initialization failed without confirmed corruption: {error}")
            }
        };

        if let Err(e) =
            crate::modules::system::adapters::sqlite::utils::unicode_keys::ensure_unicode_keys(&p)
                .await
        {
            log::warn!("Unicode key backfill skipped: {e}");
        }
        p
    })
}

/// Builds the hotkey manager and registers its bindings. Construction cannot
/// fail; a shortcut the OS refuses just leaves the manager with no bindings
/// registered, which reads as disabled.
pub fn init_hotkey_manager(
    app_handle: &tauri::AppHandle,
    hotkey_config: &crate::modules::automation::application::hotkeys::HotkeyConfig,
) -> crate::modules::automation::application::hotkeys::manager::HotkeyManager {
    let hk_manager = crate::modules::automation::application::hotkeys::manager::HotkeyManager::new(
        hotkey_config,
    );
    if let Err(error) = hk_manager.update_bindings(app_handle, hotkey_config) {
        log::warn!("startup: hotkey registration failed, continuing disabled: {error}");
    }
    hk_manager
}

/// Marks browser downloads that were mid-flight when the process last exited as
/// `failed`. The import-batch recovery below owns resumable import state.
async fn recover_interrupted_transfers(pool: &sqlx::SqlitePool) {
    match crate::modules::browser::adapters::sqlite::browser::fail_interrupted_downloads(pool).await
    {
        Ok(count) if count > 0 => log::info!("startup: failed {count} interrupted download(s)"),
        Ok(_) => {}
        Err(error) => log::warn!("startup: download recovery failed: {error}"),
    }
}

struct StartupRecoveryOwner<'a> {
    game_id: &'a str,
    generation: Option<u64>,
}

async fn reconcile_startup_game(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    watcher_state: &crate::modules::workspace::application::scanner::watcher::WatcherState,
    disk_reconcile_state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    mutation_coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    owner: StartupRecoveryOwner<'_>,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    crate::shared::errors::AppError,
> {
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::{
        try_reconcile_disk_state_for_prewarm, DiskReconcileRequest,
    };
    let game_id = owner.game_id;
    let activation_guard = disk_reconcile_state.activation_guard().await;
    let settings = config.get_settings();
    let background = settings.active_game_id.as_deref() != Some(game_id);
    let game = settings
        .games
        .into_iter()
        .find(|game| game.id == game_id)
        .ok_or_else(|| {
            crate::shared::errors::AppError::NotFound(format!("Game '{game_id}' was removed"))
        })?;
    let runtime_config_path = game.instance_path.join("d3dx.ini");
    if let Some(generation) = owner.generation {
        if !disk_reconcile_state.initial_recovery_owner_matches_configuration(
            game_id,
            generation,
            &game.mod_path,
            &runtime_config_path,
        ) {
            return Err(crate::shared::errors::AppError::Cancelled);
        }
    }
    let watcher_session = if !background {
        watcher_state
            .current_session_for_coverage(&game.mod_path, Some(&runtime_config_path))
            .map(|session| session.generation())
    } else {
        None
    };
    let watcher_session = match watcher_session {
        Some(session) => session,
        None => crate::modules::workspace::api::start_inactive_watcher_with_activation_guard(
            app,
            watcher_state,
            game_id,
            &game.mod_path,
            Some(&runtime_config_path),
            &activation_guard,
            if background {
                crate::modules::workspace::api::WatcherInstallScope::InactiveOnly
            } else {
                crate::modules::workspace::api::WatcherInstallScope::StartupSelected
            },
        )
        .await
        .map_err(crate::shared::errors::AppError::from)?
        .ok_or(crate::shared::errors::AppError::Cancelled)?,
    };
    let observed_generation = disk_reconcile_state
        .authority_event_generation(game_id, watcher_session)
        .unwrap_or(0);
    drop(activation_guard);
    let context = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
            pool,
            config,
            state: disk_reconcile_state,
            watcher_suppressor: watcher_state.suppressor.clone(),
            operation_lock: mutation_coordinator.inner_lock(),
            progress_reporter: Some(std::sync::Arc::new(
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter::new(
                    app.clone(),
                    game_id.to_string(),
                    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::StartupBoot,
                ),
            )),
        };
    let request = || {
        let request = DiskReconcileRequest::manual(
            game_id.to_string(),
            crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::StartupBoot,
            Vec::new(),
            true,
        )
        .defer_overlay_sync();
        match owner.generation {
            Some(generation) => request.for_initial_recovery(generation),
            None => request,
        }
    };
    let result = if !background {
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state(context, request()).await?
    } else {
        loop {
            if let Some(result) =
                try_reconcile_disk_state_for_prewarm(context.clone(), request()).await?
            {
                break result;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    };
    let _activation_guard = disk_reconcile_state.activation_guard().await;
    if owner.generation.is_some_and(|generation| {
        !disk_reconcile_state.initial_recovery_generation_is_pending(game_id, generation)
    }) {
        return Err(crate::shared::errors::AppError::Cancelled);
    }
    let active_now = config.get_settings().active_game_id.as_deref() == Some(game_id);
    let active_session_matches = watcher_state
        .current_session_for_coverage(&game.mod_path, Some(&runtime_config_path))
        .is_some_and(|session| session.generation() == watcher_session);
    let inactive_session_matches =
        watcher_state.inactive_watcher_session(game_id, &game.mod_path, Some(&runtime_config_path))
            == Some(watcher_session);
    let coverage_matches = if active_now {
        active_session_matches || (!background && inactive_session_matches)
    } else {
        inactive_session_matches
    };
    if result.status.applied()
        && (!coverage_matches
            || !disk_reconcile_state.mark_authority_reconciled(
                game_id,
                &game.mod_path,
                watcher_session,
                observed_generation,
                &result,
                &[],
            ))
    {
        disk_reconcile_state.reject_untrusted_reconcile(game_id, result.reconcile_revision);
        return Err(crate::shared::errors::AppError::Io(format!(
            "Disk changed while indexing '{game_id}'; retry to verify it"
        )));
    }
    if result.status.applied() && active_now && !active_session_matches {
        if let Err(error) = crate::modules::workspace::api::start_watcher(
            app.clone(),
            watcher_state,
            pool.clone(),
            game.mod_path.to_string_lossy().into_owned(),
            game_id.to_string(),
        )
        .await
        {
            disk_reconcile_state.reject_untrusted_reconcile(game_id, result.reconcile_revision);
            return Err(error.into());
        }
    }
    if result.status.applied() && active_now {
        let trusted = watcher_state
            .current_session_for_coverage(&game.mod_path, Some(&runtime_config_path))
            .is_some_and(|session| {
                matches!(
                    disk_reconcile_state.authority_catch_up(
                        game_id,
                        &game.mod_path,
                        session.generation(),
                    ),
                    crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp::Clean {
                        reconcile_revision,
                        ..
                    } if reconcile_revision == result.reconcile_revision
                )
            });
        if !trusted {
            disk_reconcile_state.reject_untrusted_reconcile(game_id, result.reconcile_revision);
            return Err(crate::shared::errors::AppError::Io(format!(
                "Watcher handoff for '{game_id}' could not be verified; retry indexing"
            )));
        }
    }
    Ok(result)
}

fn update_resumed_onboarding_background_phase(
    app: &tauri::AppHandle,
    sessions: &crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore,
    session_id: &str,
    game_id: &str,
    phase: crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase,
) {
    match sessions.set_background_phase(session_id, game_id, phase) {
        Ok(status) => {
            if let Err(error) = app.emit("onboarding_indexing:background_status", status) {
                log::debug!("Could not emit resumed onboarding indexing status: {error}");
            }
        }
        Err(error) => log::debug!("Could not update resumed onboarding indexing status: {error}"),
    }
}

async fn finish_resumed_onboarding_game(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    sessions: &crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore,
    session_id: &str,
    game_id: &str,
    result: &Result<
        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
        crate::shared::errors::AppError,
    >,
) {
    use crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase;

    let phase = match result {
        Ok(result) if result.status.applied() => OnboardingIndexingBackgroundPhase::Ready,
        Ok(_) => OnboardingIndexingBackgroundPhase::NeedsAttention,
        Err(_) => OnboardingIndexingBackgroundPhase::Failed,
    };
    update_resumed_onboarding_background_phase(app, sessions, session_id, game_id, phase.clone());
    if !matches!(phase, OnboardingIndexingBackgroundPhase::Ready) {
        return;
    }
    if let Err(error) = crate::modules::reconciliation::application::disk_reconcile::onboarding_recovery::remove_pending_game_id(
        pool,
        game_id,
    )
    .await
    {
        log::warn!("Could not clear completed resumed onboarding game {game_id}: {error}");
    }
}

/// Recovers interrupted work before setup returns; reconciliation and settled
/// task-log retention run in the background.
/// Every step is best-effort and only logs on failure.
/// Boot-time database housekeeping, plus a background reconcile.
///
/// The two recovery writes stay on the setup thread: they are single UPDATEs,
/// and the frontend must not open onto a stale crash-recovery queue. The
/// reconcile does not — it walks the entire mods folder, and `.setup()` has to
/// return before the window appears. It reports through the progress events it
/// already emits, so the UI shows it running instead of showing nothing.
pub fn run_startup_reconcile(app: tauri::AppHandle) {
    use tauri::async_runtime::{block_on, spawn};
    use tauri::Manager;

    let pool = app.state::<sqlx::SqlitePool>().inner().clone();

    block_on(async {
        match initialize_mutation_coordinator(&app).await {
            Ok(recovered) if !recovered.is_empty() => {
                log::warn!(
                    "startup: recovered or isolated {} interrupted mutation(s): {}",
                    recovered.len(),
                    recovered.join(", ")
                );
            }
            Ok(_) => {}
            Err(error) => {
                log::error!("startup: mutation coordinator initialization failed: {error}")
            }
        }

        match crate::modules::workspace::adapters::sqlite::task::reclaim_interrupted_apply_tasks(
            &pool,
        )
        .await
        {
            Ok(reclaimed) if reclaimed > 0 => {
                log::info!("startup: reclaimed {reclaimed} interrupted collection apply task(s)");
            }
            Ok(_) => {}
            Err(error) => {
                log::warn!("startup: interrupted collection apply reclaim failed: {error}");
            }
        }
        recover_interrupted_transfers(&pool).await;
        match crate::modules::ingestion::adapters::sqlite::import_batch::recover_interrupted_batch_states(&pool).await {
            Ok(recovered) if recovered > 0 => {
                log::info!("startup: made {recovered} interrupted import state(s) resumable");
            }
            Ok(_) => {}
            Err(error) => log::warn!("startup: import batch recovery failed: {error}"),
        }
        match app.path().app_data_dir() {
            Ok(app_data) => {
                let staging_root = app_data.join("import-staging");
                match tokio::try_join!(
                    crate::modules::ingestion::adapters::sqlite::import_batch::list_terminal_batch_ids_for_staging_cleanup(&pool),
                    crate::modules::ingestion::adapters::sqlite::import_batch::list_active_staging_paths(&pool),
                ) {
                    Ok((batch_ids, paths)) => {
                        match crate::modules::ingestion::api::import_batch::staging::prepare_startup_staging_cleanup(
                            &staging_root, &batch_ids, &paths,
                        ) {
                            Ok(detached) if detached > 0 => log::info!(
                                "startup: detached {detached} staging payload(s) for deferred cleanup"
                            ),
                            Ok(_) => {},
                            Err(error) => log::warn!("startup: staging detach failed: {error}"),
                        }
                    }
                    Err(error) => log::warn!("startup: staging cleanup lookup failed: {error}"),
                }
            }
            Err(error) => log::warn!("startup: app data path unavailable: {error}"),
        }
    });

    let pending_onboarding_game_ids = block_on(async {
        let pending_game_ids = match crate::modules::reconciliation::application::disk_reconcile::onboarding_recovery::load_pending_game_ids(&pool).await {
            Ok(game_ids) => game_ids,
            Err(error) => {
                log::warn!("startup: could not load pending onboarding indexing games: {error}");
                return Vec::new();
            }
        };
        let configured_game_ids = app
            .state::<crate::modules::settings::application::config::ConfigService>()
            .with_settings(|settings| {
                settings
                    .games
                    .iter()
                    .filter(|game| !game.mod_path.as_os_str().is_empty())
                    .map(|game| game.id.clone())
                    .collect::<std::collections::HashSet<_>>()
            });
        let valid_game_ids = pending_game_ids
            .iter()
            .filter(|game_id| configured_game_ids.contains(*game_id))
            .cloned()
            .collect::<Vec<_>>();
        if valid_game_ids.len() != pending_game_ids.len() {
            if let Err(error) = crate::modules::reconciliation::application::disk_reconcile::onboarding_recovery::replace_pending_game_ids(
                &pool,
                &valid_game_ids,
            )
            .await
            {
                log::warn!("startup: could not remove stale onboarding indexing games: {error}");
            }
        }
        valid_game_ids
    });
    let resumed_onboarding_session_id = if pending_onboarding_game_ids.is_empty() {
        None
    } else {
        let sessions = app.state::<crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore>();
        match sessions.begin_resumed_background(&pending_onboarding_game_ids) {
            Ok(status) => {
                if let Err(error) =
                    app.emit("onboarding_indexing:background_status", status.clone())
                {
                    log::debug!("Could not emit resumed onboarding indexing status: {error}");
                }
                Some(status.session_id)
            }
            Err(error) => {
                log::warn!(
                    "startup: could not initialize resumed onboarding indexing status: {error}"
                );
                None
            }
        }
    };

    let startup_game = app
        .state::<crate::modules::settings::application::config::ConfigService>()
        .with_settings(|settings| settings.active_game().cloned())
        .filter(|game| !game.mod_path.as_os_str().is_empty());
    let startup_recovery_generation = startup_game.as_ref().map(|game| {
        app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>()
            .mark_initial_recovery_pending_for_root(&game.id, &game.mod_path, &game.instance_path.join("d3dx.ini"))
    });

    let telemetry = app
        .state::<crate::modules::system::application::telemetry::TelemetryStore>()
        .capture();
    spawn(async move {
        let config = app.state::<crate::modules::settings::application::config::ConfigService>();
        let watcher_state =
            app.state::<crate::modules::workspace::application::scanner::watcher::WatcherState>();
        let disk_reconcile_state =
            app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>();
        let mutation_coordinator =
            app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
        let sessions = app.state::<crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore>();

        if let Some(game) = startup_game.as_ref() {
            let recovery_generation = startup_recovery_generation
                .expect("startup game and recovery generation are created together");
            if let Some(session_id) = resumed_onboarding_session_id.as_deref() {
                if pending_onboarding_game_ids.contains(&game.id) {
                    update_resumed_onboarding_background_phase(
                        &app,
                        sessions.inner(),
                        session_id,
                        &game.id,
                        crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase::Applying,
                    );
                }
            }
            let reconcile_result = reconcile_startup_game(
                &app,
                &pool,
                config.inner(),
                watcher_state.inner(),
                disk_reconcile_state.inner(),
                mutation_coordinator.inner(),
                StartupRecoveryOwner {
                    game_id: &game.id,
                    generation: Some(recovery_generation),
                },
            )
            .await;
            if config.get_settings().diagnostics.telemetry_enabled {
                let (outcome, error_code) = match &reconcile_result {
                Ok(_) => (
                    crate::modules::system::application::telemetry::TelemetryOutcome::Success,
                    crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                ),
                Err(error) => (
                    crate::modules::system::application::telemetry::TelemetryOutcome::Failed,
                    crate::modules::system::application::telemetry::TelemetryErrorCode::from_app_error(error),
                ),
            };
                let _ = telemetry
                .record_rollup(
                    env!("CARGO_PKG_VERSION"),
                    crate::modules::system::application::telemetry::TelemetryEvent::new(
                        crate::modules::system::application::telemetry::TelemetryOperation::Reconcile,
                        outcome,
                        error_code,
                    ),
                    chrono::Utc::now(),
                )
                .await;
            }
            let recovery_outcome = match &reconcile_result {
            Ok(result) => {
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Completed(Box::new(
                    result.clone(),
                ))
            }
            Err(error) => {
                log::warn!(
                    "Startup Disk Reconcile failed for '{}': {}",
                    game.name,
                    error
                );
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Failed(
                    error.to_string(),
                )
            }
        };
            let accepted = disk_reconcile_state.finish_initial_recovery(
                &game.id,
                recovery_generation,
                recovery_outcome,
            );
            if accepted {
                if let (Ok(result), Some(current_game)) = (
                    &reconcile_result,
                    config
                        .get_settings()
                        .games
                        .into_iter()
                        .find(|current| current.id == game.id),
                ) {
                    disk_reconcile_state.with_initial_recovery_owner(
                        &game.id,
                        recovery_generation,
                        &current_game.mod_path,
                        &current_game.instance_path.join("d3dx.ini"),
                        || {
                            if let Err(error) = app.emit("disk_reconcile:result", result) {
                                log::warn!("Could not emit startup disk reconcile result: {error}");
                            }
                        },
                    );
                }
            }
            if let Some(session_id) = resumed_onboarding_session_id.as_deref() {
                if pending_onboarding_game_ids.contains(&game.id) {
                    let _settlement_guard = disk_reconcile_state.activation_guard().await;
                    let owner_current = accepted
                        && config
                            .get_settings()
                            .games
                            .into_iter()
                            .find(|current| current.id == game.id)
                            .is_some_and(|current| {
                                disk_reconcile_state
                                    .with_initial_recovery_owner(
                                        &game.id,
                                        recovery_generation,
                                        &current.mod_path,
                                        &current.instance_path.join("d3dx.ini"),
                                        || (),
                                    )
                                    .is_some()
                            });
                    let rejected_result = Err(crate::shared::errors::AppError::Cancelled);
                    finish_resumed_onboarding_game(
                        &app,
                        &pool,
                        sessions.inner(),
                        session_id,
                        &game.id,
                        if owner_current {
                            &reconcile_result
                        } else {
                            &rejected_result
                        },
                    )
                    .await;
                }
            }
        }

        // Retention must not compete with the foreground baseline or hide open recovery tasks.
        match crate::modules::workspace::adapters::sqlite::task::purge_old_tasks(&pool).await {
            Ok(purged) => {
                log::info!("background: task log cleanup completed purged={purged}");
            }
            Err(error) => {
                log::warn!("background: task log cleanup failed: {error}");
            }
        }

        let Some(session_id) = resumed_onboarding_session_id.as_deref() else {
            return;
        };
        for game_id in pending_onboarding_game_ids {
            if startup_game
                .as_ref()
                .is_some_and(|startup_game| startup_game.id == game_id)
            {
                continue;
            }
            update_resumed_onboarding_background_phase(
                &app,
                sessions.inner(),
                session_id,
                &game_id,
                crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase::Applying,
            );
            let reconcile_result = reconcile_startup_game(
                &app,
                &pool,
                config.inner(),
                watcher_state.inner(),
                disk_reconcile_state.inner(),
                mutation_coordinator.inner(),
                StartupRecoveryOwner {
                    game_id: &game_id,
                    generation: None,
                },
            )
            .await;
            finish_resumed_onboarding_game(
                &app,
                &pool,
                sessions.inner(),
                session_id,
                &game_id,
                &reconcile_result,
            )
            .await;
            if let Err(error) = reconcile_result {
                log::warn!("startup: resumed onboarding indexing failed for {game_id}: {error}");
            }
        }
    });
}

/// Recursive deletion starts only after setup yields to the native event loop.
pub fn run_deferred_staging_cleanup(app: tauri::AppHandle) {
    let staging_root = match app.path().app_data_dir() {
        Ok(path) => path.join("import-staging"),
        Err(error) => {
            log::warn!("ready: staging cleanup path unavailable: {error}");
            return;
        }
    };
    tauri::async_runtime::spawn(async move {
        match tauri::async_runtime::spawn_blocking(move || {
            crate::modules::ingestion::api::import_batch::staging::cleanup_deferred_staging(
                &staging_root,
            )
        })
        .await
        {
            Ok(Ok(removed)) if removed > 0 => {
                log::info!("ready: removed {removed} detached staging payload(s)")
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => log::warn!("ready: deferred staging cleanup failed: {error}"),
            Err(error) => log::error!("ready: deferred staging cleanup worker failed: {error}"),
        }
    });
}
