use crate::shared::errors::AppError;
use crate::platform::fs::operation_lock::OperationLock;
use crate::platform::fs::guard;
use crate::modules::workspace::application::scanner::core::walker;
use crate::modules::workspace::application::scanner::dedup::resolver::{
    ResolutionProgress, ResolutionRequest, ResolutionSummary,
};
use crate::modules::workspace::application::scanner::dedup::scanner::DedupScanStatus;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::modules::workspace::application::disk_reconcile::emit;
use crate::modules::system::application::config::ConfigService;
use crate::modules::storage_optimizer::adapters::outbound::sqlite::dedup;
use crate::types::dup_scan::{DupScanEvent, DupScanReport};
use crate::modules::workspace::domain::conflicts::WhitelistEntry;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter};

pub struct DupScanState {
    pub(crate) is_running: Arc<AtomicBool>,
    pub(crate) cancel_flag: Arc<AtomicBool>,
}

impl DupScanState {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn try_start(&self) -> Result<(), AppError> {
        self.is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| AppError::Validation("Duplicate scan already running".to_string()))
            .map(|_| ())
    }

    pub fn reset_cancel(&self) {
        self.cancel_flag.store(false, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
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
    game_id: String,
    mods_root: String,
    state: State<'_, DupScanState>,
    db: State<'_, sqlx::SqlitePool>,
    on_event: Channel<DupScanEvent>,
) -> Result<(), AppError> {
    let mods_path = Path::new(&mods_root);
    if !mods_path.exists() {
        return Err(AppError::NotFound(format!(
            "Mods path does not exist: {mods_root}"
        )));
    }
    if !mods_path.is_dir() {
        return Err(AppError::Validation(format!(
            "Mods path is not a directory: {mods_root}"
        )));
    }

    state.try_start()?;
    state.reset_cancel();

    let scan_id = dup_scan_build_scan_id();
    let cancel_flag = Arc::clone(&state.cancel_flag);
    let running_flag = Arc::clone(&state.is_running);
    let mods_root_for_task = mods_root.clone();
    let game_id_for_task = game_id.clone();
    let db_for_task = db.inner().clone();

    tokio::spawn(async move {
        let _running_guard = RunningGuard::new(running_flag);

        let candidates = match walker::scan_mod_folders(Path::new(&mods_root_for_task)) {
            Ok(items) => items,
            Err(error) => {
                let message = error.to_string();
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

        let outcome = match crate::modules::workspace::application::scanner::dedup::scanner::scan_duplicates(
            Path::new(&mods_root_for_task),
            &game_id_for_task,
            &db_for_task,
            Arc::clone(&cancel_flag),
        )
        .await
        {
            Ok(data) => data,
            Err(error) => {
                let message = error.to_string();
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
                let _ = on_event.send(DupScanEvent::Cancelled {
                    scan_id,
                    processed_folders: 0,
                    total_folders: outcome.total_folders,
                });
            }
            DedupScanStatus::Completed => {
                let final_total = outcome.total_folders;

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
                    root_path: mods_root_for_task,
                    total_groups: groups.len(),
                    total_members,
                    groups,
                };

                if let Err(error) = dedup::persist_completed_report(&db_for_task, &report).await {
                    let _ = on_event.send(DupScanEvent::Failed {
                        scan_id,
                        processed_folders: final_total,
                        total_folders: final_total,
                        message: error.to_string(),
                    });
                    return;
                }

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
    state.cancel();
    Ok(())
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
    running_flag: Arc<AtomicBool>,
}

impl RunningGuard {
    fn new(running_flag: Arc<AtomicBool>) -> Self {
        Self { running_flag }
    }
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.running_flag.store(false, Ordering::SeqCst);
    }
}

#[tauri::command]
#[specta::specta]
pub async fn dup_resolve_batch(
    app: AppHandle,
    requests: Vec<ResolutionRequest>,
    game_id: String,
    watcher_state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    config: State<'_, ConfigService>,
    db: State<'_, sqlx::SqlitePool>,
) -> Result<ResolutionSummary, AppError> {
    let all_folders: Vec<String> = requests
        .iter()
        .flat_map(|request| [request.folder_a.clone(), request.folder_b.clone()])
        .collect();
    guard::validate_paths(&config, &game_id, &all_folders)?;
    emit::ensure_mutation_preflight_for_paths(&app, db.inner(), &game_id, Some(&all_folders)).await?;

    let op_guard = op_lock.acquire().await?;
    let result = crate::modules::workspace::application::scanner::dedup::resolver::resolve_batch(
        requests,
        game_id.clone(),
        db.inner(),
        &op_guard,
        &watcher_state.suppressor,
        |progress: ResolutionProgress| {
            let _ = app.emit("dup-resolve-progress", &progress);
        },
    )
    .await?;
    drop(op_guard);
    emit::run_full_internal_disk_reconcile(&app, db.inner(), &game_id).await?;
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
