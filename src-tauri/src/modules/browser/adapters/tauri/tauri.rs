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

/// Open a Discover URL in the user's default external browser.
#[tauri::command]
#[specta::specta]
pub fn browser_open_externally(url: String, app: AppHandle) -> Result<(), AppError> {
    Ok(browser_service::open_externally(app, url)?)
}

/// Change the zoom level for one Discover tab.
#[tauri::command]
#[specta::specta]
pub fn browser_set_zoom(label: String, zoom: f64, app: AppHandle) -> Result<(), AppError> {
    Ok(browser_service::set_zoom(app, &label, zoom)?)
}

/// Find text in one Discover tab.
#[tauri::command]
#[specta::specta]
pub fn browser_find_in_page(label: String, query: String, app: AppHandle) -> Result<(), AppError> {
    Ok(browser_service::find_in_page(app, &label, query)?)
}

/// Return the persisted Discover ad-block setting (enabled by default).
#[tauri::command]
#[specta::specta]
pub async fn browser_get_adblock_enabled(db: State<'_, SqlitePool>) -> Result<bool, AppError> {
    Ok(browser_service::get_enabled(db.inner()).await?)
}

/// Persist Discover's ad-block toggle. Existing tabs pick it up on reload.
#[tauri::command]
#[specta::specta]
pub async fn browser_set_adblock_enabled(
    enabled: bool,
    app: AppHandle,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(browser_service::set_enabled(&app, db.inner(), enabled).await?)
}

/// Clear cookies plus site data for the shared Discover profile.
#[tauri::command]
#[specta::specta]
pub async fn browser_clear_cookies_and_site_data(
    label: String,
    app: AppHandle,
) -> Result<(), AppError> {
    Ok(browser_service::clear_cookies_and_site_data(app, &label).await?)
}

/// Clear only disk cache for the shared Discover profile.
#[tauri::command]
#[specta::specta]
pub async fn browser_clear_cache(label: String, app: AppHandle) -> Result<(), AppError> {
    Ok(browser_service::clear_cache(app, &label).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn browser_list_bookmarks(
    db: State<'_, SqlitePool>,
) -> Result<Vec<browser_service::BrowserBookmark>, AppError> {
    Ok(browser_service::list_bookmarks(db.inner()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn browser_add_bookmark(
    url: String,
    title: Option<String>,
    favicon: Option<String>,
    db: State<'_, SqlitePool>,
) -> Result<browser_service::BrowserBookmark, AppError> {
    Ok(
        browser_service::add_bookmark(db.inner(), &url, title.as_deref(), favicon.as_deref())
            .await?,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn browser_delete_bookmark(
    id: String,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(browser_service::delete_bookmark(db.inner(), &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn browser_update_bookmark(
    id: String,
    url: String,
    title: String,
    db: State<'_, SqlitePool>,
) -> Result<browser_service::BrowserBookmark, AppError> {
    Ok(browser_service::update_bookmark(db.inner(), &id, &url, &title).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn browser_list_history(
    limit: i64,
    db: State<'_, SqlitePool>,
) -> Result<Vec<browser_service::BrowserHistoryEntry>, AppError> {
    Ok(browser_service::list_history(db.inner(), limit).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn browser_clear_history(db: State<'_, SqlitePool>) -> Result<(), AppError> {
    Ok(browser_service::clear_history(db.inner()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn browser_get_session_tabs(
    game_id: String,
    db: State<'_, SqlitePool>,
) -> Result<Vec<browser_service::BrowserSessionTab>, AppError> {
    Ok(browser_service::get_session_tabs(db.inner(), &game_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn browser_save_session_tabs(
    game_id: String,
    tabs: Vec<browser_service::BrowserSessionTab>,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(browser_service::save_session_tabs(db.inner(), &game_id, &tabs).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn browser_get_privacy_summary(
    db: State<'_, SqlitePool>,
) -> Result<browser_service::BrowserPrivacySummary, AppError> {
    Ok(browser_service::privacy_summary(db.inner()).await?)
}

/// Get the configured browser homepage URL.
#[tauri::command]
#[specta::specta]
pub async fn browser_get_homepage(
    game_id: String,
    db: State<'_, SqlitePool>,
) -> Result<String, AppError> {
    Ok(browser_service::get_homepage(db.inner(), &game_id).await)
}

/// Set a new browser homepage URL. Validates http/https scheme.
#[tauri::command]
#[specta::specta]
pub async fn browser_set_homepage(
    game_id: String,
    url: String,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(browser_service::set_homepage(db.inner(), &game_id, &url).await?)
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
    game_id: String,
    db: State<'_, SqlitePool>,
) -> Result<Vec<download_service::BrowserDownloadDto>, AppError> {
    Ok(download_service::list_downloads(db.inner(), &game_id).await?)
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

/// Pause a live native WebView2 download when the server supports it.
#[tauri::command]
#[specta::specta]
pub fn browser_pause_download(id: String, app: AppHandle) -> Result<(), AppError> {
    Ok(download_service::pause_download(&app, &id)?)
}

/// Resume a paused or interrupted native WebView2 download when available.
#[tauri::command]
#[specta::specta]
pub fn browser_resume_download(id: String, app: AppHandle) -> Result<(), AppError> {
    Ok(download_service::resume_download(&app, &id)?)
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

/// Reload the source tab so a host can issue a fresh download URL.
#[tauri::command]
#[specta::specta]
pub async fn browser_refresh_download_link(
    id: String,
    app: AppHandle,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(download_service::refresh_download_link(db.inner(), app, &id).await?)
}

/// Open the original source URL in a new Discover tab.
#[tauri::command]
#[specta::specta]
pub async fn browser_open_download_source(
    id: String,
    app: AppHandle,
    db: State<'_, SqlitePool>,
) -> Result<String, AppError> {
    Ok(download_service::open_download_source(db.inner(), app, &id).await?)
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

/// Rename a downloaded file and update its history entry.
#[tauri::command]
#[specta::specta]
pub async fn browser_rename_download(
    id: String,
    filename: String,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(download_service::rename_download(db.inner(), &id, &filename).await?)
}

/// Open a downloaded file using its default system application.
#[tauri::command]
#[specta::specta]
pub async fn browser_open_download_file(
    id: String,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(download_service::open_download_file(db.inner(), &id).await?)
}

/// Reveal a downloaded file in the system file manager.
#[tauri::command]
#[specta::specta]
pub async fn browser_open_download_location(
    id: String,
    db: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    Ok(download_service::open_download_location(db.inner(), &id).await?)
}

/// Remove old downloads that exceed the configured retention period.
#[tauri::command]
#[specta::specta]
pub async fn browser_clear_old_downloads(db: State<'_, SqlitePool>) -> Result<u64, AppError> {
    Ok(download_service::clear_old_downloads(db.inner()).await?)
}
