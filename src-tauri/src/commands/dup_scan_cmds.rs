use crate::services::scanner::dedup_scanner::{self, DedupScanStatus};
use crate::services::scanner::walker;
use crate::types::dup_scan::{DupScanEvent, DupScanReport};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::ipc::Channel;
use tauri::{State, Window};

pub struct DupScanState {
    is_running: Arc<AtomicBool>,
    cancel_flag: Arc<AtomicBool>,
    last_report: Arc<Mutex<Option<DupScanReport>>>,
}

impl DupScanState {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            last_report: Arc::new(Mutex::new(None)),
        }
    }

    pub fn try_start(&self) -> Result<(), String> {
        self.is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "Duplicate scan already running".to_string())
            .map(|_| ())
    }

    pub fn reset_cancel(&self) {
        self.cancel_flag.store(false, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel_flag)
    }

    pub fn running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.is_running)
    }

    pub fn report_store(&self) -> Arc<Mutex<Option<DupScanReport>>> {
        Arc::clone(&self.last_report)
    }

    pub fn load_report(&self) -> Option<DupScanReport> {
        self.report_store()
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }
}

impl Default for DupScanState {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
pub async fn dup_scan_start(
    game_id: String,
    mods_root: String,
    _window: Window,
    state: State<'_, DupScanState>,
    db: State<'_, sqlx::SqlitePool>,
    on_event: Channel<DupScanEvent>,
) -> Result<(), String> {
    let mods_path = Path::new(&mods_root);
    if !mods_path.exists() {
        return Err(format!("Mods path does not exist: {mods_root}"));
    }
    if !mods_path.is_dir() {
        return Err(format!("Mods path is not a directory: {mods_root}"));
    }

    state.try_start()?;
    state.reset_cancel();

    let scan_id = dup_scan_build_scan_id();
    let cancel_flag = state.cancel_flag();
    let running_flag = state.running_flag();
    let report_store = state.report_store();
    let mods_root_for_task = mods_root.clone();
    let game_id_for_task = game_id.clone();
    let db_for_task = db.inner().clone();

    tokio::spawn(async move {
        let _running_guard = RunningGuard::new(running_flag);

        let candidates = match walker::scan_mod_folders(Path::new(&mods_root_for_task)) {
            Ok(items) => items,
            Err(error) => {
                let _ = on_event.send(DupScanEvent::Cancelled {
                    scan_id,
                    processed_folders: 0,
                    total_folders: 0,
                });
                log::warn!("Failed to enumerate mods for duplicate scan: {error}");
                return;
            }
        };

        let total_folders = candidates.len();
        let _ = on_event.send(DupScanEvent::Started {
            scan_id: scan_id.clone(),
            game_id: game_id_for_task.clone(),
            total_folders,
        });

        let mods_root_for_service = mods_root_for_task.clone();
        let cancel_for_service = Arc::clone(&cancel_flag);
        let game_id_for_service = game_id_for_task.clone();
        let service_future = tokio::spawn(async move {
            dedup_scanner::scan_duplicates(
                Path::new(&mods_root_for_service),
                &game_id_for_service,
                &db_for_task,
                cancel_for_service,
            )
            .await
        });

        let mut current = 0usize;
        while !service_future.is_finished() {
            if total_folders > 0 && current < total_folders.saturating_sub(1) {
                current += 1;
                let percent = ((current * 100) / total_folders).min(100) as u8;
                let _ = on_event.send(DupScanEvent::Progress {
                    scan_id: scan_id.clone(),
                    processed_folders: current,
                    total_folders,
                    current_folder: format!("Hashing {current}/{total_folders}"),
                    percent,
                });
            }

            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        let outcome = match service_future.await {
            Ok(Ok(data)) => data,
            Ok(Err(error)) => {
                let _ = on_event.send(DupScanEvent::Cancelled {
                    scan_id,
                    processed_folders: current,
                    total_folders,
                });
                log::warn!("Duplicate scan failed: {error}");
                return;
            }
            Err(error) => {
                let _ = on_event.send(DupScanEvent::Cancelled {
                    scan_id,
                    processed_folders: current,
                    total_folders,
                });
                log::warn!("Duplicate scan task join failed: {error}");
                return;
            }
        };

        match outcome.status {
            DedupScanStatus::Cancelled => {
                let processed = current.min(outcome.total_folders);
                let _ = on_event.send(DupScanEvent::Cancelled {
                    scan_id,
                    processed_folders: processed,
                    total_folders: outcome.total_folders,
                });
            }
            DedupScanStatus::Completed => {
                let final_total = outcome.total_folders;
                let final_current = final_total;
                let final_percent = if final_total == 0 {
                    100
                } else {
                    ((final_current * 100) / final_total).min(100) as u8
                };

                let _ = on_event.send(DupScanEvent::Progress {
                    scan_id: scan_id.clone(),
                    processed_folders: final_current,
                    total_folders: final_total,
                    current_folder: format!("Hashing {final_current}/{final_total}"),
                    percent: final_percent,
                });

                for group in &outcome.groups {
                    let _ = on_event.send(DupScanEvent::Match {
                        scan_id: scan_id.clone(),
                        group: group.clone(),
                    });
                }

                let total_members = outcome.groups.iter().map(|group| group.members.len()).sum();
                let report = DupScanReport {
                    scan_id: scan_id.clone(),
                    game_id: game_id_for_task,
                    root_path: mods_root_for_task,
                    total_groups: outcome.groups.len(),
                    total_members,
                    groups: outcome.groups,
                };

                if let Ok(mut guard) = report_store.lock() {
                    *guard = Some(report.clone());
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
pub async fn dup_scan_cancel(state: State<'_, DupScanState>) -> Result<(), String> {
    state.cancel();
    Ok(())
}

#[tauri::command]
pub async fn dup_scan_get_report(state: State<'_, DupScanState>) -> Result<Option<DupScanReport>, String> {
    Ok(state.load_report())
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
