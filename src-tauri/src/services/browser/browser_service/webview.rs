//! Creating the isolated child webview that backs a browser tab.

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};

use super::paths::{compute_download_path, get_downloads_root};
use super::settings::{normalize_url, validate_http_url};

/// Open a browser tab for a user-supplied URL (normalizes a missing scheme first).
pub async fn open_tab(
    app: AppHandle,
    db: SqlitePool,
    url: String,
    session_id: Option<String>,
) -> Result<String, String> {
    open_child_webview(app, db, normalize_url(&url), session_id).await
}

/// Open a new browser tab as a child Webview of the main window.
///
/// The webview:
/// - Blocks non-http(s) navigation via `on_navigation`.
/// - Intercepts downloads via `on_download` → redirects to `BrowserDownloadsRoot`.
/// - Has NO IPC initialization script (remote pages are fully isolated).
pub async fn open_child_webview(
    app: AppHandle,
    db: SqlitePool,
    url: String,
    session_id: Option<String>,
) -> Result<String, String> {
    validate_http_url(&url)?;

    // Generate unique webview label for this tab
    let tab_id = uuid::Uuid::new_v4().to_string();
    let label = format!("browser-tab-{}", &tab_id[..8]);

    let downloads_root = get_downloads_root(&app, &db).await;

    // Ensure BrowserDownloadsRoot exists
    std::fs::create_dir_all(&downloads_root)
        .map_err(|e| format!("Cannot create BrowserDownloadsRoot: {e}"))?;

    // Clone values for use inside closures
    let session_id_dl = session_id.clone();
    let downloads_root_clone = downloads_root.clone();
    let db_for_start = db.clone();
    let app_for_finish = app.clone();

    // The main window must exist to attach a webview
    let window = app.get_window("main").ok_or("Main window not found")?;

    let webview_builder = tauri::webview::WebviewBuilder::new(
        label.clone(),
        WebviewUrl::External(url.parse().map_err(|e| format!("Invalid URL: {e}"))?),
    )
    .devtools(true)
    .on_navigation({
        let app_handle = app.clone();
        let label = label.clone();
        move |nav_url: &tauri::Url| {
            let scheme = nav_url.scheme();
            let is_allowed = scheme == "http" || scheme == "https";
            if is_allowed {
                let _ = app_handle.emit(
                    "browser:url-changed",
                    serde_json::json!({
                        "label": label,
                        "url": nav_url.to_string(),
                    }),
                );
            }
            is_allowed
        }
    })
    .on_page_load(move |webview: tauri::webview::Webview<_>, payload| {
        if let tauri::webview::PageLoadEvent::Finished = payload.event() {
            if let Ok(url) = webview.url() {
                let label = webview.label().to_string();
                let url_str = url.to_string();
                let _ = webview.app_handle().emit(
                    "browser:url-changed",
                    serde_json::json!({
                        "label": label,
                        "url": url_str,
                    }),
                );
            }
        }
    })
    .on_document_title_changed(move |webview: tauri::webview::Webview<_>, title| {
        let label = webview.label().to_string();
        let url = webview.url().map(|u| u.to_string()).unwrap_or_default();
        let _ = webview.app_handle().emit(
            "browser:url-changed",
            serde_json::json!({
                "label": label,
                "url": url,
                "title": title,
            }),
        );
    })
    .initialization_script(
        r#"
        (function() {
            function sync() {
                // Use a simple event emission if possible, or just wait for backend
                // Actually, we can't easily emit from here without IPC
                // But we can trigger a dummy title change to force the backend to fire
                const t = document.title;
                document.title = t + " ";
                document.title = t;
            }
            window.addEventListener('popstate', sync);
            window.addEventListener('hashchange', sync);
            const disp = history.pushState;
            history.pushState = function() {
                disp.apply(this, arguments);
                sync();
            };
            const disr = history.replaceState;
            history.replaceState = function() {
                disr.apply(this, arguments);
                sync();
            };
        })();
    "#,
    )
    .on_new_window(move |url: tauri::Url, _features| {
        let app_handle = app.clone();
        let url_str = url.to_string();

        // Emit event to frontend to open a new tab in our UI
        let _ = app_handle.emit(
            "browser:new-tab-requested",
            serde_json::json!({
                "url": url_str,
            }),
        );

        // Deny the default OS window creation
        tauri::webview::NewWindowResponse::Deny
    })
    .on_download(move |_webview, event| {
        match event {
            tauri::webview::DownloadEvent::Requested {
                url: dl_url,
                destination,
            } => {
                let filename = dl_url
                    .path_segments()
                    .and_then(|mut segs| segs.next_back())
                    .unwrap_or("download")
                    .to_string();

                let dest = compute_download_path(
                    &downloads_root_clone,
                    session_id_dl.as_deref(),
                    &filename,
                );

                // Create parent directory
                if let Some(parent) = dest.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                *destination = dest.clone();

                let db_c = db_for_start.clone();
                let sid = session_id_dl.clone();
                let dest_str = dest.clone();
                let url_str = dl_url.to_string();
                let app_c = app_for_finish.clone(); // Reused cloned AppHandle

                tauri::async_runtime::spawn(async move {
                    if let Err(e) =
                        crate::services::browser::download_handler::start_concurrent_download(
                            app_c,
                            db_c,
                            url_str,
                            filename.clone(),
                            dest_str,
                            sid,
                        )
                        .await
                    {
                        log::error!(
                            "Failed to start concurrent download for {}: {}",
                            filename,
                            e
                        );
                    }
                });

                // RETURN FALSE to prevent the WebView native overlapping download mechanism.
                // We're handling the download in our reqwest background task!
                false
            }
            tauri::webview::DownloadEvent::Finished { .. } => {
                // Since we returned false in Requested, the native downloader shouldn't fire this.
                // All finishing logic (and Smart Import trigger) is now safely handled inside `download_handler.rs`.
                true
            }
            _ => true,
        }
    });

    // We attach the webview to the main window.
    // By default, it would span the entire window size if we use inner_size,
    // which causes a 'full screen browser' flash. We initialize it with a minimum
    // 1x1 size so the frontend can properly animate/resize it into its container.
    let _webview = window
        .add_child(
            webview_builder,
            tauri::LogicalPosition::new(0, 0),
            tauri::LogicalSize::new(1, 1),
        )
        .map_err(|e| format!("Failed to attach webview tab to window: {e}"))?;

    Ok(label)
}
