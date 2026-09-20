//! Driving an already-open tab: navigation, history, reload, data clearing.

use tauri::{AppHandle, Manager, Webview};
use tauri_plugin_opener::OpenerExt;

use super::settings::normalize_url;
use crate::shared::errors::BrowserError;

/// Resolve the child webview a tab command targets.
///
/// Every operation in this module starts here; spelling the two lookups out
/// per function meant five copies of the same pair of error strings.
pub(super) fn resolve_webview(app: &AppHandle, label: &str) -> Result<Webview, BrowserError> {
    if !label.starts_with("browser-tab-") {
        return Err(BrowserError::WebviewNotFound {
            label: label.to_string(),
        });
    }
    let window = app
        .get_window("main")
        .ok_or(BrowserError::WindowUnavailable)?;
    window
        .get_webview(label)
        .ok_or_else(|| BrowserError::WebviewNotFound {
            label: label.to_string(),
        })
}

/// Navigate an existing webview to a new URL.
pub async fn navigate(app: AppHandle, label: &str, url: String) -> Result<(), BrowserError> {
    let webview = resolve_webview(&app, label)?;

    let url_parsed = normalize_url(&url)
        .parse::<tauri::Url>()
        .map_err(|error| BrowserError::InvalidUrl(format!("{url}: {error}")))?;

    webview.navigate(url_parsed)?;
    Ok(())
}

/// Navigate a webview one step back in its session history.
///
/// Remote pages have no Tauri IPC, but `eval` runs through the native
/// WebView2 ExecuteScript channel, so the history API is always reachable.
pub async fn go_back(app: AppHandle, label: &str) -> Result<(), BrowserError> {
    run_history_script(app, label, "history.back()")
}

/// Navigate a webview one step forward in its session history.
pub async fn go_forward(app: AppHandle, label: &str) -> Result<(), BrowserError> {
    run_history_script(app, label, "history.forward()")
}

fn run_history_script(app: AppHandle, label: &str, script: &str) -> Result<(), BrowserError> {
    let webview = resolve_webview(&app, label)?;
    webview.eval(script)?;
    Ok(())
}

/// Reload a webview.
pub async fn reload_tab(app: AppHandle, label: &str) -> Result<(), BrowserError> {
    let webview = resolve_webview(&app, label)?;
    webview.reload()?;
    Ok(())
}

/// Set the zoom level for one Discover tab.
pub fn set_zoom(app: AppHandle, label: &str, zoom: f64) -> Result<(), BrowserError> {
    if !(0.5..=3.0).contains(&zoom) {
        return Err(BrowserError::InvalidSetting(
            "Discover zoom must be between 50% and 300%".into(),
        ));
    }
    resolve_webview(&app, label)?.set_zoom(zoom)?;
    Ok(())
}

/// Ask the native page to select the next matching text fragment.
pub fn find_in_page(app: AppHandle, label: &str, query: String) -> Result<(), BrowserError> {
    let query = serde_json::to_string(&query)
        .map_err(|error| BrowserError::InvalidSetting(format!("invalid find query: {error}")))?;
    resolve_webview(&app, label)?.eval(format!(
        "window.find({query}, false, false, true, false, true, false);"
    ))?;
    Ok(())
}

/// Open an absolute HTTP(S) page in the user's default browser.
pub fn open_externally(app: AppHandle, url: String) -> Result<(), BrowserError> {
    let normalized = normalize_url(&url);
    let parsed = tauri::Url::parse(&normalized)
        .map_err(|error| BrowserError::InvalidUrl(format!("{url}: {error}")))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(BrowserError::InvalidUrl(
            "only absolute HTTP(S) URLs can be opened".into(),
        ));
    }
    app.opener()
        .open_url(parsed.to_string(), None::<&str>)
        .map_err(|error| BrowserError::Io(format!("could not open external browser: {error}")))
}

/// Clear cookies plus local/session storage and IndexedDB from the shared
/// Discover profile, then reload every Discover tab so pages observe the new
/// unauthenticated state.
pub async fn clear_cookies_and_site_data(app: AppHandle, label: &str) -> Result<(), BrowserError> {
    super::adblock::clear_cookies_and_site_data(&app, label).await?;
    reload_all_discover_tabs(&app)
}

/// Clear only the shared Discover profile's disk cache. Login/session storage
/// is intentionally preserved.
pub async fn clear_cache(app: AppHandle, label: &str) -> Result<(), BrowserError> {
    super::adblock::clear_cache(&app, label).await?;
    reload_all_discover_tabs(&app)
}

fn reload_all_discover_tabs(app: &AppHandle) -> Result<(), BrowserError> {
    let window = app
        .get_window("main")
        .ok_or(BrowserError::WindowUnavailable)?;
    for webview in window.webviews() {
        if webview.label().starts_with("browser-tab-") {
            webview.reload()?;
        }
    }
    Ok(())
}
