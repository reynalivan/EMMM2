//! Signed, read-only catalog-release updates.
//!
//! End-user installations never authenticate to GitHub and never ingest
//! upstream importer repositories. They accept only a signed ZIP published by
//! the fixed catalog release channel, validate it in a sibling staging folder,
//! and atomically replace the active data-only pack.

use std::{
    collections::HashSet,
    io::Cursor,
    path::{Component, Path, PathBuf},
    time::Duration,
};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use futures_util::StreamExt;
use semver::Version;
use serde::Deserialize;
use uuid::Uuid;

use super::asset_pack::CatalogPack;
use crate::shared::errors::ScannerError;

pub(crate) const CATALOG_REPOSITORY: &str = "reynalivan/3dm-catalog-asset";
pub(crate) const RELEASE_API: &str =
    "https://api.github.com/repos/reynalivan/3dm-catalog-asset/releases/latest";
const ZIP_ASSET_NAME: &str = "catalog-pack.zip";
const SIGNATURE_ASSET_NAME: &str = "catalog-pack.sig";
const EXPECTED_PACK_ID: &str = "3dm-catalog-asset";
pub(crate) const MAX_RELEASE_METADATA_BYTES: usize = 1_024 * 1_024;
pub(crate) const MAX_SIGNATURE_BYTES: usize = 1_024;
pub(crate) const MAX_ARCHIVE_BYTES: usize = 75 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_UNCOMPRESSED_BYTES: u64 = 150 * 1024 * 1024;
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const CATALOG_UPDATE_PUBLIC_KEY_HEX: &str =
    "a0b5daf572680311fbc5da1ba5d4de57295d3fb00d3ee57ddfb097b4b59039b6";

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct CatalogUpdateCheck {
    pub state: String,
    pub current_version: Option<String>,
    pub available_version: Option<String>,
    pub release_notes: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct CatalogUpdateInstallResult {
    pub version: String,
    pub entries: usize,
}

/// Serializes manual and background update operations. A second updater must
/// never replace the active pack while the first one is validating a staging
/// directory.
#[derive(Default)]
pub struct CatalogUpdateState(pub tokio::sync::Mutex<()>);

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    body: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    id: u64,
    name: String,
}

struct ReleaseAssets {
    version: Version,
    notes: Option<String>,
    zip_asset_id: u64,
    signature_asset_id: u64,
}

pub async fn check(app_data_dir: &Path) -> Result<CatalogUpdateCheck, ScannerError> {
    let release = fetch_release().await?;
    let current_version = installed_version(app_data_dir)?;
    let state = match current_version.as_deref().map(parse_version).transpose()? {
        Some(current) if current >= release.version => "up_to_date",
        _ => "update_available",
    };
    Ok(CatalogUpdateCheck {
        state: state.to_string(),
        current_version,
        available_version: Some(release.version.to_string()),
        release_notes: release.notes,
    })
}

pub async fn install(app_data_dir: &Path) -> Result<CatalogUpdateInstallResult, ScannerError> {
    let release = fetch_release().await?;
    if let Some(current) = installed_version(app_data_dir)? {
        let current = parse_version(&current)?;
        if current >= release.version {
            return Err(ScannerError::Validation(
                "The published catalog release is not newer than the installed pack".to_string(),
            ));
        }
    }

    let client = update_client()?;
    let archive = download_asset(&client, release.zip_asset_id, MAX_ARCHIVE_BYTES).await?;
    let signature =
        download_asset(&client, release.signature_asset_id, MAX_SIGNATURE_BYTES).await?;
    verify_archive_signature(&archive, &signature)?;

    let staging = staging_root(app_data_dir);
    let result = (|| {
        std::fs::create_dir_all(&staging)?;
        extract_archive(&archive, &staging)?;
        let pack = CatalogPack::load_from_root(staging.clone())?;
        if pack.id() != EXPECTED_PACK_ID {
            return Err(ScannerError::Validation(
                "Catalog release uses an unexpected pack identifier".to_string(),
            ));
        }
        let version = parse_version(pack.version())?;
        if version != release.version {
            return Err(ScannerError::Validation(
                "Catalog release tag does not match the pack manifest version".to_string(),
            ));
        }
        let status = pack.status()?;
        replace_active_pack(app_data_dir, &staging)?;
        Ok(CatalogUpdateInstallResult {
            version: version.to_string(),
            entries: status.entries,
        })
    })();
    if result.is_err() && staging.exists() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    result
}

async fn fetch_release() -> Result<ReleaseAssets, ScannerError> {
    let client = update_client()?;
    let response = client
        .get(RELEASE_API)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(network_error)?
        .error_for_status()
        .map_err(network_error)?;
    let bytes = read_response_limited(response, MAX_RELEASE_METADATA_BYTES).await?;
    let release: GithubRelease =
        serde_json::from_slice(&bytes).map_err(|error| ScannerError::Parse {
            what: "catalog release metadata".to_string(),
            detail: error.to_string(),
        })?;
    if release.draft || release.prerelease {
        return Err(ScannerError::Validation(
            "Catalog release channel returned a non-production release".to_string(),
        ));
    }
    let version = parse_version(release.tag_name.trim_start_matches('v'))?;
    let zip_asset_id = find_asset(&release.assets, ZIP_ASSET_NAME)?;
    let signature_asset_id = find_asset(&release.assets, SIGNATURE_ASSET_NAME)?;
    let notes = (!release.body.trim().is_empty()).then_some(release.body.trim().to_string());
    Ok(ReleaseAssets {
        version,
        notes,
        zip_asset_id,
        signature_asset_id,
    })
}

fn find_asset(assets: &[GithubAsset], expected_name: &str) -> Result<u64, ScannerError> {
    let matches = assets
        .iter()
        .filter(|asset| asset.name == expected_name)
        .map(|asset| asset.id)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [id] => Ok(*id),
        [] => Err(ScannerError::Validation(format!(
            "Catalog release does not include required asset '{expected_name}'"
        ))),
        _ => Err(ScannerError::Validation(format!(
            "Catalog release includes duplicate asset '{expected_name}'"
        ))),
    }
}

pub(crate) fn update_client() -> Result<reqwest::Client, ScannerError> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(HTTP_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("EMMM catalog updater")
        .build()
        .map_err(|error| {
            ScannerError::Network(format!("Could not initialize catalog updater: {error}"))
        })
}

async fn download_asset(
    client: &reqwest::Client,
    asset_id: u64,
    limit: usize,
) -> Result<Vec<u8>, ScannerError> {
    let url =
        format!("https://api.github.com/repos/{CATALOG_REPOSITORY}/releases/assets/{asset_id}");
    let response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()
        .await
        .map_err(network_error)?
        .error_for_status()
        .map_err(network_error)?;
    read_response_limited(response, limit).await
}

async fn read_response_limited(
    response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, ScannerError> {
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
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
        ScannerError::Network("Catalog update request timed out".to_string())
    } else {
        ScannerError::Network("Could not reach the catalog release channel".to_string())
    }
}

fn installed_version(app_data_dir: &Path) -> Result<Option<String>, ScannerError> {
    match CatalogPack::load(app_data_dir) {
        Ok(pack) => Ok(Some(pack.version().to_string())),
        Err(error) if error.to_string().contains("not installed") => Ok(None),
        Err(error) => Err(error),
    }
}

fn parse_version(value: &str) -> Result<Version, ScannerError> {
    Version::parse(value).map_err(|_| {
        ScannerError::Validation("Catalog versions must use semantic versioning".to_string())
    })
}

pub(crate) fn verify_archive_signature(
    archive: &[u8],
    signature: &[u8],
) -> Result<(), ScannerError> {
    verify_signature_with_public_key(archive, signature, CATALOG_UPDATE_PUBLIC_KEY_HEX)
}

fn verify_signature_with_public_key(
    archive: &[u8],
    signature: &[u8],
    public_key_hex: &str,
) -> Result<(), ScannerError> {
    let signature = std::str::from_utf8(signature)
        .map_err(|_| ScannerError::Validation("Catalog signature is not UTF-8 text".to_string()))?
        .trim();
    let signature = decode_hex::<64>(signature, "Catalog signature")?;
    let public_key = decode_hex::<32>(public_key_hex, "Catalog public key")?;
    let key = VerifyingKey::from_bytes(&public_key)
        .map_err(|_| ScannerError::Validation("Catalog public key is invalid".to_string()))?;
    key.verify(archive, &Signature::from_bytes(&signature))
        .map_err(|_| ScannerError::Security("Catalog release signature is invalid".to_string()))
}

fn decode_hex<const N: usize>(value: &str, label: &str) -> Result<[u8; N], ScannerError> {
    if value.len() != N * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ScannerError::Validation(format!(
            "{label} has an invalid format"
        )));
    }
    let mut bytes = [0_u8; N];
    for (index, output) in bytes.iter_mut().enumerate() {
        *output = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| ScannerError::Validation(format!("{label} has an invalid format")))?;
    }
    Ok(bytes)
}

pub(crate) fn extract_archive(bytes: &[u8], destination: &Path) -> Result<(), ScannerError> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| ScannerError::Parse {
            what: "catalog release archive".to_string(),
            detail: error.to_string(),
        })?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(ScannerError::Validation(
            "Catalog release archive contains too many entries".to_string(),
        ));
    }
    let mut written = HashSet::new();
    let mut total_uncompressed = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| ScannerError::Parse {
                what: "catalog release archive".to_string(),
                detail: error.to_string(),
            })?;
        if entry.is_symlink() {
            return Err(ScannerError::Security(
                "Catalog release archive contains a symbolic link".to_string(),
            ));
        }
        let path = entry.enclosed_name().ok_or_else(|| {
            ScannerError::Security("Catalog release archive contains an unsafe path".to_string())
        })?;
        if entry.is_dir() {
            if !is_allowed_directory(&path) {
                return Err(ScannerError::Validation(
                    "Catalog release archive contains an unsupported directory".to_string(),
                ));
            }
            std::fs::create_dir_all(destination.join(path))?;
            continue;
        }
        if !is_allowed_file(&path) {
            return Err(ScannerError::Validation(
                "Catalog release archive contains an unsupported file".to_string(),
            ));
        }
        total_uncompressed = total_uncompressed
            .checked_add(entry.size())
            .ok_or_else(|| {
                ScannerError::Validation("Catalog release archive is too large".to_string())
            })?;
        if total_uncompressed > MAX_UNCOMPRESSED_BYTES {
            return Err(ScannerError::Validation(
                "Catalog release archive is too large".to_string(),
            ));
        }
        if !written.insert(path.clone()) {
            return Err(ScannerError::Validation(
                "Catalog release archive contains duplicate files".to_string(),
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
                        && matches!(
                            extension.to_ascii_lowercase().as_str(),
                            "png" | "jpg" | "webp" | "gif"
                        )
                })
            })
        }
        _ => false,
    }
}

fn staging_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(format!(".catalog-pack-staging-{}", Uuid::new_v4()))
}

pub(crate) fn replace_active_pack(app_data_dir: &Path, staging: &Path) -> Result<(), ScannerError> {
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
        std::fs::remove_dir_all(backup)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{decode_hex, is_allowed_file, verify_signature_with_public_key};
    use ed25519_dalek::{Signer, SigningKey};
    use std::path::Path;

    #[test]
    fn archive_file_allowlist_rejects_executables_and_nested_catalogs() {
        assert!(is_allowed_file(Path::new("manifest.json")));
        assert!(is_allowed_file(Path::new("catalog/gimi.json")));
        assert!(is_allowed_file(Path::new(
            "assets/gimi/characters/amber.webp"
        )));
        assert!(!is_allowed_file(Path::new(
            "assets/gimi/characters/amber.svg"
        )));
        assert!(!is_allowed_file(Path::new("assets/gimi/amber.webp")));
        assert!(!is_allowed_file(Path::new(
            "images/gimi/characters/amber.webp"
        )));
        assert!(!is_allowed_file(Path::new("catalog/nested/gimi.json")));
        assert!(!is_allowed_file(Path::new("catalog/installer.exe")));
        assert!(!is_allowed_file(Path::new("script.js")));
    }

    #[test]
    fn fixed_size_hex_decoder_rejects_bad_input() {
        assert!(decode_hex::<2>("abcd", "test").is_ok());
        assert!(decode_hex::<2>("abc", "test").is_err());
        assert!(decode_hex::<2>("zzzz", "test").is_err());
    }

    #[test]
    fn signature_verification_accepts_only_the_matching_archive() {
        let signing_key = SigningKey::from_bytes(&[9_u8; 32]);
        let archive = b"signed catalog archive";
        let signature = signing_key.sign(archive);
        let signature_text = signature
            .to_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let public_key_text = signing_key
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        assert!(verify_signature_with_public_key(
            archive,
            signature_text.as_bytes(),
            &public_key_text
        )
        .is_ok());
        assert!(verify_signature_with_public_key(
            b"tampered archive",
            signature_text.as_bytes(),
            &public_key_text
        )
        .is_err());
    }
}
