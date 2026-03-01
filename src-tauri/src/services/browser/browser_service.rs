use chrono::Utc;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};

use super::download_service;

/// Illegal Windows filename characters to strip.
const ILLEGAL_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
/// Maximum safe filename length (chars, excluding extension).
const MAX_FILENAME_LEN: usize = 120;

/// Sanitize a raw filename from a download URL so it is safe to store on Windows.
///
/// Rules:
/// - Strip illegal Windows chars (`< > : " / \ | ? *`).
/// - Trim leading/trailing whitespace.
/// - Clamp total length to 120 chars (preserving extension).
pub fn sanitize_filename(raw: &str) -> String {
    let cleaned: String = raw.chars().filter(|c| !ILLEGAL_CHARS.contains(c)).collect();
    let cleaned = cleaned.trim().to_string();

    if cleaned.len() <= MAX_FILENAME_LEN {
        return if cleaned.is_empty() {
            format!("download_{}", Utc::now().timestamp())
        } else {
            cleaned
        };
    }

    // Preserve extension if present
    let path = Path::new(&cleaned);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&cleaned);

    if ext.is_empty() {
        stem[..MAX_FILENAME_LEN].to_string()
    } else {
        let max_stem = MAX_FILENAME_LEN.saturating_sub(ext.len() + 1);
        format!("{}.{}", &stem[..max_stem.min(stem.len())], ext)
    }
}

/// Resolve the collision-safe destination path for a download.
///
/// Layout:
/// - `root/YYYY-MM/<session_id>/<safe_filename>.<ext>` (session-linked)
/// - `root/YYYY-MM/adhoc/<timestamp>_<safe_filename>.<ext>` (no session)
pub fn compute_download_path(root: &Path, session_id: Option<&str>, filename: &str) -> PathBuf {
    let now = Utc::now();
    let month_dir = now.format("%Y-%m").to_string();
    let safe_name = sanitize_filename(filename);

    let dir = match session_id {
        Some(sid) => root.join(&month_dir).join(sid),
        None => root.join(&month_dir).join("adhoc"),
    };

    // Generate unique path (avoid collision)
    let mut candidate = dir.join(&safe_name);
    if !candidate.exists() {
        return candidate;
    }

    let path_obj = Path::new(&safe_name);
    let ext = path_obj.extension().and_then(|e| e.to_str()).unwrap_or("");
    let stem = path_obj
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&safe_name);

    let mut n = 2u32;
    loop {
        let suffixed = if ext.is_empty() {
            format!("{stem}_({n})")
        } else {
            format!("{stem}_({n}).{ext}")
        };
        candidate = dir.join(&suffixed);
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

/// Fetch the configured homepage URL from `browser_settings` table.
/// Falls back to `https://www.google.com` if not set.
pub async fn get_homepage(db: &SqlitePool) -> String {
    sqlx::query_scalar!("SELECT value FROM browser_settings WHERE key = 'homepage_url'")
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "https://www.google.com".to_string())
}

/// Save a new homepage URL to `browser_settings`.
pub async fn set_homepage(db: &SqlitePool, url: &str) -> Result<(), String> {
    validate_http_url(url)?;
    sqlx::query(
        "INSERT INTO browser_settings (key, value) VALUES ('homepage_url', ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(url)
    .execute(db)
    .await
    .map_err(|e| format!("DB error: {e}"))?;
    Ok(())
}

/// Validate that URL is http or https only.
pub fn validate_http_url(url: &str) -> Result<(), String> {
    let lower = url.trim().to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Ok(())
    } else {
        Err(format!(
            "Invalid URL scheme. Only http:// and https:// are allowed. Got: '{url}'"
        ))
    }
}

/// Auto-prepend https:// if user typed a URL without a scheme.
pub fn normalize_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

/// Get the `BrowserDownloadsRoot` path.
///
/// Priority: `browser_settings.downloads_root` (if non-empty + writable) → default.
pub async fn get_downloads_root(app: &AppHandle, db: &SqlitePool) -> PathBuf {
    let override_path: Option<String> =
        sqlx::query_scalar!("SELECT value FROM browser_settings WHERE key = 'downloads_root'")
            .fetch_optional(db)
            .await
            .ok()
            .flatten();

    if let Some(p) = override_path {
        if !p.is_empty() {
            let path = PathBuf::from(&p);
            if path.exists() || std::fs::create_dir_all(&path).is_ok() {
                return path;
            }
        }
    }

    // Default: AppData/EMM2/BrowserDownloads/
    match app.path().app_data_dir() {
        Ok(data_dir) => data_dir.join("BrowserDownloads"),
        Err(_) => PathBuf::from("BrowserDownloads"),
    }
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
    let db_for_finish = db.clone();
    let app_for_finish = app.clone();
    let label_clone = label.clone();

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
                    .and_then(|segs| segs.last())
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

                // Record download in DB (fire-and-forget)
                let db_c = db_for_start.clone();
                let sid = session_id_dl.clone();
                let dest_str = dest.to_string_lossy().to_string();
                let url_str = dl_url.to_string();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = download_service::create_download(
                        &db_c,
                        sid.as_deref(),
                        &filename,
                        &url_str,
                        &dest_str,
                    )
                    .await
                    {
                        log::error!("Failed to record download: {e}");
                    }
                });

                true // allow download
            }
            tauri::webview::DownloadEvent::Finished {
                url: dl_url,
                path,
                success,
            } => {
                let url_str = dl_url.to_string();
                let path_str = path.as_ref().map(|p| p.to_string_lossy().to_string());
                let db_c = db_for_finish.clone();
                let app_c = app_for_finish.clone();
                let lbl = label_clone.clone();

                tauri::async_runtime::spawn(async move {
                    if let Err(e) = download_service::on_download_finished(
                        &db_c,
                        &app_c,
                        &url_str,
                        path_str.as_deref(),
                        success,
                        &lbl,
                    )
                    .await
                    {
                        log::error!("Failed to handle download finish: {e}");
                    }
                });

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

/// Navigate an existing webview to a new URL.
pub async fn navigate(app: AppHandle, label: &str, url: String) -> Result<(), String> {
    let window = app.get_window("main").ok_or("Main window not found")?;
    let webview = window.get_webview(label).ok_or("Webview not found")?;

    let url_parsed = normalize_url(&url)
        .parse::<tauri::Url>()
        .map_err(|e| format!("Invalid URL: {e}"))?;

    webview
        .navigate(url_parsed)
        .map_err(|e| format!("Navigation failed: {e}"))?;

    Ok(())
}

/// Reload a webview.
pub async fn reload_tab(app: AppHandle, label: &str) -> Result<(), String> {
    let window = app.get_window("main").ok_or("Main window not found")?;
    let webview = window.get_webview(label).ok_or("Webview not found")?;
    webview
        .reload()
        .map_err(|e| format!("Reload failed: {e}"))?;
    Ok(())
}

/// Clear browsing data.
pub async fn clear_data(app: AppHandle, label: &str) -> Result<(), String> {
    let window = app.get_window("main").ok_or("Main window not found")?;
    let webview = window.get_webview(label).ok_or("Webview not found")?;

    // In Tauri v2, we can use clear_all_browsing_data directly on the webview.
    // This works even if the page is isolated and doesn't have the Tauri JS API.
    webview
        .clear_all_browsing_data()
        .map_err(|e| format!("Failed to clear browsing data: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sanitize_filename_strips_illegal_chars() {
        let raw = "file<o:m?p\"i>l|e*d.zip";
        let clean = sanitize_filename(raw);
        assert_eq!(clean, "fileompiled.zip");

        let raw2 = "   leading_and_trailing  \n";
        assert_eq!(sanitize_filename(raw2), "leading_and_trailing");

        let raw3 = "C:\\Windows\\System32\\cmd.exe";
        assert_eq!(sanitize_filename(raw3), "CWindowsSystem32cmd.exe");
    }

    #[test]
    fn test_sanitize_filename_max_length() {
        let long_stem = "A".repeat(150);
        let ext = ".zip";
        let raw = format!("{long_stem}{ext}");
        let clean = sanitize_filename(&raw);

        assert_eq!(clean.len(), MAX_FILENAME_LEN);
        assert!(clean.ends_with(".zip"));

        // Exceeded by far
        let very_long = "X".repeat(300);
        let clean2 = sanitize_filename(&very_long);
        assert_eq!(clean2.len(), MAX_FILENAME_LEN);
        assert!(clean2.chars().all(|c| c == 'X'));
    }

    #[test]
    fn test_compute_download_path_with_session() {
        let temp = tempdir().unwrap();
        let session_id = "1234-5678-uuid";
        let filename = "mod_pack.zip";

        let path = compute_download_path(temp.path(), Some(session_id), filename);

        let month_dir = Utc::now().format("%Y-%m").to_string();
        let expected = temp
            .path()
            .join(&month_dir)
            .join(session_id)
            .join("mod_pack.zip");
        assert_eq!(path, expected);
    }

    #[test]
    fn test_compute_download_path_adhoc() {
        let temp = tempdir().unwrap();
        let filename = "loose_mod.rar";

        let path = compute_download_path(temp.path(), None, filename);

        let month_dir = Utc::now().format("%Y-%m").to_string();
        let expected = temp
            .path()
            .join(&month_dir)
            .join("adhoc")
            .join("loose_mod.rar");
        assert_eq!(path, expected);
    }

    #[test]
    fn test_compute_download_path_collision() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        // Make the structure
        let month_dir = Utc::now().format("%Y-%m").to_string();
        let adhoc_dir = root.join(&month_dir).join("adhoc");
        std::fs::create_dir_all(&adhoc_dir).unwrap();

        let filename = "mod.zip";

        // 1st time
        let path1 = compute_download_path(root, None, filename);
        assert_eq!(path1.file_name().unwrap().to_str().unwrap(), "mod.zip");
        // touch it
        std::fs::File::create(&path1).unwrap();

        // 2nd time (collision)
        let path2 = compute_download_path(root, None, filename);
        assert_eq!(path2.file_name().unwrap().to_str().unwrap(), "mod_(2).zip");
        std::fs::File::create(&path2).unwrap();

        // 3rd time (collision again)
        let path3 = compute_download_path(root, None, filename);
        assert_eq!(path3.file_name().unwrap().to_str().unwrap(), "mod_(3).zip");

        // Test extensionless collision
        let path_noext1 = compute_download_path(root, None, "readme");
        assert_eq!(path_noext1.file_name().unwrap().to_str().unwrap(), "readme");
        std::fs::File::create(&path_noext1).unwrap();

        let path_noext2 = compute_download_path(root, None, "readme");
        assert_eq!(
            path_noext2.file_name().unwrap().to_str().unwrap(),
            "readme_(2)"
        );
    }
}
