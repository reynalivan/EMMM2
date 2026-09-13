use crate::shared::errors::AppError;
use tauri::{ipc::Channel, AppHandle};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfo {
    pub version: String,
    pub current_version: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(tag = "event", content = "data")]
pub enum AppUpdateProgress {
    Started {
        #[serde(rename = "contentLength")]
        content_length: Option<u64>,
    },
    Progress {
        #[serde(rename = "chunkLength")]
        chunk_length: u64,
    },
    Finished,
}

fn app_update_error(action: &str, error: tauri_plugin_updater::Error) -> AppError {
    AppError::Internal(format!("App update {action} failed: {error}"))
}

fn send_app_update_progress(channel: &Channel<AppUpdateProgress>, event: AppUpdateProgress) {
    if let Err(error) = channel.send(event) {
        log::warn!("Could not report app update progress: {error}");
    }
}

#[specta::specta]
#[tauri::command]
pub async fn check_app_update(app: AppHandle) -> Result<Option<AppUpdateInfo>, AppError> {
    let updater = app
        .updater()
        .map_err(|error| app_update_error("initialization", error))?;
    let update = updater
        .check()
        .await
        .map_err(|error| app_update_error("check", error))?;

    Ok(update.map(|update| AppUpdateInfo {
        version: update.version,
        current_version: update.current_version,
        body: update.body,
    }))
}

#[specta::specta]
#[tauri::command]
pub async fn install_app_update(
    app: AppHandle,
    on_progress: Channel<AppUpdateProgress>,
) -> Result<(), AppError> {
    let updater = app
        .updater()
        .map_err(|error| app_update_error("initialization", error))?;
    let update = updater
        .check()
        .await
        .map_err(|error| app_update_error("check", error))?
        .ok_or_else(|| AppError::NotFound("No application update is available".to_string()))?;

    let download_channel = on_progress.clone();
    let finished_channel = on_progress;
    let mut started = false;
    update
        .download_and_install(
            move |chunk_length, content_length| {
                if !started {
                    send_app_update_progress(
                        &download_channel,
                        AppUpdateProgress::Started { content_length },
                    );
                    started = true;
                }
                send_app_update_progress(
                    &download_channel,
                    AppUpdateProgress::Progress {
                        chunk_length: chunk_length as u64,
                    },
                );
            },
            move || {
                send_app_update_progress(&finished_channel, AppUpdateProgress::Finished);
            },
        )
        .await
        .map_err(|error| app_update_error("installation", error))?;

    app.restart();
}
