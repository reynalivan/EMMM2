use sqlx::SqlitePool;
use tauri::{AppHandle, State};

use crate::services::browser::{
    browser_service, download_service, import_service, session_service,
};

// ── Browser Tab ──────────────────────────────────────────────────────────────

/// Open a new in-app browser tab (creates a new Webview).
/// Returns the webview label so the frontend can track the tab.
#[tauri::command]
pub async fn browser_open_tab(
    url: String,
    session_id: Option<String>,
    app: AppHandle,
    db: State<'_, SqlitePool>,
) -> Result<String, String> {
    let url = browser_service::normalize_url(&url);
    browser_service::open_child_webview(app, db.inner().clone(), url, session_id).await
}

/// Navigate an existing browser tab to a new URL.
#[tauri::command]
pub async fn browser_navigate(label: String, url: String, app: AppHandle) -> Result<(), String> {
    browser_service::navigate(app, &label, url).await
}

/// Reload an existing browser tab.
#[tauri::command]
pub async fn browser_reload_tab(label: String, app: AppHandle) -> Result<(), String> {
    browser_service::reload_tab(app, &label).await
}

/// Clear cookies and cache for a specific browser tab.
#[tauri::command]
pub async fn browser_clear_data(label: String, app: AppHandle) -> Result<(), String> {
    browser_service::clear_data(app, &label).await
}

/// Get the configured browser homepage URL.
#[tauri::command]
pub async fn browser_get_homepage(db: State<'_, SqlitePool>) -> Result<String, String> {
    Ok(browser_service::get_homepage(db.inner()).await)
}

/// Set a new browser homepage URL. Validates http/https scheme.
#[tauri::command]
pub async fn browser_set_homepage(url: String, db: State<'_, SqlitePool>) -> Result<(), String> {
    browser_service::set_homepage(db.inner(), &url).await
}

// ── Download Manager ─────────────────────────────────────────────────────────

/// Return all browser downloads ordered by most recent first.
#[tauri::command]
pub async fn browser_list_downloads(
    db: State<'_, SqlitePool>,
) -> Result<Vec<download_service::BrowserDownloadDto>, String> {
    download_service::list_downloads(db.inner()).await
}

/// Cancel (and optionally delete the file for) a specific download.
#[tauri::command]
pub async fn browser_cancel_download(
    id: String,
    delete_file: Option<bool>,
    db: State<'_, SqlitePool>,
) -> Result<(), String> {
    download_service::update_status(db.inner(), &id, "canceled", None, None, None, None).await?;
    if delete_file.unwrap_or(false) {
        download_service::delete_download(db.inner(), &id, true).await?;
    }
    Ok(())
}

/// Delete a download record (and optionally the file on disk).
#[tauri::command]
pub async fn browser_delete_download(
    id: String,
    delete_file: bool,
    db: State<'_, SqlitePool>,
) -> Result<(), String> {
    download_service::delete_download(db.inner(), &id, delete_file).await
}

/// Remove all downloads with status `imported`.
#[tauri::command]
pub async fn browser_clear_imported(db: State<'_, SqlitePool>) -> Result<u64, String> {
    download_service::clear_imported(db.inner()).await
}

/// Remove old downloads that exceed the configured retention period.
#[tauri::command]
pub async fn browser_clear_old_downloads(db: State<'_, SqlitePool>) -> Result<u64, String> {
    download_service::clear_old_downloads(db.inner()).await
}

// ── Import ───────────────────────────────────────────────────────────────────

/// Queue multiple import jobs for a set of finished download IDs with a chosen game.
/// Called by the Download Manager "Import Selected" bulk action after Game Picker confirms.
#[tauri::command]
pub async fn browser_import_selected(
    ids: Vec<String>,
    game_id: String,
    db: State<'_, SqlitePool>,
    app: AppHandle,
) -> Result<Vec<String>, String> {
    import_service::bulk_queue_imports(db.inner(), &app, &ids, &game_id).await
}

/// Return all pending/active import jobs.
#[tauri::command]
pub async fn import_get_queue(
    db: State<'_, SqlitePool>,
) -> Result<Vec<import_service::ImportJobDto>, String> {
    import_service::list_jobs(db.inner()).await
}

/// Confirm a `needs_review` import job — provide game, category, and optional object.
#[tauri::command]
pub async fn import_confirm_review(
    job_id: String,
    game_id: String,
    category: String,
    object_id: Option<String>,
    db: State<'_, SqlitePool>,
    app: AppHandle,
) -> Result<(), String> {
    import_service::confirm_review(
        db.inner(),
        &app,
        &job_id,
        &game_id,
        &category,
        object_id.as_deref(),
    )
    .await
}

/// Skip / cancel a specific import job and remove its staging folder.
#[tauri::command]
pub async fn import_skip(job_id: String, db: State<'_, SqlitePool>) -> Result<(), String> {
    import_service::cancel_job(db.inner(), &job_id).await
}

// ── Download Session ──────────────────────────────────────────────────────────

/// Create a Download Session for a Discover Hub (e.g., GameBanana) mod download.
#[tauri::command]
pub async fn create_download_session(
    source: String,
    submission_id: Option<String>,
    mod_title: Option<String>,
    profile_url: Option<String>,
    game_id: Option<String>,
    expected_keywords: Option<Vec<String>>,
    db: State<'_, SqlitePool>,
) -> Result<String, String> {
    let kw_refs: Option<Vec<&str>> = expected_keywords
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());
    session_service::create_session(
        db.inner(),
        &source,
        submission_id.as_deref(),
        mod_title.as_deref(),
        profile_url.as_deref(),
        game_id.as_deref(),
        kw_refs.as_deref(),
    )
    .await
}
