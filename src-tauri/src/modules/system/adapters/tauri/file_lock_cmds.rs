use crate::{modules::system::api::FileLockReport, shared::errors::AppError};

#[tauri::command]
#[specta::specta]
pub async fn get_file_lock_info(recovery_id: String) -> Result<FileLockReport, AppError> {
    ensure_recovery_allowed()?;
    tokio::task::spawn_blocking(move || {
        crate::modules::system::api::file_lock::get_file_lock_info(&recovery_id)
    })
    .await
    .map_err(|_| AppError::Internal("File-lock discovery worker failed".into()))?
}

#[tauri::command]
#[specta::specta]
pub async fn control_file_lock_process(
    recovery_id: String,
    process_ref: String,
    force: bool,
) -> Result<FileLockReport, AppError> {
    ensure_recovery_allowed()?;
    tokio::task::spawn_blocking(move || {
        crate::modules::system::api::file_lock::control_file_lock_process(
            &recovery_id,
            &process_ref,
            force,
        )
    })
    .await
    .map_err(|_| AppError::Internal("Process-control worker failed".into()))?
}

#[tauri::command]
#[specta::specta]
pub async fn dismiss_file_lock_recovery(recovery_id: String) -> Result<(), AppError> {
    crate::modules::system::api::file_lock::dismiss_file_lock_recovery(&recovery_id)
}

fn ensure_recovery_allowed() -> Result<(), AppError> {
    if crate::modules::system::application::app::readonly_audit::is_active() {
        return Err(AppError::Validation(
            "File-lock recovery is unavailable during read-only audit".into(),
        ));
    }
    Ok(())
}
