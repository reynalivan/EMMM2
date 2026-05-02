use std::sync::atomic::{AtomicBool, Ordering};
use tauri::State;

pub struct ScanState {
    pub is_cancelled: AtomicBool,
}

impl ScanState {
    pub fn new() -> Self {
        Self {
            is_cancelled: AtomicBool::new(false),
        }
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }
}

impl Default for ScanState {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_scan_cmd(state: State<'_, ScanState>) -> Result<(), String> {
    state.cancel();
    Ok(())
}
