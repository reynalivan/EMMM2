use crate::shared::errors::BrowserError;
use crate::shared::sync::lock;
use futures_util::StreamExt;
use reqwest::Client;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::modules::browser::application::browser::browser_service::{
    compute_download_path, compute_download_path_with, validate_http_url,
};
use crate::modules::browser::application::browser::download_service;

/// Accepted downloads and their FIFO scheduler state. A job joins this registry
/// before its row is persisted, but cannot start until persistence marks it ready.
static DOWNLOAD_REGISTRY: OnceLock<Mutex<DownloadRegistry>> = OnceLock::new();

/// Download requests awaiting a user decision. They are deliberately transient:
/// rejecting one must not create a database row or a file on disk.
static PENDING_DOWNLOADS: OnceLock<Mutex<HashMap<String, PendingDownload>>> = OnceLock::new();

static NEXT_QUEUE_ORDER: AtomicU64 = AtomicU64::new(1);

/// Max concurrent transfers.
const MAX_CONCURRENT_DOWNLOADS: usize = 5;

/// Maximum accepted jobs waiting for a transfer slot, in addition to active
/// transfers. This bounds memory, sockets waiting on the semaphore, and the
/// number of files a mistaken multi-click can schedule.
const MAX_QUEUED_DOWNLOADS: usize = 20;

/// Bound confirmations too: a hostile page must not consume memory or make an
/// arbitrarily long modal queue before the user makes a choice.
const MAX_PENDING_CONFIRMATIONS: usize = 20;

/// A confirmation is only valid while the browser page remains in the same
/// short-lived interaction. Expired confirmations cannot later start a stale URL.
const PENDING_CONFIRMATION_TTL: Duration = Duration::from_secs(5 * 60);

/// How often the progress event and DB row are refreshed mid-transfer.
const PROGRESS_EMIT_INTERVAL_MS: u128 = 100;

/// Write buffer for the streamed body. Without it every ~8-16 KB reqwest chunk
/// is its own `write` syscall.
const DOWNLOAD_BUFFER_BYTES: usize = 1 << 20;

/// Reused across downloads so the connection pool and TLS config survive a
/// multi-select from one host.
static HTTP_CLIENT: std::sync::OnceLock<Client> = std::sync::OnceLock::new();

struct ActiveDownload {
    cancel_flag: Arc<AtomicBool>,
    started: Arc<AtomicBool>,
    source_url: String,
    destination: PathBuf,
    ready: bool,
}

#[derive(Clone)]
struct DownloadJob {
    id: String,
    filename: String,
    source_url: String,
    destination: PathBuf,
    destination_path: String,
    queue_order: i64,
    client: Client,
    app: AppHandle,
    db: SqlitePool,
    cancel_flag: Arc<AtomicBool>,
}

#[derive(Default)]
struct DownloadRegistry {
    downloads: HashMap<String, ActiveDownload>,
    queue: VecDeque<DownloadJob>,
    active_count: usize,
    reserved_destinations: HashSet<PathBuf>,
}

struct PendingDownload {
    source_url: String,
    filename: String,
    downloads_root: PathBuf,
    session_id: Option<String>,
    requested_at: Instant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancelRequest {
    Queued,
    InProgress,
}

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

fn download_registry() -> &'static Mutex<DownloadRegistry> {
    DOWNLOAD_REGISTRY.get_or_init(|| Mutex::new(DownloadRegistry::default()))
}

fn pending_downloads() -> &'static Mutex<HashMap<String, PendingDownload>> {
    PENDING_DOWNLOADS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[allow(clippy::too_many_arguments)]
fn register_download(
    id: String,
    source_url: String,
    filename: String,
    downloads_root: &Path,
    session_id: Option<&str>,
    client: Client,
    app: AppHandle,
    db: SqlitePool,
) -> Result<DownloadJob, BrowserError> {
    let mut registry = lock(download_registry());
    if registry.downloads.len() >= MAX_CONCURRENT_DOWNLOADS + MAX_QUEUED_DOWNLOADS {
        return Err(BrowserError::QueueFull);
    }
    if registry
        .downloads
        .values()
        .any(|download| download.source_url == source_url)
    {
        return Err(BrowserError::DownloadAlreadyActive);
    }

    // Destination reservations prevent two jobs admitted in the same process
    // from selecting the same basename before either writes to disk. The final
    // tempfile persist remains a no-clobber guard against external changes.
    let destination = compute_download_path_with(downloads_root, session_id, &filename, |path| {
        !path.exists() && !registry.reserved_destinations.contains(path)
    });
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let started = Arc::new(AtomicBool::new(false));
    let queue_order = NEXT_QUEUE_ORDER.fetch_add(1, Ordering::Relaxed) as i64;
    let job = DownloadJob {
        id: id.clone(),
        filename,
        source_url: source_url.clone(),
        destination: destination.clone(),
        destination_path: destination.to_string_lossy().to_string(),
        queue_order,
        client,
        app,
        db,
        cancel_flag: cancel_flag.clone(),
    };

    registry.reserved_destinations.insert(destination.clone());
    registry.downloads.insert(
        id,
        ActiveDownload {
            cancel_flag,
            started,
            source_url,
            destination,
            ready: false,
        },
    );
    registry.queue.push_back(job.clone());
    Ok(job)
}

fn remove_download(id: &str, count_as_active: bool) {
    let mut registry = lock(download_registry());
    if let Some(download) = registry.downloads.remove(id) {
        registry.reserved_destinations.remove(&download.destination);
        registry.queue.retain(|job| job.id != id);
        if count_as_active && download.started.load(Ordering::Relaxed) {
            registry.active_count = registry.active_count.saturating_sub(1);
        }
    }
}

fn schedule_downloads() {
    let jobs = {
        let mut registry = lock(download_registry());
        let mut jobs = Vec::new();

        while registry.active_count < MAX_CONCURRENT_DOWNLOADS {
            let Some(next) = registry.queue.front() else {
                break;
            };
            let Some(active) = registry.downloads.get(&next.id) else {
                registry.queue.pop_front();
                continue;
            };
            if !active.ready {
                break;
            }

            let job = registry
                .queue
                .pop_front()
                .expect("front was checked before pop");
            if let Some(active) = registry.downloads.get(&job.id) {
                active.started.store(true, Ordering::Relaxed);
                registry.active_count += 1;
                jobs.push(job);
            }
        }

        jobs
    };

    for job in jobs {
        tauri::async_runtime::spawn(run_download(job));
    }
}

fn mark_download_ready(id: &str) -> bool {
    let marked_ready = {
        let mut registry = lock(download_registry());
        let Some(download) = registry.downloads.get_mut(id) else {
            return false;
        };
        download.ready = true;
        true
    };
    schedule_downloads();
    marked_ready
}

/// Request cancellation of an accepted download.
///
/// Queued jobs need their stored status updated immediately; active transfers
/// let their worker remove the partial file before it records cancellation.
pub fn request_cancel(id: &str) -> Option<CancelRequest> {
    let request = {
        let mut registry = lock(download_registry());
        let download = registry.downloads.get(id)?;
        download.cancel_flag.store(true, Ordering::Relaxed);

        if download.started.load(Ordering::Relaxed) {
            CancelRequest::InProgress
        } else {
            let destination = download.destination.clone();
            registry.downloads.remove(id);
            registry.reserved_destinations.remove(&destination);
            registry.queue.retain(|job| job.id != id);
            CancelRequest::Queued
        }
    };
    if request == CancelRequest::Queued {
        schedule_downloads();
    }
    Some(request)
}

/// Register a browser download request and ask the frontend for a user decision.
/// No transfer, DB row, directory, or file is created until `confirm_download`.
pub fn request_download_confirmation(
    app: &AppHandle,
    source_url: String,
    filename: String,
    downloads_root: PathBuf,
    session_id: Option<String>,
) -> Result<(), BrowserError> {
    validate_http_url(&source_url)?;

    let request_id = Uuid::new_v4().to_string();
    let destination_path = compute_download_path(&downloads_root, session_id.as_deref(), &filename)
        .to_string_lossy()
        .to_string();
    {
        let mut pending = lock(pending_downloads());
        pending.retain(|_, request| request.requested_at.elapsed() <= PENDING_CONFIRMATION_TTL);
        if pending.len() >= MAX_PENDING_CONFIRMATIONS {
            return Err(BrowserError::QueueFull);
        }
        pending.insert(
            request_id.clone(),
            PendingDownload {
                source_url: source_url.clone(),
                filename: filename.clone(),
                downloads_root,
                session_id,
                requested_at: Instant::now(),
            },
        );
    }

    if let Err(error) = app.emit(
        "browser:download-confirmation-requested",
        serde_json::json!({
            "id": request_id,
            "filename": filename,
            "source_url": source_url,
            "destination_path": destination_path,
        }),
    ) {
        lock(pending_downloads()).remove(&request_id);
        return Err(BrowserError::Download(format!(
            "failed to show download confirmation: {error}"
        )));
    }

    Ok(())
}

fn take_pending_download(request_id: &str) -> Result<PendingDownload, BrowserError> {
    let mut pending = lock(pending_downloads());
    pending.retain(|_, request| request.requested_at.elapsed() <= PENDING_CONFIRMATION_TTL);
    pending
        .remove(request_id)
        .ok_or(BrowserError::DownloadConfirmationUnavailable)
}

/// Start a previously confirmed request. Removing it from the registry first
/// makes duplicate confirmations harmless.
pub async fn confirm_download(
    app: AppHandle,
    db: SqlitePool,
    request_id: &str,
) -> Result<(), BrowserError> {
    let request = take_pending_download(request_id)?;
    validate_http_url(&request.source_url)?;

    std::fs::create_dir_all(&request.downloads_root)?;
    start_concurrent_download(
        app,
        db,
        request.source_url,
        request.filename,
        request.downloads_root,
        request.session_id,
    )
    .await
}

/// Reject a previously requested download. The only side effect is removing the
/// transient confirmation request.
pub fn reject_download(request_id: &str) -> Result<(), BrowserError> {
    take_pending_download(request_id).map(|_| ())
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
    downloads_root: PathBuf,
    session_id: Option<String>,
) -> Result<(), BrowserError> {
    validate_http_url(&url)?;
    let client = http_client()?.clone();

    let download_id = Uuid::new_v4().to_string();
    let job = register_download(
        download_id,
        url,
        filename,
        &downloads_root,
        session_id.as_deref(),
        client,
        app,
        db,
    )?;

    // Persist before the scheduler can run. If concurrent inserts finish in a
    // different order, the FIFO head remains blocked until its own row is ready.
    if let Err(error) = download_service::create_download_with_id(
        &job.db,
        &job.id,
        session_id.as_deref(),
        &job.filename,
        &job.source_url,
        &job.destination_path,
        job.queue_order,
    )
    .await
    {
        remove_download(&job.id, false);
        schedule_downloads();
        return Err(error);
    }

    let _ = job.app.emit(
        "browser:download-status",
        serde_json::json!({
            "id": job.id,
            "status": "requested",
        }),
    );

    mark_download_ready(&job.id);

    Ok(())
}

async fn run_download(job: DownloadJob) {
    let outcome = if job.cancel_flag.load(Ordering::Relaxed) {
        Ok(DownloadOutcome::Canceled)
    } else {
        perform_download(
            &job.client,
            &job.source_url,
            &job.destination,
            &job.id,
            &job.app,
            &job.db,
            &job.cancel_flag,
        )
        .await
    };

    match outcome {
        Ok(DownloadOutcome::Completed) => {
            if let Err(error) = download_service::mark_download_finished(
                &job.db,
                &job.app,
                &job.id,
                &job.destination_path,
            )
            .await
            {
                log::error!("Failed to persist completed download {}: {error}", job.id);
            }
        }
        Ok(DownloadOutcome::Canceled) => {
            log::info!("Download canceled by user: {}", job.filename);
            if let Err(error) = download_service::update_status(
                &job.db, &job.id, "canceled", None, None, None, None,
            )
            .await
            {
                log::error!("Failed to persist canceled download {}: {error}", job.id);
            }
            let _ = job.app.emit(
                "browser:download-status",
                serde_json::json!({ "id": job.id, "status": "canceled" }),
            );
        }
        Err(error) => {
            log::error!("Concurrent download failed for {}: {error}", job.filename);
            if let Err(status_error) = download_service::update_status(
                &job.db,
                &job.id,
                "failed",
                None,
                None,
                Some(&error.to_string()),
                None,
            )
            .await
            {
                log::error!(
                    "Failed to persist failed download {}: {status_error}",
                    job.id
                );
            }
            let _ = job.app.emit(
                "browser:download-status",
                serde_json::json!({ "id": job.id, "status": "failed" }),
            );
        }
    }

    // Keep source and destination reservations until terminal persistence has
    // completed so a retry cannot overlap this job's terminal state.
    remove_download(&job.id, true);
    schedule_downloads();
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

    // The status row is committed before the event so listeners that refetch
    // cannot observe a missing or stale download.
    let _ = app.emit(
        "browser:download-status",
        serde_json::json!({
            "id": download_id,
            "status": "in_progress",
        }),
    );

    let destination_dir = destination.parent().ok_or_else(|| {
        BrowserError::Download("download destination has no parent directory".into())
    })?;
    let mut temp_file = tempfile::Builder::new()
        .prefix(".emmm-download-")
        .suffix(".part")
        .tempfile_in(destination_dir)
        .map_err(|error| BrowserError::Download(format!("failed to create temp file: {error}")))?;
    let mut file =
        std::io::BufWriter::with_capacity(DOWNLOAD_BUFFER_BYTES, temp_file.as_file_mut());
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
    drop(file);

    // `persist_noclobber` is atomic and refuses to replace a file created by
    // another process after the in-memory reservation was made.
    temp_file.persist_noclobber(destination).map_err(|error| {
        BrowserError::Download(format!(
            "failed to finalize download without overwriting an existing file: {}",
            error.error
        ))
    })?;

    Ok(DownloadOutcome::Completed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn register_test_download(id: &str, source_url: &str) -> Arc<AtomicBool> {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let destination = PathBuf::from(format!("test-download-{id}"));
        let mut registry = lock(download_registry());
        registry.reserved_destinations.insert(destination.clone());
        registry.downloads.insert(
            id.to_string(),
            ActiveDownload {
                cancel_flag: cancel_flag.clone(),
                started: Arc::new(AtomicBool::new(false)),
                source_url: source_url.to_string(),
                destination,
                ready: false,
            },
        );
        cancel_flag
    }

    #[test]
    fn cancel_registry_flags_only_inflight_downloads() {
        assert!(request_cancel("unknown-id").is_none());

        let control = register_test_download("dl-1", "https://example.test/one");
        assert!(!control.load(Ordering::Relaxed));
        assert_eq!(request_cancel("dl-1"), Some(CancelRequest::Queued));
        assert!(control.load(Ordering::Relaxed));

        remove_download("dl-1", false);
        assert!(request_cancel("dl-1").is_none());
    }

    #[test]
    fn cancel_flag_is_per_download_not_global() {
        let a = register_test_download("dl-a", "https://example.test/a");
        let b = register_test_download("dl-b", "https://example.test/b");

        assert_eq!(request_cancel("dl-a"), Some(CancelRequest::Queued));

        assert!(a.load(Ordering::Relaxed));
        assert!(!b.load(Ordering::Relaxed));

        remove_download("dl-a", false);
        remove_download("dl-b", false);
    }

    #[test]
    fn queued_cancel_removes_the_registration_after_setting_its_flag() {
        let control = register_test_download("dl-repeat", "https://example.test/repeat");

        assert_eq!(request_cancel("dl-repeat"), Some(CancelRequest::Queued));
        assert!(request_cancel("dl-repeat").is_none());
        assert!(control.load(Ordering::Relaxed));

        remove_download("dl-repeat", false);
    }

    #[test]
    fn re_registering_an_id_resets_its_cancel_flag() {
        let first = register_test_download("dl-reuse", "https://example.test/first");
        assert_eq!(request_cancel("dl-reuse"), Some(CancelRequest::Queued));
        assert!(first.load(Ordering::Relaxed));

        // A new transfer under the same id gets a fresh, un-canceled flag;
        // the old handle keeps its value (the running task still sees `true`).
        remove_download("dl-reuse", false);
        let second = register_test_download("dl-reuse", "https://example.test/second");
        assert!(!second.load(Ordering::Relaxed));
        assert!(first.load(Ordering::Relaxed));

        remove_download("dl-reuse", false);
    }

    #[test]
    fn duplicate_source_url_is_rejected_before_a_second_job_is_queued() {
        let first = register_test_download("dl-first", "https://example.test/same");
        let registry = lock(download_registry());
        let duplicate_exists = registry
            .downloads
            .values()
            .any(|download| download.source_url == "https://example.test/same");
        drop(registry);

        assert!(duplicate_exists);
        assert!(!first.load(Ordering::Relaxed));
        remove_download("dl-first", false);
    }

    #[test]
    fn confirmation_request_can_only_be_consumed_once() {
        let request_id = "request-once";
        lock(pending_downloads()).insert(
            request_id.to_string(),
            PendingDownload {
                source_url: "https://example.test/file.zip".to_string(),
                filename: "file.zip".to_string(),
                downloads_root: PathBuf::from("C:/downloads"),
                session_id: None,
                requested_at: Instant::now(),
            },
        );

        assert!(take_pending_download(request_id).is_ok());
        assert!(matches!(
            take_pending_download(request_id),
            Err(BrowserError::DownloadConfirmationUnavailable)
        ));
    }
}
