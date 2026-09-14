//! Reviewed, manual imports for data-only catalog packs.
//!
//! A pack can arrive from a public GitHub release or a user-selected ZIP.
//! Both paths share the same bounded staging and validation path; activation
//! is always tied to the reviewed staging token.

use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::asset_pack::{CatalogPack, CatalogPackReview};
use crate::shared::errors::ScannerError;

const ZIP_ASSET_NAME: &str = "catalog-pack.zip";
const STAGING_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_RELEASE_METADATA_BYTES: usize = 1_024 * 1_024;
pub(crate) const MAX_ARCHIVE_BYTES: usize = 75 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_UNCOMPRESSED_BYTES: u64 = 150 * 1024 * 1024;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CatalogImportPreview {
    pub staging_token: String,
    pub source_kind: String,
    pub source_label: String,
    pub source_url: Option<String>,
    pub release_tag: Option<String>,
    pub review: CatalogPackReview,
    pub replaces_active_pack: bool,
}

#[derive(Debug, Clone)]
pub struct InstalledCatalogProvenance {
    pub source_kind: String,
    pub source_url: Option<String>,
    pub repository: Option<String>,
    pub release_tag: Option<String>,
    pub archive_digest: String,
}

#[derive(Clone)]
struct PendingImport {
    staging_path: PathBuf,
    source_kind: String,
    source_url: Option<String>,
    repository: Option<String>,
    release_tag: Option<String>,
    archive_digest: String,
    expires_at: Instant,
}

#[derive(Default)]
pub struct CatalogImportState {
    pending: tokio::sync::Mutex<HashMap<String, PendingImport>>,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    id: u64,
    name: String,
}

pub async fn preview_github(
    state: &CatalogImportState,
    app_data_dir: &Path,
    source_url: &str,
) -> Result<CatalogImportPreview, ScannerError> {
    let source = GithubReleaseSource::parse(source_url)?;
    let release = fetch_release(&source).await?;
    let zip_asset_id = find_asset(&release.assets, ZIP_ASSET_NAME)?;
    let client = github_import_client()?;
    let archive = download_asset(&client, &source.repository, zip_asset_id, MAX_ARCHIVE_BYTES).await?;
    stage_archive(
        state,
        app_data_dir,
        archive,
        PendingSource {
            source_kind: "community".to_string(),
            source_url: Some(source.original),
            repository: Some(source.repository),
            release_tag: Some(release.tag_name),
        },
    )
    .await
}

pub async fn preview_local_archive(
    state: &CatalogImportState,
    app_data_dir: &Path,
    archive_path: &Path,
) -> Result<CatalogImportPreview, ScannerError> {
    validate_local_archive_path(archive_path)?;
    let archive_metadata = std::fs::metadata(archive_path)?;
    if archive_metadata.len() > MAX_ARCHIVE_BYTES as u64 {
        return Err(ScannerError::Validation(
            "Catalog ZIP exceeds the size limit".to_string(),
        ));
    }
    let archive = std::fs::read(archive_path)?;
    if archive.len() > MAX_ARCHIVE_BYTES {
        return Err(ScannerError::Validation(
            "Catalog ZIP exceeds the size limit".to_string(),
        ));
    }
    let source_label = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("catalog-pack.zip")
        .to_string();
    stage_archive(
        state,
        app_data_dir,
        archive,
        PendingSource {
            source_kind: "local".to_string(),
            source_url: None,
            repository: Some(source_label),
            release_tag: None,
        },
    )
    .await
}

pub async fn install(
    state: &CatalogImportState,
    app_data_dir: &Path,
    staging_token: &str,
) -> Result<InstalledCatalogProvenance, ScannerError> {
    let pending = state.pending.lock().await.remove(staging_token).ok_or_else(|| {
        ScannerError::Validation(
            "Catalog review has expired. Check the pack again before installing.".to_string(),
        )
    })?;
    if Instant::now() > pending.expires_at {
        let _ = std::fs::remove_dir_all(&pending.staging_path);
        return Err(ScannerError::Validation(
            "Catalog review has expired. Check the pack again before installing.".to_string(),
        ));
    }
    // Confirm the staged content immediately before the atomic swap. The
    // original URL or ZIP is never re-read after the user has reviewed it.
    CatalogPack::load_from_root(pending.staging_path.clone())?.review()?;
    replace_active_pack(app_data_dir, &pending.staging_path)?;
    Ok(InstalledCatalogProvenance {
        source_kind: pending.source_kind,
        source_url: pending.source_url,
        repository: pending.repository,
        release_tag: pending.release_tag,
        archive_digest: pending.archive_digest,
    })
}

pub async fn record_provenance(
    pool: &sqlx::SqlitePool,
    provenance: &InstalledCatalogProvenance,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO catalog_pack_provenance (singleton, source_kind, source_url, repository, release_tag, asset_name, asset_digest)
         VALUES (1, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(singleton) DO UPDATE SET source_kind = excluded.source_kind, source_url = excluded.source_url,
             repository = excluded.repository, release_tag = excluded.release_tag, asset_name = excluded.asset_name,
             asset_digest = excluded.asset_digest, installed_at = CURRENT_TIMESTAMP",
    )
    .bind(&provenance.source_kind)
    .bind(&provenance.source_url)
    .bind(&provenance.repository)
    .bind(&provenance.release_tag)
    .bind(ZIP_ASSET_NAME)
    .bind(&provenance.archive_digest)
    .execute(pool)
    .await?;
    Ok(())
}

struct PendingSource {
    source_kind: String,
    source_url: Option<String>,
    repository: Option<String>,
    release_tag: Option<String>,
}

async fn stage_archive(
    state: &CatalogImportState,
    app_data_dir: &Path,
    archive: Vec<u8>,
    source: PendingSource,
) -> Result<CatalogImportPreview, ScannerError> {
    if archive.len() > MAX_ARCHIVE_BYTES {
        return Err(ScannerError::Validation(
            "Catalog ZIP exceeds the size limit".to_string(),
        ));
    }
    let staging_path = app_data_dir.join(format!(".catalog-import-staging-{}", Uuid::new_v4()));
    let result = (|| {
        std::fs::create_dir_all(&staging_path)?;
        extract_archive(&archive, &staging_path)?;
        let pack = CatalogPack::load_from_root(staging_path.clone())?;
        let review = pack.review()?;
        if review.entries == 0 {
            return Err(ScannerError::Validation(
                "Catalog pack does not contain supported catalog entries".to_string(),
            ));
        }
        Ok(review)
    })();
    let review = match result {
        Ok(review) => review,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging_path);
            return Err(error);
        }
    };
    let staging_token = Uuid::new_v4().to_string();
    let source_label = source
        .repository
        .clone()
        .unwrap_or_else(|| "Local ZIP".to_string());
    let release_tag = source.release_tag.clone();
    state.pending.lock().await.insert(
        staging_token.clone(),
        PendingImport {
            staging_path,
            source_kind: source.source_kind.clone(),
            source_url: source.source_url.clone(),
            repository: source.repository,
            release_tag: source.release_tag,
            archive_digest: format!("{:x}", Sha256::digest(&archive)),
            expires_at: Instant::now() + STAGING_TTL,
        },
    );
    Ok(CatalogImportPreview {
        staging_token,
        source_kind: source.source_kind,
        source_label,
        source_url: source.source_url,
        release_tag,
        review,
        replaces_active_pack: CatalogPack::root(app_data_dir).exists(),
    })
}

fn validate_local_archive_path(path: &Path) -> Result<(), ScannerError> {
    if !path.is_file() {
        return Err(ScannerError::Validation(
            "Choose a local catalog ZIP file".to_string(),
        ));
    }
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        return Err(ScannerError::Validation(
            "Choose a catalog file with the .zip extension".to_string(),
        ));
    }
    Ok(())
}

struct GithubReleaseSource {
    repository: String,
    release_api: String,
    original: String,
}

impl GithubReleaseSource {
    fn parse(value: &str) -> Result<Self, ScannerError> {
        let url = reqwest::Url::parse(value.trim()).map_err(|_| {
            ScannerError::Validation("Enter a public GitHub repository or release URL".to_string())
        })?;
        if url.scheme() != "https"
            || !matches!(url.host_str(), Some("github.com") | Some("www.github.com"))
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(ScannerError::Validation(
                "Enter a public HTTPS GitHub repository or release URL".to_string(),
            ));
        }
        let parts = url
            .path_segments()
            .map(|segments| segments.filter(|part| !part.is_empty()).collect::<Vec<_>>())
            .unwrap_or_default();
        let (owner, repository, suffix) = match parts.as_slice() {
            [owner, repository] => (*owner, *repository, None),
            [owner, repository, "releases", "latest"] => (*owner, *repository, None),
            [owner, repository, "releases", "tag", tag] if !tag.is_empty() => {
                (*owner, *repository, Some(*tag))
            }
            _ => return Err(ScannerError::Validation("Use https://github.com/owner/repository, /releases/latest, or /releases/tag/version".to_string())),
        };
        if !is_repo_part(owner) || !is_repo_part(repository) {
            return Err(ScannerError::Validation(
                "GitHub repository URL contains an invalid owner or repository name".to_string(),
            ));
        }
        let repo = format!("{owner}/{repository}");
        let release_api = suffix.map_or_else(
            || format!("https://api.github.com/repos/{repo}/releases/latest"),
            |tag| format!("https://api.github.com/repos/{repo}/releases/tags/{}", urlencoding::encode(tag)),
        );
        Ok(Self {
            repository: repo,
            release_api,
            original: url.into(),
        })
    }
}

fn is_repo_part(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

async fn fetch_release(source: &GithubReleaseSource) -> Result<GithubRelease, ScannerError> {
    let client = github_import_client()?;
    let response = client
        .get(&source.release_api)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(network_error)?
        .error_for_status()
        .map_err(network_error)?;
    let bytes = read_response_limited(response, MAX_RELEASE_METADATA_BYTES).await?;
    let release: GithubRelease =
        serde_json::from_slice(&bytes).map_err(|error| ScannerError::Parse {
            what: "GitHub release metadata".to_string(),
            detail: error.to_string(),
        })?;
    if release.draft || release.prerelease {
        return Err(ScannerError::Validation(
            "Catalog release must be a published stable release".to_string(),
        ));
    }
    Ok(release)
}

fn find_asset(assets: &[GithubAsset], expected: &str) -> Result<u64, ScannerError> {
    let matches = assets
        .iter()
        .filter(|asset| asset.name == expected)
        .map(|asset| asset.id)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [id] => Ok(*id),
        [] => Err(ScannerError::Validation(format!(
            "Release does not include required asset '{expected}'"
        ))),
        _ => Err(ScannerError::Validation(format!(
            "Release has multiple '{expected}' assets"
        ))),
    }
}

async fn download_asset(
    client: &reqwest::Client,
    repository: &str,
    asset_id: u64,
    limit: usize,
) -> Result<Vec<u8>, ScannerError> {
    let url = format!("https://api.github.com/repos/{repository}/releases/assets/{asset_id}");
    let response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()
        .await
        .map_err(network_error)?
        .error_for_status()
        .map_err(network_error)?;
    let host = response.url().host_str().unwrap_or_default();
    if !is_allowed_delivery_host(host) {
        return Err(ScannerError::Security(
            "Catalog asset redirect left an approved GitHub delivery host".to_string(),
        ));
    }
    read_response_limited(response, limit).await
}

fn github_import_client() -> Result<reqwest::Client, ScannerError> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.url().host_str().is_some_and(is_allowed_delivery_host) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .user_agent("EMMM catalog importer")
        .build()
        .map_err(|error| {
            ScannerError::Network(format!(
                "Could not initialize GitHub catalog importer: {error}"
            ))
        })
}

fn is_allowed_delivery_host(host: &str) -> bool {
    matches!(
        host,
        "api.github.com"
            | "github-releases.githubusercontent.com"
            | "release-assets.githubusercontent.com"
            | "objects.githubusercontent.com"
    )
}

async fn read_response_limited(
    response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, ScannerError> {
    if response.content_length().is_some_and(|size| size > limit as u64) {
        return Err(ScannerError::Validation(
            "Catalog release response exceeds the size limit".to_string(),
        ));
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(network_error)?;
        if body
            .len()
            .checked_add(chunk.len())
            .is_none_or(|size| size > limit)
        {
            return Err(ScannerError::Validation(
                "Catalog release response exceeds the size limit".to_string(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn network_error(error: reqwest::Error) -> ScannerError {
    if error.is_timeout() {
        ScannerError::Network("Catalog request timed out".to_string())
    } else {
        ScannerError::Network("Could not reach GitHub's public release API".to_string())
    }
}

pub(crate) fn extract_archive(bytes: &[u8], destination: &Path) -> Result<(), ScannerError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| ScannerError::Parse {
        what: "catalog archive".to_string(),
        detail: error.to_string(),
    })?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(ScannerError::Validation(
            "Catalog archive contains too many entries".to_string(),
        ));
    }
    let mut written = HashSet::new();
    let mut total_uncompressed = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| ScannerError::Parse {
            what: "catalog archive".to_string(),
            detail: error.to_string(),
        })?;
        if entry.is_symlink() {
            return Err(ScannerError::Security(
                "Catalog archive contains a symbolic link".to_string(),
            ));
        }
        let path = entry.enclosed_name().ok_or_else(|| {
            ScannerError::Security("Catalog archive contains an unsafe path".to_string())
        })?;
        if entry.is_dir() {
            if !is_allowed_directory(&path) {
                return Err(ScannerError::Validation(
                    "Catalog archive contains an unsupported directory".to_string(),
                ));
            }
            std::fs::create_dir_all(destination.join(path))?;
            continue;
        }
        if !is_allowed_file(&path) {
            return Err(ScannerError::Validation(
                "Catalog archive contains an unsupported file".to_string(),
            ));
        }
        total_uncompressed = total_uncompressed.checked_add(entry.size()).ok_or_else(|| {
            ScannerError::Validation("Catalog archive is too large".to_string())
        })?;
        if total_uncompressed > MAX_UNCOMPRESSED_BYTES {
            return Err(ScannerError::Validation(
                "Catalog archive is too large".to_string(),
            ));
        }
        if !written.insert(path.clone()) {
            return Err(ScannerError::Validation(
                "Catalog archive contains duplicate files".to_string(),
            ));
        }
        let target = destination.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut output = std::fs::File::create(target)?;
        std::io::copy(&mut entry, &mut output)?;
    }
    Ok(())
}

fn is_allowed_directory(path: &Path) -> bool {
    let components = path.components().collect::<Vec<_>>();
    matches!(components.as_slice(), [Component::Normal(root)] if *root == "catalog" || *root == "assets")
        || matches!(components.as_slice(), [Component::Normal(root), Component::Normal(_)] if *root == "assets")
        || matches!(components.as_slice(), [Component::Normal(root), Component::Normal(_), Component::Normal(category)] if *root == "assets" && matches!(category.to_str(), Some("characters" | "weapons")))
}

fn is_allowed_file(path: &Path) -> bool {
    let components = path.components().collect::<Vec<_>>();
    if path == Path::new("manifest.json") {
        return true;
    }
    match components.as_slice() {
        [Component::Normal(root), Component::Normal(name)] if *root == "catalog" => {
            name.to_str().is_some_and(|name| name.ends_with(".json"))
        }
        [Component::Normal(root), Component::Normal(_), Component::Normal(category), Component::Normal(name)]
            if *root == "assets" && matches!(category.to_str(), Some("characters" | "weapons")) =>
        {
            name.to_str().is_some_and(|name| {
                name.rsplit_once('.').is_some_and(|(stem, extension)| {
                    !stem.is_empty()
                        && matches!(extension.to_ascii_lowercase().as_str(), "png" | "jpg" | "webp" | "gif")
                })
            })
        }
        _ => false,
    }
}

fn replace_active_pack(app_data_dir: &Path, staging: &Path) -> Result<(), ScannerError> {
    let active = CatalogPack::root(app_data_dir);
    let backup = app_data_dir.join(format!(".catalog-pack-backup-{}", Uuid::new_v4()));
    let had_active_pack = active.exists();
    if had_active_pack {
        std::fs::rename(&active, &backup)?;
    }
    if let Err(error) = std::fs::rename(staging, &active) {
        if had_active_pack {
            let _ = std::fs::rename(&backup, &active);
        }
        return Err(ScannerError::Io(format!(
            "Could not activate the validated catalog pack: {error}"
        )));
    }
    if had_active_pack {
        if let Err(error) = std::fs::remove_dir_all(backup) {
            log::warn!("Catalog pack backup could not be removed after activation: {error}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_allowed_file, validate_local_archive_path, GithubReleaseSource};
    use std::path::Path;

    #[test]
    fn github_release_source_accepts_only_public_repository_release_urls() {
        let repository = GithubReleaseSource::parse("https://github.com/example/catalog-pack")
            .expect("repository URL should be accepted");
        assert_eq!(repository.repository, "example/catalog-pack");
        assert!(repository.release_api.ends_with("/releases/latest"));

        let tagged = GithubReleaseSource::parse(
            "https://github.com/example/catalog-pack/releases/tag/v1.2.3",
        )
        .expect("tag URL should be accepted");
        assert!(tagged.release_api.ends_with("/releases/tags/v1.2.3"));
    }

    #[test]
    fn github_release_source_rejects_raw_and_ambiguous_urls() {
        for url in [
            "http://github.com/example/catalog-pack",
            "https://raw.githubusercontent.com/example/catalog-pack/main/manifest.json",
            "https://github.com/example/catalog-pack/archive/refs/heads/main.zip",
            "https://github.com/example/catalog-pack?download=1",
        ] {
            assert!(GithubReleaseSource::parse(url).is_err(), "{url}");
        }
    }

    #[test]
    fn archive_file_allowlist_rejects_executables_and_nested_catalogs() {
        assert!(is_allowed_file(Path::new("manifest.json")));
        assert!(is_allowed_file(Path::new("catalog/gimi.json")));
        assert!(is_allowed_file(Path::new("assets/gimi/characters/amber.webp")));
        assert!(!is_allowed_file(Path::new("assets/gimi/characters/amber.svg")));
        assert!(!is_allowed_file(Path::new("assets/gimi/amber.webp")));
        assert!(!is_allowed_file(Path::new("images/gimi/characters/amber.webp")));
        assert!(!is_allowed_file(Path::new("catalog/nested/gimi.json")));
        assert!(!is_allowed_file(Path::new("catalog/installer.exe")));
        assert!(!is_allowed_file(Path::new("script.js")));
    }

    #[test]
    fn local_import_requires_a_zip_file() {
        assert!(validate_local_archive_path(Path::new("missing-catalog.zip")).is_err());
        assert!(validate_local_archive_path(Path::new("catalog.json")).is_err());
    }
}
