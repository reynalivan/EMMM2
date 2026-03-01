use log::{info, warn};
use reqwest::Client;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::time::Duration;

/// Remote manifest structure
#[derive(Debug, Deserialize)]
struct RemoteManifest {
    db_version: u64,
    /// URL to download the character database payload
    db_url: Option<String>,
}

/// Result of a metadata sync check
#[derive(Debug, serde::Serialize)]
pub struct MetadataSyncResult {
    pub updated: bool,
    pub version: Option<u64>,
}

/// Base URL for metadata files on GitHub CDN.
/// Format: raw.githubusercontent.com/{owner}/{repo}/{branch}/data/
const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/reynalivan/EMMM2/main/data/manifest.json";

/// Maximum number of retries on rate-limit (HTTP 429).
const MAX_RETRIES: u32 = 3;

/// Check remote manifest and sync metadata if a newer version is available.
///
/// This function is designed to be called at startup and fail silently on any
/// network error so it never blocks the app from launching.
pub async fn check_and_sync_metadata(pool: &SqlitePool) -> MetadataSyncResult {
    match try_sync(pool).await {
        Ok(result) => result,
        Err(e) => {
            warn!("Metadata sync skipped: {e}");
            MetadataSyncResult {
                updated: false,
                version: None,
            }
        }
    }
}

async fn try_sync(pool: &SqlitePool) -> Result<MetadataSyncResult, anyhow::Error> {
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

    // Read cached ETag / Last-Modified from DB
    let etag = get_meta(pool, "etag").await.unwrap_or_default();
    let last_modified = get_meta(pool, "last_modified").await.unwrap_or_default();

    // Conditional GET with retry logic for rate limiting
    let response = request_with_retry(&client, MANIFEST_URL, &etag, &last_modified).await?;

    // 304 Not Modified — nothing to do
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        info!("Metadata: up-to-date (304 Not Modified)");
        return Ok(MetadataSyncResult {
            updated: false,
            version: None,
        });
    }

    if !response.status().is_success() {
        anyhow::bail!("Manifest fetch failed: HTTP {}", response.status());
    }

    // Cache new ETag / Last-Modified headers
    if let Some(new_etag) = response.headers().get("etag") {
        if let Ok(val) = new_etag.to_str() {
            set_meta(pool, "etag", val).await;
        }
    }
    if let Some(new_lm) = response.headers().get("last-modified") {
        if let Ok(val) = new_lm.to_str() {
            set_meta(pool, "last_modified", val).await;
        }
    }

    let manifest: RemoteManifest = response.json().await?;
    let local_version: u64 = get_meta(pool, "metadata_version")
        .await
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);

    if manifest.db_version <= local_version {
        info!(
            "Metadata: already at version {} (remote: {})",
            local_version, manifest.db_version
        );
        return Ok(MetadataSyncResult {
            updated: false,
            version: Some(local_version),
        });
    }

    info!(
        "Metadata: updating {} -> {}",
        local_version, manifest.db_version
    );

    // Download the character DB payload if URL is provided
    if let Some(db_url) = &manifest.db_url {
        let db_response = request_with_retry(&client, db_url, "", "").await?;
        if db_response.status().is_success() {
            let payload: serde_json::Value = db_response.json().await?;
            // Store raw payload in app_meta for downstream consumers
            set_meta(pool, "metadata_payload", &payload.to_string()).await;
        }
    }

    // Update the local version marker
    set_meta(pool, "metadata_version", &manifest.db_version.to_string()).await;

    Ok(MetadataSyncResult {
        updated: true,
        version: Some(manifest.db_version),
    })
}

/// GET with conditional headers and exponential backoff on 429.
async fn request_with_retry(
    client: &Client,
    url: &str,
    etag: &str,
    last_modified: &str,
) -> Result<reqwest::Response, anyhow::Error> {
    let mut delay = Duration::from_millis(500);

    for attempt in 0..=MAX_RETRIES {
        let mut req = client.get(url);
        if !etag.is_empty() {
            req = req.header("If-None-Match", etag);
        }
        if !last_modified.is_empty() {
            req = req.header("If-Modified-Since", last_modified);
        }

        let response = req.send().await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            if attempt < MAX_RETRIES {
                warn!(
                    "Rate limited (429), retry {}/{} in {:?}",
                    attempt + 1,
                    MAX_RETRIES,
                    delay
                );
                tokio::time::sleep(delay).await;
                delay *= 2;
                continue;
            }
            anyhow::bail!("Rate limited after {} retries", MAX_RETRIES);
        }

        return Ok(response);
    }

    anyhow::bail!("Request failed after {} retries", MAX_RETRIES);
}

// ── DB helpers for app_meta key-value store ──

async fn get_meta(pool: &SqlitePool, key: &str) -> Option<String> {
    crate::database::settings_repo::get_app_meta(pool, key).await
}

async fn set_meta(pool: &SqlitePool, key: &str, value: &str) {
    crate::database::settings_repo::set_app_meta(pool, key, value).await;
}
