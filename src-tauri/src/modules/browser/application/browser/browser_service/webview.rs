//! Creating the isolated child webview that backs a browser tab.

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};

use super::adblock::{discover_profile_dir, BrowserAdblockState};
use super::paths::get_downloads_root_for_game;
use super::settings::{normalize_url, validate_http_url};
use crate::shared::errors::BrowserError;

/// Open a browser tab for a user-supplied URL (normalizes a missing scheme first).
pub async fn open_tab(
    app: AppHandle,
    db: SqlitePool,
    url: String,
    session_id: Option<String>,
) -> Result<String, BrowserError> {
    open_child_webview(app, db, normalize_url(&url), session_id).await
}

/// Open a new browser tab as a child Webview of the main window.
///
/// The webview:
/// - Blocks non-http(s) navigation via `on_navigation`.
/// - Uses WebView2's native download operation on Windows so authenticated
///   downloads retain the browser profile; other platforms use the fallback.
/// - Has NO IPC initialization script (remote pages are fully isolated).
pub async fn open_child_webview(
    app: AppHandle,
    db: SqlitePool,
    url: String,
    session_id: Option<String>,
) -> Result<String, BrowserError> {
    validate_http_url(&url)?;

    app.state::<BrowserAdblockState>()
        .ensure_loaded(&app, &db)
        .await?;
    let discover_profile = discover_profile_dir(&app)?;

    // Generate unique webview label for this tab
    let tab_id = uuid::Uuid::new_v4().to_string();
    let label = format!("browser-tab-{}", &tab_id[..8]);

    let game_id =
        crate::modules::system::adapters::sqlite::settings::get_setting(&db, "active_game_id")
            .await?
            .ok_or_else(|| {
                BrowserError::InvalidSetting("Discover requires an active game".to_string())
            })?;
    let downloads_root = get_downloads_root_for_game(&app, &db, &game_id).await;
    let history_db_for_load = db.clone();
    let history_db_for_title = db.clone();

    // The fallback downloader does not have WebView's authenticated request
    // context, so it is deliberately only installed where native WebView2
    // download operations are unavailable.
    #[cfg(not(target_os = "windows"))]
    let session_id_dl = session_id.clone();
    #[cfg(not(target_os = "windows"))]
    let game_id_dl = game_id.clone();
    #[cfg(not(target_os = "windows"))]
    let downloads_root_clone = downloads_root.clone();
    #[cfg(not(target_os = "windows"))]
    let app_for_confirmation = app.clone();

    // The main window must exist to attach a webview
    let window = app
        .get_window("main")
        .ok_or(BrowserError::WindowUnavailable)?;

    let webview_builder = tauri::webview::WebviewBuilder::new(
        label.clone(),
        WebviewUrl::External(
            "about:blank"
                .parse()
                .map_err(|error| BrowserError::InvalidUrl(format!("about:blank: {error}")))?,
        ),
    )
    .data_directory(discover_profile)
    .devtools(cfg!(debug_assertions))
    .zoom_hotkeys_enabled(true)
    .on_navigation({
        let app_handle = app.clone();
        let label = label.clone();
        move |nav_url: &tauri::Url| {
            let scheme = nav_url.scheme();
            let is_allowed = scheme == "http" || scheme == "https";
            let is_download_navigation = crate::modules::browser::application::browser::download_handler::is_gamebanana_download_url(nav_url.as_str());
            if is_allowed && !is_download_navigation {
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
        let loading = matches!(payload.event(), tauri::webview::PageLoadEvent::Started);
        let _ = webview.app_handle().emit(
            "browser:loading-changed",
            serde_json::json!({ "label": webview.label(), "loading": loading }),
        );
        if let tauri::webview::PageLoadEvent::Finished = payload.event() {
            if let Ok(url) = webview.url() {
                let label = webview.label().to_string();
                let url_str = url.to_string();
                let is_download_navigation = crate::modules::browser::application::browser::download_handler::is_gamebanana_download_url(&url_str)
                    || crate::modules::browser::application::browser::download_handler::has_native_download_for_label(&label);
                if !is_download_navigation {
                    let history_db = history_db_for_load.clone();
                    let history_url = url_str.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = super::metadata::record_history(
                            &history_db,
                            &history_url,
                            None,
                            None,
                        )
                        .await
                        {
                            log::debug!("Could not record Discover history: {error}");
                        }
                    });
                    let _ = webview.app_handle().emit(
                        "browser:url-changed",
                        serde_json::json!({
                            "label": label,
                            "url": url_str,
                        }),
                    );
                }
            }
        }
    })
    .on_document_title_changed(move |webview: tauri::webview::Webview<_>, title| {
        let label = webview.label().to_string();
        let url = webview.url().map(|u| u.to_string()).unwrap_or_default();
        let is_download_navigation = crate::modules::browser::application::browser::download_handler::is_gamebanana_download_url(&url)
            || crate::modules::browser::application::browser::download_handler::has_native_download_for_label(&label);
        if is_download_navigation {
            return;
        }
        let history_db = history_db_for_title.clone();
        let history_url = url.clone();
        let history_title = title.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) =
                super::metadata::update_history_metadata(&history_db, &history_url, &history_title)
                    .await
            {
                log::debug!("Could not update Discover history title: {error}");
            }
        });
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
        // WebView2 treats target=_blank and scripted windows alike. Discover
        // has no user-consent UI for them, so block the whole class by default.
        log::debug!("Blocked Discover new-window request: {url}");
        tauri::webview::NewWindowResponse::Deny
    });

    #[cfg(not(target_os = "windows"))]
    let webview_builder = webview_builder.on_download(move |_webview, event| {
        match event {
            tauri::webview::DownloadEvent::Requested {
                url: dl_url,
                destination,
            } => {
                let filename = destination
                    .file_name()
                    .and_then(|name| name.to_str())
                    .filter(|name| !name.trim().is_empty())
                    .map(str::to_owned)
                    .or_else(|| {
                        dl_url
                            .path_segments()
                            .and_then(|mut segs| segs.next_back())
                            .filter(|name| !name.trim().is_empty())
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(|| "download".to_string());

                let url_str = dl_url.to_string();
                let app = app_for_confirmation.clone();
                let downloads_root = downloads_root_clone.clone();
                let session_id = session_id_dl.clone();
                let game_id = game_id_dl.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = crate::modules::browser::application::browser::download_handler::request_download_confirmation(
                        &app,
                        game_id,
                        url_str,
                        filename,
                        downloads_root,
                        session_id,
                    )
                    .await
                    {
                        log::warn!("Unable to request download confirmation: {error}");
                    }
                });

                // Returning false prevents the native transfer. The background
                // downloader starts only after the frontend confirms.
                false
            }
            tauri::webview::DownloadEvent::Finished { .. } => {
                // Since we returned false in Requested, the native downloader shouldn't fire this.
                // Download completion persistence is handled inside `download_handler.rs`.
                true
            }
            _ => true,
        }
    });

    // We attach the webview to the main window.
    // By default, it would span the entire window size if we use inner_size,
    // which causes a 'full screen browser' flash. We initialize it with a minimum
    // 1x1 size so the frontend can properly animate/resize it into its container.
    let webview = window.add_child(
        webview_builder,
        tauri::LogicalPosition::new(0, 0),
        tauri::LogicalSize::new(1, 1),
    )?;

    // The native interceptor must be in place before the destination begins
    // loading; every child starts at about:blank for that reason.
    super::adblock::attach_native_request_filter(&webview, &app, &label)?;
    crate::modules::browser::application::browser::download_handler::attach_native_download_handler(
        &webview,
        &app,
        &db,
        &label,
        &downloads_root,
        session_id.clone(),
        game_id,
    )?;
    webview.navigate(
        url.parse()
            .map_err(|error| BrowserError::InvalidUrl(format!("{url}: {error}")))?,
    )?;

    Ok(label)
}
