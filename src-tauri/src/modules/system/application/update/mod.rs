//! Remote metadata and asset sync from the project's GitHub CDN.

pub mod asset_fetch;
pub mod metadata_sync;

use std::time::Duration;

/// Root of the published data on the GitHub CDN. Both submodules resolve their
/// URLs from here so a repo or branch move is a single edit.
pub(super) const CDN_BASE_URL: &str = "https://raw.githubusercontent.com/reynalivan/EMMM/main/";

/// Metadata is a small JSON document; assets can be images.
pub(super) const MANIFEST_TIMEOUT: Duration = Duration::from_secs(10);
pub(super) const ASSET_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn http_client(timeout: Duration) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder().timeout(timeout).build()
}

#[cfg(test)]
#[path = "tests/update_service_tests.rs"]
mod tests;
