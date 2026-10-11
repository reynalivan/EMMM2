use crate::shared::errors::AppError;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{ipc::Channel, AppHandle};
use tauri_plugin_updater::UpdaterExt;

static APP_UPDATE_INSTALL_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

struct AppUpdateInstallGuard<'a> {
    in_progress: &'a AtomicBool,
}

impl<'a> AppUpdateInstallGuard<'a> {
    fn acquire(in_progress: &'a AtomicBool) -> Result<Self, AppError> {
        in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                AppError::Validation(
                    "An application update installation is already in progress".to_string(),
                )
            })?;
        Ok(Self { in_progress })
    }
}

impl Drop for AppUpdateInstallGuard<'_> {
    fn drop(&mut self) {
        self.in_progress.store(false, Ordering::Release);
    }
}

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
    let _install_guard = AppUpdateInstallGuard::acquire(&APP_UPDATE_INSTALL_IN_PROGRESS)?;
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

#[cfg(test)]
mod install_admission_tests {
    use super::AppUpdateInstallGuard;
    use crate::shared::errors::AppError;
    use std::future::{pending, Future};
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Barrier};
    use std::task::{Context, Waker};

    #[test]
    fn only_one_concurrent_install_is_admitted() {
        const CONTENDERS: usize = 4;
        let in_progress = Arc::new(AtomicBool::new(false));
        let start = Arc::new(Barrier::new(CONTENDERS));
        let attempted = Arc::new(Barrier::new(CONTENDERS));
        let jobs: Vec<_> = (0..CONTENDERS)
            .map(|_| {
                let in_progress = Arc::clone(&in_progress);
                let start = Arc::clone(&start);
                let attempted = Arc::clone(&attempted);
                std::thread::spawn(move || {
                    start.wait();
                    let guard = AppUpdateInstallGuard::acquire(&in_progress);
                    let admitted = guard.is_ok();
                    attempted.wait();
                    drop(guard);
                    admitted
                })
            })
            .collect();

        let admitted = jobs
            .into_iter()
            .map(|job| usize::from(job.join().expect("install admission contender panicked")))
            .sum::<usize>();
        assert_eq!(admitted, 1);
        assert!(AppUpdateInstallGuard::acquire(&in_progress).is_ok());
    }

    #[test]
    fn rejected_attempt_does_not_unlock_an_install() {
        let in_progress = AtomicBool::new(false);
        let guard = AppUpdateInstallGuard::acquire(&in_progress).expect("first install admitted");

        assert!(matches!(
            AppUpdateInstallGuard::acquire(&in_progress),
            Err(AppError::Validation(_))
        ));
        assert!(AppUpdateInstallGuard::acquire(&in_progress).is_err());
        drop(guard);

        let successor =
            AppUpdateInstallGuard::acquire(&in_progress).expect("successor install admitted");
        assert!(AppUpdateInstallGuard::acquire(&in_progress).is_err());
        drop(successor);
        assert!(AppUpdateInstallGuard::acquire(&in_progress).is_ok());
    }

    #[test]
    fn install_error_releases_admission_for_retry() {
        fn failing_install(in_progress: &AtomicBool) -> Result<(), AppError> {
            let _guard = AppUpdateInstallGuard::acquire(in_progress)?;
            Err(AppError::Internal("simulated updater error".to_string()))
        }

        let in_progress = AtomicBool::new(false);
        assert!(failing_install(&in_progress).is_err());
        assert!(AppUpdateInstallGuard::acquire(&in_progress).is_ok());
    }

    #[test]
    fn cancelled_install_future_releases_admission_for_retry() {
        let in_progress = AtomicBool::new(false);
        let mut install = Box::pin(async {
            let _guard = AppUpdateInstallGuard::acquire(&in_progress).expect("install admitted");
            pending::<()>().await;
        });
        let mut context = Context::from_waker(Waker::noop());

        assert!(install.as_mut().poll(&mut context).is_pending());
        assert!(AppUpdateInstallGuard::acquire(&in_progress).is_err());
        drop(install);
        assert!(AppUpdateInstallGuard::acquire(&in_progress).is_ok());
    }
}
