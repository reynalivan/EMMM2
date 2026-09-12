use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use tauri::{AppHandle, State};

use crate::modules::browser::application::browser::{
    browser_service, download_handler, download_service,
};

// ── Browser Tab ──────────────────────────────────────────────────────────────

/// Open a new in-app browser tab (creates a new Webview).
/// Returns the webview label so the frontend can track the tab.
#[tauri::command]
#[specta::specta]
pub async fn browser_open_tab(
    url: String,
    session_id: Option<String>,
    app: AppHandle,
    db: State<'_, SqlitePool>,
) -> Result<String, AppError> {
    Ok(browser_service::open_tab(app, db.inner().clone(), url, session_id).await?)
}

/// Navigate an existing browser tab to a new URL.
#[tauri::command]
#[specta::specta]
pub async fn browser_navigate(label: String, url: String, app: AppHandle) -> Result<(), AppError> {
    Ok(browser_service::navigate(app, &label, url).await?)
}

/// Navigate an existing browser tab back in history.
#[tauri::command]
#[specta::specta]
pub async fn browser_go_back(label: String, app: AppHandle) -> Result<(), AppError> {
    Ok(browser_service::go_back(app, &label).await?)
}

/// Navigate an existing browser tab forward in history.
#[tauri::command]
#[specta::specta]
pub async fn browser_go_forward(label: String, app: AppHandle) -> Result<(), AppError> {
    Ok(browser_service::go_forward(app, &label).await?)
}

/// Reload an existing browser tab.
#[tauri::command]
#[specta::specta]
pub async fn browser_reload_tab(label: String, app: AppHandle) -> Result<(), AppError> {
    Ok(browser_service::reload_tab(app, &label).await?)
}

/// Clear cookies and cache for a specific browser tab.
#[tauri::command]
#[specta::specta]
pub async fn browser_clear_data(label: String, app: AppHandle) -> Result<(), AppError> {
    Ok(browser_service::clear_data(app, &label).await?)
}

/// Get the configured browser homepage URL.
#[tauri::command]
#[specta::specta]
pub async fn browser_get_homepage(db: State<'_, SqlitePool>) -> Result<String, AppError> {
    Ok(browser_service::get_homepage(db.inner()).await)
}

/// Set a new browser homepage URL. Validates http/https scheme.
#[tauri::command]
#[specta::specta]
pub async fn browser_set_homepage(url: String, db: State<'_, SqlitePool>) -> Result<(), AppError> {
    Ok(browser_service::set_homepage(db.inner(), &url).await?)
}

/// Get the number of days terminal downloads stay in history.
#[tauri::command]
#[specta::specta]
pub async fn browser_get_retention_days(
    legacy_retention_days: Option<i64>,
    db: State<'_, SqlitePool>,
) -> Result<i64, AppError> {
    Ok(browser_service::get_or_migrate_retention_days(db.inner(), legacy_retention_days).await?)
}

/// Set the number of days terminal downloads stay in history.
#[tauri::command]
#[specta::specta]
pub async fn browser_set_retention_days(
    days: i64,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(browser_service::set_retention_days(db.inner(), days).await?)
}

// ── Download Manager ─────────────────────────────────────────────────────────

/// Return all browser downloads ordered by most recent first.
#[tauri::command]
#[specta::specta]
pub async fn browser_list_downloads(
    db: State<'_, SqlitePool>,
) -> Result<Vec<download_service::BrowserDownloadDto>, AppError> {
    Ok(download_service::list_downloads(db.inner()).await?)
}

/// Cancel (and optionally delete the file for) a specific download.
///
/// An in-flight transfer is aborted by its background task, which also
/// deletes the partial file and marks the record `canceled`. Otherwise
/// (stale/finished record) only the DB status is updated here.
#[tauri::command]
#[specta::specta]
pub async fn browser_cancel_download(
    id: String,
    delete_file: Option<bool>,
    app: AppHandle,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(download_service::cancel_download_with_feedback(db.inner(), &app, &id, delete_file).await?)
}

/// Start a download only after the user accepts its short-lived confirmation.
#[tauri::command]
#[specta::specta]
pub async fn browser_confirm_download(
    request_id: String,
    app: AppHandle,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(download_handler::confirm_download(app, db.inner().clone(), &request_id).await?)
}

/// Reject a pending download confirmation without creating a file or DB row.
#[tauri::command]
#[specta::specta]
pub fn browser_reject_download(request_id: String) -> Result<(), AppError> {
    Ok(download_handler::reject_download(&request_id)?)
}

/// Ask for confirmation before retrying a failed or canceled download.
#[tauri::command]
#[specta::specta]
pub async fn browser_retry_download(
    id: String,
    app: AppHandle,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(download_service::retry_download(db.inner(), &app, &id).await?)
}

/// Delete a download record (and optionally the file on disk).
#[tauri::command]
#[specta::specta]
pub async fn browser_delete_download(
    id: String,
    delete_file: bool,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(download_service::delete_download(db.inner(), &id, delete_file).await?)
}

/// Remove old downloads that exceed the configured retention period.
#[tauri::command]
#[specta::specta]
pub async fn browser_clear_old_downloads(db: State<'_, SqlitePool>) -> Result<u64, AppError> {
    Ok(download_service::clear_old_downloads(db.inner()).await?)
}
