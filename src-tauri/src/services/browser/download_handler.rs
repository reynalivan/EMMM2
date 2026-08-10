use crate::common::sync::lock;
use crate::domain::errors::BrowserError;
use futures_util::StreamExt;
use reqwest::Client;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;

use crate::services::browser::download_service;

/// In-flight downloads by id; the flag requests cancellation.
static ACTIVE_DOWNLOADS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();

/// Max concurrent transfers. Queueing beyond this keeps one slow host from
/// starving the rest and stops a multi-select from opening dozens of sockets.
/// ponytail: one global cap; make it per-host if a site starts rate-limiting.
static DOWNLOAD_SEMAPHORE: Semaphore = Semaphore::const_new(MAX_CONCURRENT_DOWNLOADS);

/// Max concurrent transfers.
const MAX_CONCURRENT_DOWNLOADS: usize = 3;

/// How often the progress event and DB row are refreshed mid-transfer.
const PROGRESS_EMIT_INTERVAL_MS: u128 = 100;

/// Write buffer for the streamed body. Without it every ~8-16 KB reqwest chunk
/// is its own `write` syscall.
const DOWNLOAD_BUFFER_BYTES: usize = 1 << 20;

/// Reused across downloads so the connection pool and TLS config survive a
/// multi-select from one host.
static HTTP_CLIENT: std::sync::OnceLock<Client> = std::sync::OnceLock::new();

fn http_client() -> Result<&'static Client, BrowserError> {
    if let Some(client) = HTTP_CLIENT.get() {
        return Ok(client);
    }
    let client = Client::builder()
        .user_agent(concat!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) EMMM/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|e| BrowserError::Download(format!("failed to create HTTP client: {e}")))?;
    Ok(HTTP_CLIENT.get_or_init(|| client))
}

fn active_downloads() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    ACTIVE_DOWNLOADS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_download(id: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    lock(active_downloads()).insert(id.to_string(), flag.clone());
    flag
}

fn unregister_download(id: &str) {
    lock(active_downloads()).remove(id);
}

/// Request cancellation of an in-flight download.
/// Returns true when the download was running; its task then aborts the
/// transfer, deletes the partial file, and marks the record `canceled`.
pub fn request_cancel(id: &str) -> bool {
    match lock(active_downloads()).get(id) {
        Some(flag) => {
            flag.store(true, Ordering::Relaxed);
            true
        }
        None => false,
    }
}

enum DownloadOutcome {
    Completed,
    Canceled,
}

/// Spawns an asynchronous download using reqwest.
/// This bypasses Tauri's blocking, sequential `on_download` queue.
pub async fn start_concurrent_download(
    app: AppHandle,
    db: SqlitePool,
    url: String,
    filename: String,
    destination: PathBuf,
    session_id: Option<String>,
) -> Result<(), BrowserError> {
    // 1. Create DB record first
    let dest_str = destination.to_string_lossy().to_string();
    let download_id = match download_service::create_download(
        &db,
        session_id.as_deref(),
        &filename,
        &url,
        &dest_str,
    )
    .await
    {
        Ok(id) => id,
        Err(error) => return Err(error),
    };

    let client = http_client()?.clone();

    let cancel_flag = register_download(&download_id);

    // 2. Start the download in a background task so we don't block. It waits for a
    //    `DOWNLOAD_SEMAPHORE` permit first; the record stays `requested` until then
    //    and stays cancellable, since it is already in the registry.
    tauri::async_runtime::spawn(async move {
        let outcome = match DOWNLOAD_SEMAPHORE.acquire().await {
            Ok(_permit) if cancel_flag.load(Ordering::Relaxed) => Ok(DownloadOutcome::Canceled),
            Ok(_permit) => {
                perform_download(
                    &client,
                    &url,
                    &destination,
                    &download_id,
                    &app,
                    &db,
                    &cancel_flag,
                )
                .await
            }
            Err(_) => Err(BrowserError::QueueClosed),
        };
        unregister_download(&download_id);

        match outcome {
            Ok(DownloadOutcome::Completed) => {
                let _ = download_service::on_download_finished(
                    &db,
                    &app,
                    &url,
                    Some(&dest_str),
                    true,
                    "background_downloader",
                )
                .await;
            }
            Ok(DownloadOutcome::Canceled) => {
                log::info!("Download canceled by user: {}", filename);
                if let Err(error) =
                    crate::services::fs_utils::recycle_bin::move_path_to_recycle_bin(&destination)
                {
                    log::warn!("Failed to move cancelled download to the Recycle Bin: {error}");
                }
                let _ = download_service::update_status(
                    &db,
                    &download_id,
                    "canceled",
                    None,
                    None,
                    None,
                    None,
                )
                .await;

                let _ = app.emit(
                    "browser:download-status",
                    serde_json::json!({
                        "id": download_id,
                        "status": "canceled",
                    }),
                );
            }
            Err(e) => {
                // Failure
                log::error!("Concurrent download failed for {}: {}", filename, e);
                let _ = download_service::update_status(
                    &db,
                    &download_id,
                    "failed",
                    None,
                    None,
                    Some(&e.to_string()),
                    None,
                )
                .await;

                let _ = app.emit(
                    "browser:download-status",
                    serde_json::json!({
                        "id": download_id,
                        "status": "failed",
                    }),
                );
            }
        }
    });

    Ok(())
}

async fn perform_download(
    client: &Client,
    url: &str,
    destination: &Path,
    download_id: &str,
    app: &AppHandle,
    db: &SqlitePool,
    cancel_flag: &AtomicBool,
) -> Result<DownloadOutcome, BrowserError> {
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| BrowserError::Download(format!("request failed: {e}")))?;

    if !res.status().is_success() {
        return Err(BrowserError::Download(format!(
            "server returned {}",
            res.status()
        )));
    }

    let total_size = res.content_length().unwrap_or(0);

    // Initial progress setup
    let _ = download_service::update_status(
        db,
        download_id,
        "in_progress",
        Some(0),
        Some(total_size as i64),
        None,
        None,
    )
    .await;

    let file = File::create(destination)?;
    let mut file = std::io::BufWriter::with_capacity(DOWNLOAD_BUFFER_BYTES, file);
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();
    let mut last_emit_time = std::time::Instant::now();

    while let Some(item) = stream.next().await {
        if cancel_flag.load(Ordering::Relaxed) {
            drop(file);
            return Ok(DownloadOutcome::Canceled);
        }

        let chunk = item.map_err(|e| BrowserError::Download(format!("chunk read failed: {e}")))?;
        file.write_all(&chunk)?;

        downloaded += chunk.len() as u64;

        // Throttle emissions to ~10 times per second to avoid completely destroying the IPC channel
        if last_emit_time.elapsed().as_millis() >= PROGRESS_EMIT_INTERVAL_MS {
            let _ = app.emit(
                "browser:download-progress",
                serde_json::json!({
                    "id": download_id,
                    "bytes_received": downloaded as i64,
                    "bytes_total": total_size as i64,
                }),
            );

            // Also update DB occasionally (maybe not every 100ms, but this is okay for now)
            let _ = download_service::update_status(
                db,
                download_id,
                "in_progress",
                Some(downloaded as i64),
                Some(total_size as i64),
                None,
                None,
            )
            .await;

            last_emit_time = std::time::Instant::now();
        }
    }

    // Explicit: `BufWriter`'s drop flush ignores errors, which would truncate
    // the file silently and report the download as complete.
    file.flush()?;

    Ok(DownloadOutcome::Completed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_registry_flags_only_inflight_downloads() {
        assert!(!request_cancel("unknown-id"));

        let flag = register_download("dl-1");
        assert!(!flag.load(Ordering::Relaxed));
        assert!(request_cancel("dl-1"));
        assert!(flag.load(Ordering::Relaxed));

        unregister_download("dl-1");
        assert!(!request_cancel("dl-1"));
    }

    #[test]
    fn cancel_flag_is_per_download_not_global() {
        let a = register_download("dl-a");
        let b = register_download("dl-b");

        assert!(request_cancel("dl-a"));

        assert!(a.load(Ordering::Relaxed));
        assert!(!b.load(Ordering::Relaxed));

        unregister_download("dl-a");
        unregister_download("dl-b");
    }

    #[test]
    fn repeated_cancel_requests_stay_true_while_registered() {
        let flag = register_download("dl-repeat");

        assert!(request_cancel("dl-repeat"));
        assert!(request_cancel("dl-repeat"));
        assert!(flag.load(Ordering::Relaxed));

        unregister_download("dl-repeat");
    }

    #[test]
    fn re_registering_an_id_resets_its_cancel_flag() {
        let first = register_download("dl-reuse");
        assert!(request_cancel("dl-reuse"));
        assert!(first.load(Ordering::Relaxed));

        // A new transfer under the same id gets a fresh, un-canceled flag;
        // the old handle keeps its value (the running task still sees `true`).
        let second = register_download("dl-reuse");
        assert!(!second.load(Ordering::Relaxed));
        assert!(first.load(Ordering::Relaxed));

        unregister_download("dl-reuse");
    }
}
