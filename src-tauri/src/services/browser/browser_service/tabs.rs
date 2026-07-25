//! Driving an already-open tab: navigation, history, reload, data clearing.

use tauri::{AppHandle, Manager};

use super::settings::normalize_url;

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

/// Navigate a webview one step back in its session history.
///
/// Remote pages have no Tauri IPC, but `eval` runs through the native
/// WebView2 ExecuteScript channel, so the history API is always reachable.
pub async fn go_back(app: AppHandle, label: &str) -> Result<(), String> {
    run_history_script(app, label, "history.back()")
}

/// Navigate a webview one step forward in its session history.
pub async fn go_forward(app: AppHandle, label: &str) -> Result<(), String> {
    run_history_script(app, label, "history.forward()")
}

fn run_history_script(app: AppHandle, label: &str, script: &str) -> Result<(), String> {
    let window = app.get_window("main").ok_or("Main window not found")?;
    let webview = window.get_webview(label).ok_or("Webview not found")?;
    webview
        .eval(script)
        .map_err(|e| format!("History navigation failed: {e}"))
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
