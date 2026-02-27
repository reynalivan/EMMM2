use log::{info, warn};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Base URL for asset files on GitHub CDN.
const ASSET_BASE_URL: &str = "https://raw.githubusercontent.com/reynalivan/EMMM2/main/assets/";

/// Fetch an asset file if it's not already cached locally.
///
/// Returns `Some(path)` if the asset is available (cached or freshly downloaded),
/// or `None` if the download failed.
pub async fn fetch_asset_if_missing(asset_name: &str, cache_dir: &Path) -> Option<PathBuf> {
    let assets_dir = cache_dir.join("assets");
    let local_path = assets_dir.join(asset_name);

    // Already cached
    if local_path.exists() {
        return Some(local_path);
    }

    // Ensure cache directory exists
    if let Err(e) = tokio::fs::create_dir_all(&assets_dir).await {
        warn!("Failed to create asset cache dir: {e}");
        return None;
    }

    let url = format!("{}{}", ASSET_BASE_URL, urlencoding::encode(asset_name));
    info!("Fetching missing asset: {}", url);

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .ok()?;

    let response = client.get(&url).send().await.ok()?;

    if !response.status().is_success() {
        warn!(
            "Asset fetch failed for '{}': HTTP {}",
            asset_name,
            response.status()
        );
        return None;
    }

    let bytes = response.bytes().await.ok()?;

    if let Err(e) = tokio::fs::write(&local_path, &bytes).await {
        warn!("Failed to cache asset '{}': {e}", asset_name);
        return None;
    }

    info!("Asset cached: {}", local_path.display());
    Some(local_path)
}
