use futures_util::StreamExt;
use reqwest::Client;
use sqlx::SqlitePool;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

use crate::services::browser::download_service;

/// Spawns an asynchronous download using reqwest.
/// This bypasses Tauri's blocking, sequential `on_download` queue.
pub async fn start_concurrent_download(
    app: AppHandle,
    db: SqlitePool,
    url: String,
    filename: String,
    destination: PathBuf,
    session_id: Option<String>,
) -> Result<(), String> {
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
        Err(e) => return Err(format!("Failed to record download: {e}")),
    };

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) EMMM2/0.1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // 2. Start the download in a background task so we don't block
    tauri::async_runtime::spawn(async move {
        match perform_download(&client, &url, &destination, &download_id, &app, &db).await {
            Ok(_) => {
                // Success
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
            Err(e) => {
                // Failure
                log::error!("Concurrent download failed for {}: {}", filename, e);
                let _ = download_service::update_status(
                    &db,
                    &download_id,
                    "failed",
                    None,
                    None,
                    Some(&e),
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
) -> Result<(), String> {
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("Server returned error: {}", res.status()));
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

    let mut file = File::create(destination).map_err(|e| format!("Failed to create file: {e}"))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();
    let mut last_emit_time = std::time::Instant::now();

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| format!("Error while reading chunk: {e}"))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Error writing to file: {e}"))?;

        downloaded += chunk.len() as u64;

        // Throttle emissions to ~10 times per second to avoid completely destroying the IPC channel
        if last_emit_time.elapsed().as_millis() >= 100 {
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

    Ok(())
}
