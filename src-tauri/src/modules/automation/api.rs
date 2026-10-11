pub use super::application::*;

pub(crate) fn update_hotkey_notice_runtime(
    app: &tauri::AppHandle,
    status: &crate::modules::reconciliation::api::disk_reconcile::types::RuntimeSyncStatus,
) {
    super::adapters::tauri::hotkey_notice::update_runtime_status(app, status);
}

#[cfg(debug_assertions)]
pub mod testing {
    pub mod adapters {
        pub use super::super::super::adapters::*;
    }
    pub mod application {
        pub use super::super::super::application::*;
    }
}
