use crate::modules::games::domain::models::ConfigStatus;
use crate::shared::errors::AppError;
use serde::Serialize;

#[derive(Serialize, specta::Type)]
pub struct PendingCrashReportSummary {
    pub error_code: String,
}

/// Records an IPC failure after the shared frontend command boundary has
/// reduced it to bounded enums. Raw error text is never accepted here.
#[specta::specta]
#[tauri::command]
pub async fn record_native_error_metric(
    operation: String,
    error_code: String,
    config: tauri::State<'_, crate::modules::settings::application::config::ConfigService>,
    telemetry: tauri::State<'_, crate::modules::system::application::telemetry::TelemetryStore>,
) -> Result<(), AppError> {
    if !config.get_settings().diagnostics.telemetry_enabled {
        return Ok(());
    }
    let event = crate::modules::system::application::telemetry::TelemetryEvent::new(
        crate::modules::system::application::telemetry::TelemetryOperation::from_label(&operation),
        crate::modules::system::application::telemetry::TelemetryOutcome::Failed,
        crate::modules::system::application::telemetry::TelemetryErrorCode::from_label(&error_code),
    );
    let _ = telemetry
        .record_rollup(env!("CARGO_PKG_VERSION"), event, chrono::Utc::now())
        .await;
    Ok(())
}

/// Returns a single privacy-safe indication that the last session did not
/// close normally. The underlying report never contains the panic message.
#[specta::specta]
#[tauri::command]
pub async fn get_pending_crash_report(
    telemetry: tauri::State<'_, crate::modules::system::application::telemetry::TelemetryStore>,
) -> Result<Option<PendingCrashReportSummary>, AppError> {
    telemetry
        .pending_crash()
        .await
        .map(|report| {
            report.map(|report| PendingCrashReportSummary {
                error_code: report.fingerprint.error_code.as_str().to_string(),
            })
        })
        .map_err(|_| AppError::Db("Could not read the local crash report".to_string()))
}

/// Discard the local abnormal-exit marker without uploading it.
#[specta::specta]
#[tauri::command]
pub async fn discard_pending_crash_report(
    telemetry: tauri::State<'_, crate::modules::system::application::telemetry::TelemetryStore>,
) -> Result<(), AppError> {
    telemetry
        .clear_pending_crash()
        .await
        .map_err(|_| AppError::Db("Could not discard the local crash report".to_string()))
}

/// Exit the application successfully.
#[specta::specta]
#[tauri::command]
pub fn exit_app(app: tauri::AppHandle) -> Result<(), AppError> {
    app.exit(0);
    Ok(())
}

/// Check if the app has any games configured (determines which screen to show on startup).
#[specta::specta]
#[tauri::command]
pub async fn check_config_status(
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<ConfigStatus, AppError> {
    crate::modules::system::application::app::app_service::check_config_status(pool.inner()).await
}

/// Read the last N lines of the application log.
#[specta::specta]
#[tauri::command]
pub async fn get_logs(
    app: tauri::AppHandle,
    limit: Option<usize>,
    count: Option<usize>,
) -> Result<Vec<String>, AppError> {
    use tauri::Manager;
    let log_dir = app.path().app_log_dir()?;
    let log_path = log_dir.join("emmm.log");

    let lines = limit.or(count).unwrap_or(200);
    crate::modules::system::application::app::log_service::read_last_n_lines(&log_path, lines)
}

/// Open the logs directory in the OS file explorer.
#[specta::specta]
#[tauri::command]
pub async fn open_log_folder(app: tauri::AppHandle) -> Result<(), AppError> {
    use tauri::Manager;
    let log_dir = app.path().app_log_dir()?;

    crate::modules::system::application::app::log_service::open_log_folder_service(&log_dir)
}

/// Reset the application setup by clearing all data from the database.
/// Before clearing, a backup copy of `app.db` is saved to the trash folder.
/// No mod files or folders on disk are deleted — only database records are cleared.
#[specta::specta]
#[tauri::command]
pub async fn reset_database(
    app: tauri::AppHandle,
    config: tauri::State<'_, crate::modules::settings::application::config::ConfigService>,
    credentials: tauri::State<'_, crate::platform::security::credential_store::CredentialStore>,
    telemetry: tauri::State<'_, crate::modules::system::application::telemetry::TelemetryStore>,
) -> Result<(), AppError> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir()?;

    telemetry
        .purge_all()
        .await
        .map_err(|_| AppError::Db("Could not clear queued diagnostics during reset".to_string()))?;
    config.reset_database(&app_data_dir)?;
    credentials.delete_ai_api_key()?;
    config.set_ai_key_status(false);
    Ok(())
}

/// Check if a given absolute path exists on the disk.
/// Bypasses restrictive Tauri v2 plugin-fs scopes.
#[specta::specta]
#[tauri::command]
pub fn check_path_exists_cmd(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

#[cfg(test)]
#[path = "tests/app_cmds_tests.rs"]
mod tests;
