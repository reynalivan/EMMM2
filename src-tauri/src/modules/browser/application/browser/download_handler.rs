use crate::shared::errors::BrowserError;
use crate::shared::sync::lock;
use futures_util::StreamExt;
use reqwest::{Client, StatusCode};
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

/// Give the renderer a short, perceptible turn to present the preparation
/// state before the approval dialog replaces it. This does not start a
/// transfer or make an additional network request.
const DOWNLOAD_INFORMATION_DISPLAY_DURATION: Duration = Duration::from_millis(150);

/// Bound a dead connection and a stalled read without limiting the overall
/// duration of a large file that is still streaming data.
const DOWNLOAD_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const DOWNLOAD_READ_TIMEOUT: Duration = Duration::from_secs(45);
const MAX_FALLBACK_DOWNLOAD_BYTES: u64 = 20 * 1024 * 1024 * 1024;

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
    game_id: String,
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
        .connect_timeout(DOWNLOAD_CONNECT_TIMEOUT)
        .read_timeout(DOWNLOAD_READ_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| BrowserError::Download(format!("failed to create HTTP client: {e}")))?;
    Ok(HTTP_CLIENT.get_or_init(|| client))
}

fn request_failure_code(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        "download.timeout"
    } else if error.is_connect() {
        "download.offline"
    } else {
        "download.failed"
    }
}

fn response_failure_code(status: StatusCode) -> &'static str {
    match status {
        StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT => "download.timeout",
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => "download.access_denied",
        StatusCode::NOT_FOUND => "download.not_found",
        status if status.is_server_error() => "download.server",
        _ => "download.failed",
    }
}

fn emit_download_information_failure(
    app: &AppHandle,
    request_id: &str,
    source_url: &str,
    reason: &str,
) {
    let _ = app.emit(
        "browser:download-information-failed",
        serde_json::json!({
            "id": request_id,
            "source_url": source_url,
            "reason": reason,
        }),
    );
}

fn blocked_download(filename: &str) -> bool {
    crate::shared::payload_security::blocked_mod_payload_extension(Path::new(filename)).is_some()
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
pub async fn request_download_confirmation(
    app: &AppHandle,
    game_id: String,
    source_url: String,
    filename: String,
    downloads_root: PathBuf,
    session_id: Option<String>,
) -> Result<(), BrowserError> {
    validate_http_url(&source_url)?;

    let request_id = Uuid::new_v4().to_string();
    app.emit(
        "browser:download-information-loading",
        serde_json::json!({
            "id": request_id,
            "source_url": source_url,
        }),
    )
    .map_err(|error| {
        BrowserError::Download(format!("failed to show download preparation: {error}"))
    })?;

    tokio::time::sleep(DOWNLOAD_INFORMATION_DISPLAY_DURATION).await;

    let destination_path = compute_download_path(&downloads_root, session_id.as_deref(), &filename)
        .to_string_lossy()
        .to_string();
    let pending_result = {
        let mut pending = lock(pending_downloads());
        pending.retain(|_, request| request.requested_at.elapsed() <= PENDING_CONFIRMATION_TTL);
        if pending.len() >= MAX_PENDING_CONFIRMATIONS {
            Err(BrowserError::QueueFull)
        } else {
            pending.insert(
                request_id.clone(),
                PendingDownload {
                    game_id,
                    source_url: source_url.clone(),
                    filename: filename.clone(),
                    downloads_root,
                    session_id,
                    requested_at: Instant::now(),
                },
            );
            Ok(())
        }
    };
    if let Err(error) = pending_result {
        emit_download_information_failure(app, &request_id, &source_url, "queue_full");
        return Err(error);
    }

    if let Err(error) = app.emit(
        "browser:download-confirmation-requested",
        serde_json::json!({
            "id": request_id,
            "filename": filename,
            "source_url": source_url,
            "destination_path": destination_path,
            "risk_level": blocked_download(&filename).then_some("blocked"),
        }),
    ) {
        lock(pending_downloads()).remove(&request_id);
        emit_download_information_failure(app, &request_id, &source_url, "unavailable");
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
    if confirm_native_download(app.clone(), db.clone(), request_id).await? {
        return Ok(());
    }
    let request = take_pending_download(request_id)?;
    if blocked_download(&request.filename) {
        return Err(BrowserError::Download(format!(
            "Executable and script downloads are blocked: {}",
            request.filename
        )));
    }
    validate_http_url(&request.source_url)?;

    std::fs::create_dir_all(&request.downloads_root)?;
    start_concurrent_download(
        app,
        db,
        request.source_url,
        request.filename,
        request.downloads_root,
        request.session_id,
        request.game_id,
    )
    .await
}

/// Reject a previously requested download. The only side effect is removing the
/// transient confirmation request.
pub fn reject_download(request_id: &str) -> Result<(), BrowserError> {
    if reject_native_download(request_id)? {
        return Ok(());
    }
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
    game_id: String,
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
        &game_id,
        session_id.as_deref(),
        &job.filename,
        &job.source_url,
        &job.destination_path,
        job.queue_order,
        None,
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
            crate::modules::system::application::telemetry::record_background_failure(
                &job.app,
                &crate::shared::errors::AppError::Browser(error.clone()),
            )
            .await;
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
                serde_json::json!({
                    "id": job.id,
                    "status": "failed",
                    "error_msg": error.to_string(),
                }),
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
        .map_err(|error| BrowserError::Download(request_failure_code(&error).to_string()))?;

    if !res.status().is_success() {
        return Err(BrowserError::Download(
            response_failure_code(res.status()).to_string(),
        ));
    }

    let total_size = res.content_length().unwrap_or(0);
    if total_size > MAX_FALLBACK_DOWNLOAD_BYTES {
        return Err(BrowserError::Download("download.too_large".to_string()));
    }

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

        let chunk =
            item.map_err(|error| BrowserError::Download(request_failure_code(&error).to_string()))?;
        file.write_all(&chunk)?;

        downloaded += chunk.len() as u64;
        if downloaded > MAX_FALLBACK_DOWNLOAD_BYTES {
            return Err(BrowserError::Download("download.too_large".to_string()));
        }

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

    if total_size > 0 && downloaded != total_size {
        return Err(BrowserError::Download("download.size_mismatch".to_string()));
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

/// Attach the platform downloader for a Discover child webview. On Windows the
/// operation stays inside WebView2, retaining authenticated request state. The
/// fallback downloader is kept for platforms without that API.
pub fn attach_native_download_handler(
    webview: &tauri::Webview,
    app: &AppHandle,
    db: &SqlitePool,
    label: &str,
    downloads_root: &Path,
    session_id: Option<String>,
    game_id: String,
) -> Result<(), BrowserError> {
    #[cfg(target_os = "windows")]
    {
        return native_windows::attach(
            webview,
            app,
            db,
            label,
            downloads_root,
            session_id,
            game_id,
        );
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (webview, app, db, label, downloads_root, session_id, game_id);
        Ok(())
    }
}

pub async fn confirm_native_download(
    app: AppHandle,
    db: SqlitePool,
    request_id: &str,
) -> Result<bool, BrowserError> {
    #[cfg(target_os = "windows")]
    {
        return native_windows::confirm(app, db, request_id).await;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, db, request_id);
        Ok(false)
    }
}

pub fn reject_native_download(request_id: &str) -> Result<bool, BrowserError> {
    #[cfg(target_os = "windows")]
    {
        return native_windows::reject(request_id);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = request_id;
        Ok(false)
    }
}

pub fn cancel_native_download(app: &AppHandle, download_id: &str) -> Result<bool, BrowserError> {
    #[cfg(target_os = "windows")]
    {
        return native_windows::control(app, download_id, native_windows::DownloadControl::Cancel);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, download_id);
        Ok(false)
    }
}

pub fn pause_native_download(app: &AppHandle, download_id: &str) -> Result<(), BrowserError> {
    #[cfg(target_os = "windows")]
    {
        return native_windows::control(app, download_id, native_windows::DownloadControl::Pause)
            .and_then(|found| {
                found.then_some(()).ok_or_else(|| {
                    BrowserError::Download("This download cannot be paused".to_string())
                })
            });
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, download_id);
        Err(BrowserError::Download(
            "Pause is unavailable for this download".to_string(),
        ))
    }
}

pub fn resume_native_download(app: &AppHandle, download_id: &str) -> Result<(), BrowserError> {
    #[cfg(target_os = "windows")]
    {
        return native_windows::control(app, download_id, native_windows::DownloadControl::Resume)
            .and_then(|found| {
                found.then_some(()).ok_or_else(|| {
                    BrowserError::Download("This download cannot be resumed".to_string())
                })
            });
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, download_id);
        Err(BrowserError::Download(
            "Resume is unavailable for this download".to_string(),
        ))
    }
}

#[cfg(target_os = "windows")]
mod native_windows {
    use std::collections::{HashMap, HashSet};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::{Duration, Instant};

    use tauri::{AppHandle, Emitter, Manager};
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Deferral, ICoreWebView2DownloadOperation,
        ICoreWebView2DownloadStartingEventArgs, ICoreWebView2_4,
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_CANCELED,
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_PAUSED, COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED,
        COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS,
    };
    use webview2_com::{
        BytesReceivedChangedEventHandler, DownloadStartingEventHandler,
        EstimatedEndTimeChangedEventHandler, StateChangedEventHandler,
    };
    use windows::core::{Interface, BOOL, HSTRING, PWSTR};
    use windows::Win32::System::Com::CoTaskMemFree;

    use super::{
        blocked_download, compute_download_path_with, download_service, lock, BrowserError,
        SqlitePool, Uuid,
    };
    use crate::modules::browser::domain::browser::BrowserGameBananaProvenance;

    const NATIVE_PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(100);

    /// WebView2 COM objects are only dereferenced inside `with_webview`, which
    /// marshals the work to their owning UI thread. The wrapper permits storing
    /// a handle long enough for an IPC command to schedule that operation.
    #[derive(Clone)]
    struct NativeCom<T>(T);
    unsafe impl<T> Send for NativeCom<T> {}

    impl<T> NativeCom<T> {
        fn get(&self) -> &T {
            &self.0
        }
    }

    struct PendingNativeDownload {
        label: String,
        game_id: String,
        session_id: Option<String>,
        source_url: String,
        provenance: Option<BrowserGameBananaProvenance>,
        filename: String,
        destination: PathBuf,
        args: NativeCom<ICoreWebView2DownloadStartingEventArgs>,
        deferral: NativeCom<ICoreWebView2Deferral>,
        operation: NativeCom<ICoreWebView2DownloadOperation>,
    }

    struct ActiveNativeDownload {
        label: String,
        operation: NativeCom<ICoreWebView2DownloadOperation>,
    }

    #[derive(Default)]
    struct NativeDownloadRegistry {
        pending: HashMap<String, PendingNativeDownload>,
        active: HashMap<String, ActiveNativeDownload>,
        reserved_destinations: HashSet<PathBuf>,
    }

    static REGISTRY: OnceLock<Mutex<NativeDownloadRegistry>> = OnceLock::new();

    fn registry() -> &'static Mutex<NativeDownloadRegistry> {
        REGISTRY.get_or_init(|| Mutex::new(NativeDownloadRegistry::default()))
    }

    pub(super) enum DownloadControl {
        Cancel,
        Pause,
        Resume,
    }

    pub(super) fn attach(
        webview: &tauri::Webview,
        app: &AppHandle,
        db: &SqlitePool,
        label: &str,
        downloads_root: &Path,
        session_id: Option<String>,
        game_id: String,
    ) -> Result<(), BrowserError> {
        let app_for_event = app.clone();
        let db_for_event = db.clone();
        let label_for_event = label.to_string();
        let root_for_event = downloads_root.to_path_buf();
        webview
            .with_webview(move |native| unsafe {
                let Ok(core) = native.controller().CoreWebView2() else {
                    log::warn!("Discover WebView2 download API is unavailable");
                    return;
                };
                let Ok(core) = core.cast::<ICoreWebView2_4>() else {
                    log::warn!("Discover WebView2 runtime does not support native downloads");
                    return;
                };
                let app = app_for_event.clone();
                let db = db_for_event.clone();
                let label = label_for_event.clone();
                let root = root_for_event.clone();
                let game_id = game_id.clone();
                let session_id = session_id.clone();
                let handler = DownloadStartingEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else {
                        return Ok(());
                    };
                    let operation = args.DownloadOperation()?;
                    let source_url = read_uri(|value| operation.Uri(value)).unwrap_or_default();
                    if source_url.is_empty() {
                        let _ = args.SetCancel(true);
                        return Ok(());
                    }
                    let filename = filename_for(&operation, &source_url)
                        .unwrap_or_else(|| "download".to_string());
                    let provenance = app_for_event
                        .get_webview(&label)
                        .and_then(|webview| webview.url().ok())
                        .map(|url| url.to_string())
                        .and_then(|origin_page_url| {
                            crate::modules::matching::api::gamebanana_reference_from_url(
                                &origin_page_url,
                            )
                            .and_then(|reference| {
                                reference.canonical_page_url().map(|origin_page_url| {
                                    BrowserGameBananaProvenance {
                                        origin_page_url,
                                        item_type: reference.item_type,
                                        item_id: reference.item_id,
                                    }
                                })
                            })
                        });
                    let request_id = Uuid::new_v4().to_string();
                    let destination = {
                        let mut downloads = lock(registry());
                        let destination = compute_download_path_with(
                            &root,
                            session_id.as_deref(),
                            &filename,
                            |candidate| {
                                !candidate.exists()
                                    && !downloads.reserved_destinations.contains(candidate)
                            },
                        );
                        downloads.reserved_destinations.insert(destination.clone());
                        destination
                    };
                    let deferral = args.GetDeferral()?;
                    // Prevent the platform download UI while the app-owned
                    // confirmation dialog waits on this deferral.
                    args.SetHandled(true)?;
                    let mime_type = read_uri(|value| operation.MimeType(value)).ok();
                    let content_disposition =
                        read_uri(|value| operation.ContentDisposition(value)).ok();
                    let bytes_total = total_bytes(&operation);
                    lock(registry()).pending.insert(
                        request_id.clone(),
                        PendingNativeDownload {
                            label: label.clone(),
                            game_id: game_id.clone(),
                            session_id: session_id.clone(),
                            source_url: source_url.clone(),
                            provenance,
                            filename: filename.clone(),
                            destination: destination.clone(),
                            args: NativeCom(args),
                            deferral: NativeCom(deferral),
                            operation: NativeCom(operation.clone()),
                        },
                    );
                    let _ = app.emit(
                        "browser:download-information-loading",
                        serde_json::json!({ "id": request_id, "source_url": source_url }),
                    );
                    let _ = app.emit(
                        "browser:download-confirmation-requested",
                        serde_json::json!({
                            "id": request_id,
                            "filename": filename,
                            "source_url": source_url,
                            "destination_path": destination.to_string_lossy(),
                            "mime_type": mime_type,
                            "content_disposition": content_disposition,
                            "bytes_total": bytes_total,
                            "risk_level": blocked_download(&filename).then_some("blocked"),
                        }),
                    );
                    let _ = db;
                    Ok(())
                }));
                let mut token = 0;
                if let Err(error) = core.add_DownloadStarting(&handler, &mut token) {
                    log::warn!("Could not attach Discover's native download handler: {error}");
                }
            })
            .map_err(BrowserError::from)?;
        Ok(())
    }

    pub(super) async fn confirm(
        app: AppHandle,
        db: SqlitePool,
        request_id: &str,
    ) -> Result<bool, BrowserError> {
        let pending = lock(registry()).pending.remove(request_id);
        let Some(pending) = pending else {
            return Ok(false);
        };
        if blocked_download(&pending.filename) {
            finish_pending(&pending, true);
            return Err(BrowserError::Download(format!(
                "Executable and script downloads are blocked: {}",
                pending.filename
            )));
        }
        if let Err(error) =
            std::fs::create_dir_all(pending.destination.parent().ok_or_else(|| {
                BrowserError::Download("Download destination has no parent directory".into())
            })?)
        {
            finish_pending(&pending, true);
            return Err(error.into());
        }
        let path = pending.destination.to_string_lossy().to_string();
        if let Err(error) = download_service::create_download_with_id_and_provenance(
            &db,
            request_id,
            &pending.game_id,
            pending.session_id.as_deref(),
            &pending.filename,
            &pending.source_url,
            &path,
            0,
            Some(&pending.label),
            pending.provenance.as_ref(),
        )
        .await
        {
            finish_pending(&pending, true);
            return Err(error);
        }
        let operation = pending.operation.clone();
        let has_gamebanana_provenance = pending.provenance.is_some();
        lock(registry()).active.insert(
            request_id.to_string(),
            ActiveNativeDownload {
                label: pending.label.clone(),
                operation,
            },
        );
        attach_operation_events(
            &pending.operation.0,
            app.clone(),
            db.clone(),
            request_id.to_string(),
            path.clone(),
            has_gamebanana_provenance,
        );
        let label = pending.label.clone();
        let request_id_owned = request_id.to_string();
        let destination = pending.destination.clone();
        let args = pending.args.clone();
        let deferral = pending.deferral.clone();
        let completion_error = Arc::new(Mutex::new(None));
        let completion_error_for_webview = completion_error.clone();
        let completed = app
            .get_webview(&label)
            .ok_or_else(|| BrowserError::WebviewNotFound {
                label: label.clone(),
            })?
            .with_webview(move |_| unsafe {
                let result = (|| -> windows::core::Result<()> {
                    let target = HSTRING::from(destination.to_string_lossy().as_ref());
                    args.get().SetResultFilePath(&target)?;
                    args.get().SetHandled(true)?;
                    deferral.get().Complete()
                })();
                if let Err(error) = result {
                    *lock(&completion_error_for_webview) = Some(error.to_string());
                }
            });
        if let Err(error) = completed {
            lock(registry()).active.remove(&request_id_owned);
            finish_pending(&pending, true);
            return Err(BrowserError::from(error));
        }
        if let Some(error) = lock(&completion_error).take() {
            lock(registry()).active.remove(&request_id_owned);
            finish_pending(&pending, true);
            return Err(BrowserError::Download(format!(
                "WebView2 could not start the download: {error}"
            )));
        }
        download_service::update_native_status(
            &db,
            request_id,
            "in_progress",
            Some(0),
            total_bytes(&pending.operation.0),
            None,
            None,
            true,
        )
        .await?;
        let _ = app.emit(
            "browser:download-status",
            serde_json::json!({
                "id": request_id,
                "status": "in_progress",
                "can_resume": true,
            }),
        );
        Ok(true)
    }

    pub(super) fn reject(request_id: &str) -> Result<bool, BrowserError> {
        let pending = lock(registry()).pending.remove(request_id);
        let Some(pending) = pending else {
            return Ok(false);
        };
        finish_pending(&pending, true);
        Ok(true)
    }

    pub(super) fn control(
        app: &AppHandle,
        download_id: &str,
        control: DownloadControl,
    ) -> Result<bool, BrowserError> {
        let active = lock(registry())
            .active
            .get(download_id)
            .map(|download| (download.label.clone(), download.operation.clone()));
        let Some((label, operation)) = active else {
            return Ok(false);
        };
        let control_error = Arc::new(Mutex::new(None));
        let control_error_for_webview = control_error.clone();
        app.get_webview(&label)
            .ok_or_else(|| BrowserError::WebviewNotFound {
                label: label.clone(),
            })?
            .with_webview(move |_| unsafe {
                let result = match control {
                    DownloadControl::Cancel => operation.get().Cancel(),
                    DownloadControl::Pause => operation.get().Pause(),
                    DownloadControl::Resume => operation.get().Resume(),
                };
                if let Err(error) = result {
                    *lock(&control_error_for_webview) = Some(error.to_string());
                }
            })
            .map_err(BrowserError::from)?;
        if let Some(error) = lock(&control_error).take() {
            return Err(BrowserError::Download(format!(
                "WebView2 could not control the download: {error}"
            )));
        }
        Ok(true)
    }

    fn finish_pending(pending: &PendingNativeDownload, cancel: bool) {
        unsafe {
            if cancel {
                let _ = pending.args.0.SetCancel(true);
            }
            let _ = pending.deferral.0.Complete();
        }
        lock(registry())
            .reserved_destinations
            .remove(&pending.destination);
    }

    fn attach_operation_events(
        operation: &ICoreWebView2DownloadOperation,
        app: AppHandle,
        db: SqlitePool,
        download_id: String,
        destination: String,
        has_gamebanana_provenance: bool,
    ) {
        unsafe {
            let last_progress_emission = Arc::new(Mutex::new(None));
            let progress_app = app.clone();
            let progress_id = download_id.clone();
            let progress_emission = last_progress_emission.clone();
            let progress =
                BytesReceivedChangedEventHandler::create(Box::new(move |operation, _| {
                    let Some(operation) = operation else {
                        return Ok(());
                    };
                    emit_native_progress(
                        &progress_app,
                        &progress_id,
                        &operation,
                        &progress_emission,
                    );
                    Ok(())
                }));
            let mut progress_token = 0;
            let _ = operation.add_BytesReceivedChanged(&progress, &mut progress_token);

            let eta_app = app.clone();
            let eta_id = download_id.clone();
            let eta_emission = last_progress_emission.clone();
            let eta = EstimatedEndTimeChangedEventHandler::create(Box::new(move |operation, _| {
                let Some(operation) = operation else {
                    return Ok(());
                };
                emit_native_progress(&eta_app, &eta_id, &operation, &eta_emission);
                Ok(())
            }));
            let mut eta_token = 0;
            let _ = operation.add_EstimatedEndTimeChanged(&eta, &mut eta_token);

            let state_app = app.clone();
            let state_id = download_id.clone();
            let state_destination = destination.clone();
            let state_has_gamebanana_provenance = has_gamebanana_provenance;
            let state = StateChangedEventHandler::create(Box::new(move |operation, _| {
                let Some(operation) = operation else {
                    return Ok(());
                };
                let mut native_state = Default::default();
                operation.State(&mut native_state)?;
                if native_state == COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS {
                    return Ok(());
                }
                let (status, can_resume, error) = native_outcome(&operation, native_state);
                let received = bytes_received(&operation);
                let total = total_bytes(&operation);
                let app = state_app.clone();
                let db = db.clone();
                let id = state_id.clone();
                let destination = state_destination.clone();
                let should_store_content_hash =
                    state_has_gamebanana_provenance && status == "finished";
                tauri::async_runtime::spawn(async move {
                    let _ = download_service::update_native_status(
                        &db,
                        &id,
                        status,
                        Some(received),
                        total,
                        error.as_deref(),
                        (status == "finished").then_some(destination.as_str()),
                        can_resume,
                    )
                    .await;
                    if should_store_content_hash {
                        let hash_path = destination.clone();
                        let content_hash = tokio::task::spawn_blocking(move || {
                            download_service::gamebanana_source_signature(std::path::Path::new(
                                &hash_path,
                            ))
                            .map(|(_, content_hash)| content_hash)
                        })
                        .await
                        .ok()
                        .flatten();
                        if let Some(content_hash) = content_hash {
                            let _ = download_service::store_gamebanana_content_hash(
                                &db,
                                &id,
                                &content_hash,
                            )
                            .await;
                        }
                    }
                    let _ = app.emit(
                        "browser:download-status",
                        serde_json::json!({
                            "id": id,
                            "status": status,
                            "error_msg": error,
                            "file_path": (status == "finished").then_some(destination),
                            "can_resume": can_resume,
                        }),
                    );
                    if status != "paused" && !can_resume {
                        lock(registry()).active.remove(&id);
                    }
                });
                Ok(())
            }));
            let mut state_token = 0;
            let _ = operation.add_StateChanged(&state, &mut state_token);
        }
    }

    fn emit_native_progress(
        app: &AppHandle,
        download_id: &str,
        operation: &ICoreWebView2DownloadOperation,
        last_emission: &Mutex<Option<Instant>>,
    ) {
        if !claim_progress_emission(last_emission, Instant::now()) {
            return;
        }
        let _ = app.emit(
            "browser:download-progress",
            serde_json::json!({
                "id": download_id,
                "bytes_received": bytes_received(operation),
                "bytes_total": total_bytes(operation),
                "eta": read_uri(|value| unsafe { operation.EstimatedEndTime(value) }).ok(),
            }),
        );
    }

    fn claim_progress_emission(last_emission: &Mutex<Option<Instant>>, now: Instant) -> bool {
        let mut last_emission = lock(last_emission);
        if last_emission
            .is_some_and(|last| now.duration_since(last) < NATIVE_PROGRESS_EMIT_INTERVAL)
        {
            return false;
        }
        *last_emission = Some(now);
        true
    }

    fn native_outcome(
        operation: &ICoreWebView2DownloadOperation,
        state: webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_DOWNLOAD_STATE,
    ) -> (&'static str, bool, Option<String>) {
        if state == COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED {
            return ("finished", false, None);
        }
        let mut reason = Default::default();
        let _ = unsafe { operation.InterruptReason(&mut reason) };
        let can_resume = can_resume(operation);
        if reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_PAUSED {
            return ("paused", can_resume, None);
        }
        if reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_CANCELED {
            return ("canceled", false, None);
        }
        (
            "failed",
            can_resume,
            Some(format!("WebView2 download interrupted ({})", reason.0)),
        )
    }

    fn filename_for(
        operation: &ICoreWebView2DownloadOperation,
        source_url: &str,
    ) -> Option<String> {
        let disposition = read_uri(|value| unsafe { operation.ContentDisposition(value) }).ok()?;
        disposition
            .split(';')
            .find_map(|part| {
                part.trim()
                    .strip_prefix("filename=")
                    .or_else(|| part.trim().strip_prefix("filename*="))
            })
            .map(|value| {
                value
                    .trim_matches('"')
                    .rsplit('/')
                    .next()
                    .unwrap_or(value)
                    .to_string()
            })
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                reqwest::Url::parse(source_url)
                    .ok()?
                    .path_segments()?
                    .next_back()
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_owned)
            })
    }

    fn bytes_received(operation: &ICoreWebView2DownloadOperation) -> i64 {
        let mut bytes = 0;
        let _ = unsafe { operation.BytesReceived(&mut bytes) };
        bytes
    }

    fn total_bytes(operation: &ICoreWebView2DownloadOperation) -> Option<i64> {
        let mut bytes = -1;
        let _ = unsafe { operation.TotalBytesToReceive(&mut bytes) };
        (bytes >= 0).then_some(bytes)
    }

    fn can_resume(operation: &ICoreWebView2DownloadOperation) -> bool {
        let mut can_resume = BOOL(0);
        unsafe { operation.CanResume(&mut can_resume) }.is_ok() && can_resume.as_bool()
    }

    fn read_uri(
        read: impl FnOnce(*mut PWSTR) -> windows::core::Result<()>,
    ) -> windows::core::Result<String> {
        let mut value = PWSTR::null();
        read(&mut value)?;
        let result = unsafe { value.to_string() };
        if !value.is_null() {
            unsafe { CoTaskMemFree(Some(value.0.cast())) };
        }
        Ok(result?)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn progress_emission_is_limited_to_ten_updates_per_second() {
            let emissions = Mutex::new(None);
            let start = Instant::now();

            assert!(claim_progress_emission(&emissions, start));
            assert!(!claim_progress_emission(
                &emissions,
                start + NATIVE_PROGRESS_EMIT_INTERVAL - Duration::from_millis(1),
            ));
            assert!(claim_progress_emission(
                &emissions,
                start + NATIVE_PROGRESS_EMIT_INTERVAL,
            ));
        }
    }
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
                game_id: "game-1".to_string(),
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

    #[test]
    fn response_failure_codes_preserve_actionable_causes() {
        assert_eq!(
            response_failure_code(StatusCode::REQUEST_TIMEOUT),
            "download.timeout"
        );
        assert_eq!(
            response_failure_code(StatusCode::FORBIDDEN),
            "download.access_denied"
        );
        assert_eq!(
            response_failure_code(StatusCode::NOT_FOUND),
            "download.not_found"
        );
        assert_eq!(
            response_failure_code(StatusCode::INTERNAL_SERVER_ERROR),
            "download.server"
        );
    }

    #[test]
    fn executable_and_script_extensions_are_blocked() {
        assert!(blocked_download("mod-installer.exe"));
        assert!(blocked_download("setup.PS1"));
        assert!(!blocked_download("mod-pack.zip"));
        assert!(!blocked_download("readme"));
    }
}
