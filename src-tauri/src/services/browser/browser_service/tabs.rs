//! Driving an already-open tab: navigation, history, reload, data clearing.

use tauri::{AppHandle, Manager, Webview};

use super::settings::normalize_url;
use crate::domain::errors::BrowserError;

/// Resolve the child webview a tab command targets.
///
/// Every operation in this module starts here; spelling the two lookups out
/// per function meant five copies of the same pair of error strings.
pub(super) fn resolve_webview(app: &AppHandle, label: &str) -> Result<Webview, BrowserError> {
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

/// Clear browsing data.
pub async fn clear_data(app: AppHandle, label: &str) -> Result<(), BrowserError> {
    let webview = resolve_webview(&app, label)?;

    // In Tauri v2, we can use clear_all_browsing_data directly on the webview.
    // This works even if the page is isolated and doesn't have the Tauri JS API.
    webview.clear_all_browsing_data()?;

    Ok(())
}
