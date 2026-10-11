use crate::modules::duplicates::adapters::sqlite::dedup;
use crate::modules::duplicates::application::dedup::resolver::{
    ResolutionProgress, ResolutionRequest, ResolutionSummary,
};
use crate::modules::duplicates::application::dedup::scanner::DedupScanStatus;
use crate::modules::duplicates::domain::dup_scan::{DupScanEvent, DupScanReport};
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::reconciliation::application::disk_reconcile::emit;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::core::walker;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::modules::workspace::domain::conflicts::WhitelistEntry;
use crate::platform::fs::guard;
use crate::shared::errors::AppError;
use crate::shared::sync::lock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::ipc::Channel;
use tauri::State;
use tauri::{AppHandle, Emitter, Manager};

pub struct DupScanState {
    active: Arc<Mutex<Option<ActiveScan>>>,
}

struct ActiveScan {
    cancel_flag: Arc<AtomicBool>,
    committing: bool,
}

impl DupScanState {
    pub fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(None)),
        }
    }

    fn try_start(&self) -> Result<RunningGuard, AppError> {
        let mut active = lock(&self.active);
        if active.is_some() {
            return Err(AppError::Validation(
                "Duplicate scan already running".to_string(),
            ));
        }
        let cancel_flag = Arc::new(AtomicBool::new(false));
        *active = Some(ActiveScan {
            cancel_flag: Arc::clone(&cancel_flag),
            committing: false,
        });
        Ok(RunningGuard {
            active: Arc::clone(&self.active),
            cancel_flag,
        })
    }

    fn cancel(&self) -> Result<(), AppError> {
        let active = lock(&self.active);
        if let Some(scan) = active.as_ref() {
            if scan.committing {
                return Err(AppError::Validation(
                    "Duplicate scan is saving its report; wait for it to finish".to_string(),
                ));
            }
            scan.cancel_flag.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
}

impl Default for DupScanState {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
#[specta::specta]
pub async fn dup_scan_start(
    app: AppHandle,
    game_id: String,
    mods_root: String,
    state: State<'_, DupScanState>,
    db: State<'_, sqlx::SqlitePool>,
    on_event: Channel<DupScanEvent>,
) -> Result<(), AppError> {
    let config = app.state::<ConfigService>();
    let mods_root = guard::validate_mods_root(&config, &game_id, &mods_root)?;
    if !mods_root.is_dir() {
        return Err(AppError::Validation(format!(
            "Mods path is not a directory: {}",
            mods_root.display()
        )));
    }

    let running_guard = state.try_start()?;

    let scan_id = dup_scan_build_scan_id();
    let cancel_flag = running_guard.cancel_flag();
    let mods_root_for_task = mods_root.clone();
    let game_id_for_task = game_id.clone();
    let db_for_task = db.inner().clone();
    let telemetry =
        crate::modules::system::application::telemetry::capture_background_telemetry(&app);

    tokio::spawn(async move {
        let candidates = match walker::scan_mod_folders(&mods_root_for_task) {
            Ok(items) => items,
            Err(error) => {
                crate::modules::system::application::telemetry::record_background_failure(
                    &telemetry,
                    &AppError::Scanner(error.clone()),
                )
                .await;
                let message = error.to_string();
                drop(running_guard);
                let _ = on_event.send(DupScanEvent::Failed {
                    scan_id,
                    processed_folders: 0,
                    total_folders: 0,
                    message: message.clone(),
                });
                log::warn!("Failed to enumerate mods for duplicate scan: {message}");
                return;
            }
        };

        let total_folders = candidates.len();
        let _ = on_event.send(DupScanEvent::Started {
            scan_id: scan_id.clone(),
            game_id: game_id_for_task.clone(),
            total_folders,
        });

        let outcome =
            match crate::modules::duplicates::application::dedup::scanner::scan_duplicates_for_candidates(
                &mods_root_for_task,
                candidates,
                &game_id_for_task,
                &db_for_task,
                Arc::clone(&cancel_flag),
            )
            .await
            {
                Ok(data) => data,
                Err(error) => {
                    crate::modules::system::application::telemetry::record_background_failure(
                        &telemetry,
                        &AppError::Scanner(error.clone()),
                    )
                    .await;
                    let message = error.to_string();
                    drop(running_guard);
                    let _ = on_event.send(DupScanEvent::Failed {
                        scan_id,
                        processed_folders: 0,
                        total_folders,
                        message: message.clone(),
                    });
                    log::warn!("Duplicate scan failed: {message}");
                    return;
                }
            };

        match outcome.status {
            DedupScanStatus::Cancelled => {
                drop(running_guard);
                let _ = on_event.send(DupScanEvent::Cancelled {
                    scan_id,
                    processed_folders: 0,
                    total_folders: outcome.total_folders,
                });
            }
            DedupScanStatus::Completed => {
                let final_total = outcome.total_folders;

                if !running_guard.begin_commit() {
                    drop(running_guard);
                    let _ = on_event.send(DupScanEvent::Cancelled {
                        scan_id,
                        processed_folders: final_total,
                        total_folders: final_total,
                    });
                    return;
                }

                let _ = on_event.send(DupScanEvent::Progress {
                    scan_id: scan_id.clone(),
                    processed_folders: final_total,
                    total_folders: final_total,
                    current_folder: format!("Hashing {final_total}/{final_total}"),
                    percent: 100,
                });

                let mut groups = outcome.groups;
                for group in &mut groups {
                    group.group_id = format!("{}:{}", scan_id, group.group_id);
                }
                for group in &groups {
                    let _ = on_event.send(DupScanEvent::Match {
                        scan_id: scan_id.clone(),
                        group: group.clone(),
                    });
                }

                let total_members = groups.iter().map(|group| group.members.len()).sum();
                let report = DupScanReport {
                    scan_id: scan_id.clone(),
                    game_id: game_id_for_task,
                    root_path: mods_root_for_task.to_string_lossy().into_owned(),
                    total_groups: groups.len(),
                    total_members,
                    groups,
                };

                if let Err(error) = dedup::persist_completed_report(&db_for_task, &report).await {
                    drop(running_guard);
                    let _ = on_event.send(DupScanEvent::Failed {
                        scan_id,
                        processed_folders: final_total,
                        total_folders: final_total,
                        message: error.to_string(),
                    });
                    return;
                }

                drop(running_guard);
                let _ = on_event.send(DupScanEvent::Finished {
                    scan_id,
                    total_groups: report.total_groups,
                    total_members: report.total_members,
                });
            }
        }
    });

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn dup_scan_cancel(state: State<'_, DupScanState>) -> Result<(), AppError> {
    state.cancel()
}

#[tauri::command]
#[specta::specta]
pub async fn dup_scan_get_report(
    game_id: String,
    db: State<'_, sqlx::SqlitePool>,
) -> Result<Option<DupScanReport>, AppError> {
    Ok(dedup::load_latest_completed_report(db.inner(), &game_id).await?)
}

fn dup_scan_build_scan_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);

    format!("dup_scan_{millis}")
}

struct RunningGuard {
    active: Arc<Mutex<Option<ActiveScan>>>,
    cancel_flag: Arc<AtomicBool>,
}

impl RunningGuard {
    fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel_flag)
    }

    // The same lock orders cancellation against the report's commit boundary.
    fn begin_commit(&self) -> bool {
        let mut active = lock(&self.active);
        let Some(scan) = active.as_mut() else {
            return false;
        };
        if !Arc::ptr_eq(&scan.cancel_flag, &self.cancel_flag)
            || self.cancel_flag.load(Ordering::SeqCst)
        {
            return false;
        }
        scan.committing = true;
        true
    }
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        let mut active = lock(&self.active);
        if active
            .as_ref()
            .is_some_and(|scan| Arc::ptr_eq(&scan.cancel_flag, &self.cancel_flag))
        {
            *active = None;
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn dup_resolve_batch(
    app: AppHandle,
    requests: Vec<ResolutionRequest>,
    game_id: String,
    watcher_state: State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    db: State<'_, sqlx::SqlitePool>,
) -> Result<ResolutionSummary, AppError> {
    let config = app.state::<ConfigService>();
    let all_folders: Vec<String> = requests
        .iter()
        .flat_map(|request| [request.folder_a.clone(), request.folder_b.clone()])
        .collect();
    guard::validate_paths(&config, &game_id, &all_folders)?;
    emit::ensure_mutation_preflight_for_paths(&app, db.inner(), &game_id, Some(&all_folders))
        .await?;

    let game_guard = disk_reconcile.game_lock(&game_id).lock_owned().await;
    disk_reconcile.ensure_core_ready_for_mutation(&game_id)?;
    let prepared = crate::modules::duplicates::application::dedup::resolver::prepare_durable_batch(
        requests,
        &game_id,
        db.inner(),
    )
    .await?;
    let Some(operation_plan) = prepared.operation_plan(&game_id) else {
        drop(game_guard);
        let op_guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::DuplicateIgnoreRules,
            )
            .await?;
        return crate::modules::duplicates::application::dedup::resolver::resolve_batch(
            prepared.requests(),
            game_id,
            db.inner(),
            op_guard.op_guard(),
            &watcher_state.suppressor,
            |progress: ResolutionProgress| {
                let _ = app.emit("dup-resolve-progress", &progress);
            },
        )
        .await;
    };
    let operation_guard = op_lock.acquire_operation(operation_plan).await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_ready_durable_guard(
        disk_reconcile.inner(),
        &game_id,
        game_guard,
        operation_guard,
    )?;
    let result = crate::modules::duplicates::application::dedup::resolver::resolve_durable_batch(
        &prepared,
        &game_id,
        db.inner(),
        &mutation_lease,
        &watcher_state.suppressor,
        |progress: ResolutionProgress| {
            let _ = app.emit("dup-resolve-progress", &progress);
        },
    )
    .await?;
    emit::run_full_internal_disk_reconcile_under_lease(&app, db.inner(), &game_id, &mutation_lease)
        .await?;
    mutation_lease.mark_db_committed()?;
    mutation_lease.commit()?;
    prepared.finalize();
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn get_ignored_pairs(
    game_id: String,
    db: State<'_, sqlx::SqlitePool>,
) -> Result<Vec<WhitelistEntry>, AppError> {
    dedup::get_whitelist_detailed(db.inner(), &game_id)
        .await
        .map_err(|e| AppError::Db(format!("Failed to fetch ignored pairs: {}", e)))
}

#[tauri::command]
#[specta::specta]
pub async fn remove_ignored_pair(
    entry_id: String,
    db: State<'_, sqlx::SqlitePool>,
) -> Result<u64, AppError> {
    dedup::delete_whitelist_entry(db.inner(), &entry_id)
        .await
        .map_err(|e| AppError::Db(format!("Failed to remove ignored pair: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::DupScanState;
    use crate::modules::settings::application::config::{ConfigService, GameConfig};
    use crate::platform::fs::guard::validate_mods_root;
    use crate::shared::errors::AppError;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn accepted_cancellation_prevents_report_commit() {
        let state = DupScanState::new();
        let scan = state.try_start().unwrap();
        state.cancel().unwrap();
        assert!(!scan.begin_commit());
    }

    #[test]
    fn cancellation_is_rejected_after_report_commit_begins() {
        let state = DupScanState::new();
        let scan = state.try_start().unwrap();
        assert!(scan.begin_commit());
        assert!(state.cancel().is_err());
        assert!(state.try_start().is_err());
        drop(scan);
        assert!(state.try_start().is_ok());
    }

    #[test]
    fn a_new_scan_does_not_inherit_the_previous_cancellation_token() {
        let state = DupScanState::new();
        let first = state.try_start().unwrap();
        let previous_token = first.cancel_flag();
        state.cancel().unwrap();
        drop(first);
        let next = state.try_start().unwrap();
        assert!(!next.cancel_flag().load(std::sync::atomic::Ordering::SeqCst));
        assert!(previous_token.load(std::sync::atomic::Ordering::SeqCst));
        assert!(next.begin_commit());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn scan_root_must_match_the_configured_game_root() {
        let temp = TempDir::new().unwrap();
        let mods_root = temp.path().join("Mods");
        let child = mods_root.join("Character");
        fs::create_dir_all(&child).unwrap();
        let pool = crate::test_utils::init_test_db().await.pool;
        let config = ConfigService::new_for_test_async(pool).await;
        let mut settings = config.get_settings();
        settings.games.push(GameConfig {
            id: "game-1".to_string(),
            name: "Test Game".to_string(),
            game_type: crate::modules::games::domain::models::GameType::Gimi,
            instance_path: temp.path().join("Game"),
            mod_path: mods_root,
            ready_to_move_path: None,
            launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
            game_exe: None,
            loader_exe: None,
            xxmi_launcher_exe: None,
            launch_args: None,
            warnings: Vec::new(),
        });
        config.save_settings(settings).unwrap();

        let error = validate_mods_root(&config, "game-1", &child.to_string_lossy())
            .expect_err("a scanner cannot run from a child of the configured Mods root");
        assert!(matches!(error, AppError::Security(_)));
    }
}
