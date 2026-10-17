use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::Manager;

use crate::modules::matching::application::deep_matcher::models::types::RuntimeTarget;
use crate::modules::matching::application::deep_matcher::{DbEntry, MasterDb};
use crate::shared::errors::ScannerError;

pub(crate) const PACK_DIRECTORY: &str = "asset-pack";
pub(crate) const MANIFEST_FILE: &str = "manifest.json";
const MAX_CATALOG_BYTES: u64 = 10 * 1024 * 1024;
const MAX_AVATAR_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct CatalogPackStatus {
    pub state: String,
    pub pack_id: Option<String>,
    pub version: Option<String>,
    pub message: Option<String>,
    pub entries: usize,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct CatalogPackRefreshResult {
    pub state: String,
    pub entries: usize,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPackReview {
    pub pack_id: String,
    pub version: String,
    pub publisher: String,
    pub source: String,
    pub supported_games: Vec<String>,
    pub entries: usize,
    pub has_keyviewer_targets: bool,
}

/// A runtime-eligible catalog entry for KeyViewer generation.
#[derive(Debug, Clone)]
pub struct CatalogKeyviewerEntry {
    pub name: String,
    pub aliases: Vec<String>,
    pub object_type: String,
    /// Validated non-shader targets only. Their typed provenance remains intact.
    pub runtime_targets: Vec<RuntimeTarget>,
    pub catalog_id: String,
    pub catalog_version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    id: String,
    version: String,
    author: String,
    source: String,
    catalogs: HashMap<String, CatalogFile>,
}

#[derive(Deserialize)]
struct CatalogFile {
    path: String,
    sha256: String,
}

pub(crate) struct CatalogPack {
    root: PathBuf,
    manifest: Manifest,
    manifest_checksum: String,
}

impl CatalogPack {
    pub fn root(app_data_dir: &Path) -> PathBuf {
        app_data_dir.join(PACK_DIRECTORY)
    }

    pub(crate) fn load(app_data_dir: &Path) -> Result<Self, ScannerError> {
        let root = Self::root(app_data_dir);
        Self::load_from_root(root)
    }

    /// Parse a pack from an already-selected directory. The updater validates
    /// its staging directory through this entry point before an atomic swap.
    pub(crate) fn load_from_root(root: PathBuf) -> Result<Self, ScannerError> {
        let manifest_path = root.join(MANIFEST_FILE);
        if !manifest_path.is_file() {
            return Err(ScannerError::Io(
                "Catalog asset pack is not installed".to_string(),
            ));
        }
        let manifest_bytes = std::fs::read(&manifest_path)?;
        let manifest: Manifest =
            serde_json::from_slice(&manifest_bytes).map_err(|error| ScannerError::Parse {
                what: "catalog asset-pack manifest".to_string(),
                detail: error.to_string(),
            })?;
        if manifest.id.trim().is_empty()
            || manifest.version.trim().is_empty()
            || manifest.author.trim().is_empty()
            || manifest.source.trim().is_empty()
        {
            return Err(ScannerError::Parse {
                what: "catalog asset-pack manifest".to_string(),
                detail: "missing required metadata".to_string(),
            });
        }
        Ok(Self {
            root,
            manifest,
            manifest_checksum: format!("{:x}", Sha256::digest(&manifest_bytes)),
        })
    }

    pub(crate) fn id(&self) -> &str {
        &self.manifest.id
    }

    pub(crate) fn version(&self) -> &str {
        &self.manifest.version
    }

    pub(crate) fn manifest_checksum(&self) -> &str {
        &self.manifest_checksum
    }

    pub(crate) fn review(&self) -> Result<CatalogPackReview, ScannerError> {
        let mut supported_games = Vec::new();
        let mut entries = 0;
        let mut has_keyviewer_targets = false;
        for game_type in crate::modules::games::domain::models::GameType::ALL {
            let game_entries = self.entries_for(game_type as i32)?;
            if game_entries.is_empty() {
                continue;
            }
            entries += game_entries.len();
            has_keyviewer_targets |= game_entries.iter().any(|entry| {
                entry
                    .runtime_targets
                    .iter()
                    .any(RuntimeTarget::is_resource_target)
            });
            supported_games.push(game_type.resource_code().to_string());
        }
        Ok(CatalogPackReview {
            pack_id: self.manifest.id.clone(),
            version: self.manifest.version.clone(),
            publisher: self.manifest.author.clone(),
            source: self.manifest.source.clone(),
            supported_games,
            entries,
            has_keyviewer_targets,
        })
    }

    pub(crate) fn entries_for(&self, game_type: i32) -> Result<Vec<DbEntry>, ScannerError> {
        let entries = self.read_entries(game_type)?;
        for entry in &entries {
            validate_runtime_targets(entry)?;
        }
        Ok(entries)
    }

    /// Reads catalog metadata, validates its checksum, and resolves bounded
    /// catalog-owned avatar paths. User-selected preview images never come
    /// from a catalog pack.
    fn read_entries(&self, game_type: i32) -> Result<Vec<DbEntry>, ScannerError> {
        self.read_entries_with(game_type, || {})
    }

    fn read_entries_with(
        &self,
        game_type: i32,
        after_checksum: impl FnOnce(),
    ) -> Result<Vec<DbEntry>, ScannerError> {
        let game =
            crate::modules::games::application::game::schema_loader::normalize_game_type(game_type);
        let Some(catalog) = self.manifest.catalogs.get(&game) else {
            return Ok(Vec::new());
        };
        let path = resolve_child(&self.root, &catalog.path, "catalog")?;
        let metadata = std::fs::metadata(&path)?;
        if metadata.len() > MAX_CATALOG_BYTES {
            return Err(ScannerError::Io(
                "Catalog file exceeds the size limit".to_string(),
            ));
        }
        let mut entries = {
            let bytes = std::fs::read(&path)?;
            if bytes.len() as u64 > MAX_CATALOG_BYTES {
                return Err(ScannerError::Io(
                    "Catalog file exceeds the size limit".to_string(),
                ));
            }
            if format!("{:x}", Sha256::digest(&bytes)) != catalog.sha256.to_ascii_lowercase() {
                return Err(ScannerError::Io(
                    "Catalog checksum does not match its manifest".to_string(),
                ));
            }
            after_checksum();
            let json = std::str::from_utf8(&bytes).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "stream did not contain valid UTF-8",
                )
            })?;
            MasterDb::from_json(json)?.entries
        };
        for entry in &mut entries {
            resolve_entry_thumbnail(&self.root, entry)?;
        }
        Ok(entries)
    }

    pub(crate) fn keyviewer_entries_for(
        &self,
        game_type: i32,
    ) -> Result<Vec<CatalogKeyviewerEntry>, ScannerError> {
        let catalog_id = self.manifest.id.clone();
        let catalog_version = self.manifest.version.clone();

        let mut keyviewer_entries = Vec::new();
        for entry in self.read_entries(game_type)? {
            validate_runtime_targets(&entry)?;
            let runtime_targets: Vec<RuntimeTarget> = entry
                .runtime_targets
                .into_iter()
                .filter(RuntimeTarget::is_resource_target)
                .collect();
            if runtime_targets.is_empty() {
                continue;
            }
            keyviewer_entries.push(CatalogKeyviewerEntry {
                name: entry.name,
                aliases: entry.aliases,
                object_type: entry.object_type,
                runtime_targets,
                catalog_id: catalog_id.clone(),
                catalog_version: catalog_version.clone(),
            });
        }
        Ok(keyviewer_entries)
    }

    pub(crate) fn status(&self) -> Result<CatalogPackStatus, ScannerError> {
        let mut entries = 0;
        for game_type in 0..=4 {
            entries += self.entries_for(game_type)?.len();
        }
        Ok(CatalogPackStatus {
            state: "ready".to_string(),
            pack_id: Some(self.manifest.id.clone()),
            version: Some(self.manifest.version.clone()),
            message: None,
            entries,
        })
    }
}

fn resolve_entry_thumbnail(root: &Path, entry: &mut DbEntry) -> Result<(), ScannerError> {
    let Some(relative) = entry.thumbnail_path.as_deref() else {
        return Ok(());
    };
    let relative_path = Path::new(relative);
    if !is_avatar_path(relative_path) {
        return Err(ScannerError::Parse {
            what: "catalog avatar path".to_string(),
            detail: format!("entry '{}' declares an unsupported avatar path", entry.name),
        });
    }
    let path = resolve_child(root, relative, "catalog avatar")?;
    let metadata = std::fs::metadata(&path)?;
    if metadata.len() == 0 || metadata.len() > MAX_AVATAR_BYTES {
        return Err(ScannerError::Io(format!(
            "Catalog avatar for '{}' exceeds the size limit",
            entry.name
        )));
    }
    let expected_hash = entry
        .metadata
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .and_then(|metadata| metadata.get("thumbnail_asset_sha256"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| ScannerError::Parse {
            what: "catalog avatar metadata".to_string(),
            detail: format!("entry '{}' lacks a valid avatar checksum", entry.name),
        })?;
    if sha256_file(&path)? != expected_hash.to_ascii_lowercase() {
        return Err(ScannerError::Io(format!(
            "Catalog avatar checksum does not match metadata for '{}'",
            entry.name
        )));
    }
    let bytes = std::fs::read(&path)?;
    if !is_supported_avatar(&bytes) {
        return Err(ScannerError::Parse {
            what: "catalog avatar".to_string(),
            detail: format!("entry '{}' has an unsupported avatar format", entry.name),
        });
    }
    entry.thumbnail_path = Some(path.to_string_lossy().into_owned());
    Ok(())
}

fn is_avatar_path(path: &Path) -> bool {
    let components = path.components().collect::<Vec<_>>();
    match components.as_slice() {
        [Component::Normal(root), Component::Normal(game), Component::Normal(category), Component::Normal(filename)]
            if *root == "assets"
                && !game.is_empty()
                && matches!(category.to_str(), Some("characters" | "weapons")) =>
        {
            filename
                .to_str()
                .and_then(|name| name.rsplit_once('.'))
                .is_some_and(|(stem, extension)| {
                    !stem.is_empty()
                        && matches!(
                            extension.to_ascii_lowercase().as_str(),
                            "png" | "jpg" | "webp" | "gif"
                        )
                })
        }
        _ => false,
    }
}

fn is_supported_avatar(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
        || bytes.starts_with(&[0xff, 0xd8, 0xff])
        || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"))
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
}

fn validate_runtime_targets(entry: &DbEntry) -> Result<(), ScannerError> {
    for target in &entry.runtime_targets {
        target.validate().map_err(|detail| ScannerError::Parse {
            what: "catalog runtime target".to_string(),
            detail: format!("entry '{}': {detail}", entry.name),
        })?;
    }
    Ok(())
}

fn resolve_child(root: &Path, relative: &str, label: &str) -> Result<PathBuf, ScannerError> {
    let candidate = Path::new(relative);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(ScannerError::Io(format!(
            "Invalid {label} path in catalog asset pack"
        )));
    }
    let root = root.canonicalize()?;
    let resolved = root.join(candidate).canonicalize()?;
    if !resolved.starts_with(&root) {
        return Err(ScannerError::Io(format!(
            "{label} path escapes catalog asset pack"
        )));
    }
    Ok(resolved)
}

fn sha256_file(path: &Path) -> Result<String, ScannerError> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|error| ScannerError::Io(error.to_string()))?;
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn status(app: &tauri::AppHandle) -> CatalogPackStatus {
    let result = app
        .path()
        .app_data_dir()
        .map_err(|error| ScannerError::Io(error.to_string()))
        .and_then(|dir| CatalogPack::load(&dir))
        .and_then(|pack| pack.status());
    match result {
        Ok(status) => status,
        Err(error) if error.to_string().contains("not installed") => CatalogPackStatus {
            state: "not_installed".to_string(),
            pack_id: None,
            version: None,
            message: None,
            entries: 0,
        },
        Err(error) => CatalogPackStatus {
            state: "invalid".to_string(),
            pack_id: None,
            version: None,
            message: Some(error.to_string()),
            entries: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{is_avatar_path, is_supported_avatar, CatalogPack};
    use sha2::{Digest, Sha256};
    use std::path::Path;

    fn legacy_entries_for(
        pack: &CatalogPack,
        game_type: i32,
    ) -> Result<Vec<super::DbEntry>, super::ScannerError> {
        let game =
            crate::modules::games::application::game::schema_loader::normalize_game_type(game_type);
        let Some(catalog) = pack.manifest.catalogs.get(&game) else {
            return Ok(Vec::new());
        };
        let path = super::resolve_child(&pack.root, &catalog.path, "catalog")?;
        if std::fs::metadata(&path)?.len() > super::MAX_CATALOG_BYTES {
            return Err(super::ScannerError::Io(
                "Catalog file exceeds the size limit".to_string(),
            ));
        }
        if super::sha256_file(&path)? != catalog.sha256.to_ascii_lowercase() {
            return Err(super::ScannerError::Io(
                "Catalog checksum does not match its manifest".to_string(),
            ));
        }
        let json = std::fs::read_to_string(path)?;
        let mut entries = super::MasterDb::from_json(&json)?.entries;
        for entry in &mut entries {
            super::resolve_entry_thumbnail(&pack.root, entry)?;
        }
        for entry in &entries {
            super::validate_runtime_targets(entry)?;
        }
        Ok(entries)
    }

    fn write_catalog_fixture(root: &Path, bytes: &[u8]) {
        std::fs::write(root.join("gimi.json"), bytes).expect("write catalog");
        let manifest = serde_json::json!({
            "id": "owned-profile-catalog",
            "version": "1.0.0",
            "author": "EMMM test",
            "source": "https://example.invalid/catalog",
            "catalogs": { "gimi": {
                "path": "gimi.json",
                "sha256": format!("{:x}", Sha256::digest(bytes))
            } }
        });
        std::fs::write(
            root.join("manifest.json"),
            serde_json::to_vec(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");
    }

    #[test]
    fn review_derives_resource_eligibility_without_including_shaders() {
        for (kind, hash, eligible) in [
            ("shader", "0123456789abcdef", false),
            ("position_vb", "01234567", true),
        ] {
            let root = tempfile::tempdir().unwrap();
            let catalog = serde_json::json!({ "entries": [{
                "name": "Owned entry",
                "object_type": "Character",
                "runtime_targets": [{
                    "variant": "default", "resource_kind": kind, "hash": hash,
                    "provenance": {
                        "source_repo": "github.com/example/catalog",
                        "commit": "owned-fixture", "path": "gimi/owned.json"
                    }
                }]
            }] });
            write_catalog_fixture(root.path(), &serde_json::to_vec(&catalog).unwrap());

            let review = CatalogPack::load_from_root(root.path().to_path_buf())
                .unwrap()
                .review()
                .unwrap();

            assert_eq!(review.entries, 1);
            assert_eq!(review.supported_games, vec!["gimi"]);
            assert_eq!(review.has_keyviewer_targets, eligible);
        }
    }

    #[test]
    fn review_rejects_invalid_shader_targets_even_when_they_are_not_eligible() {
        let root = tempfile::tempdir().unwrap();
        let catalog = serde_json::json!({ "entries": [{
            "name": "Owned entry", "object_type": "Character",
            "runtime_targets": [{
                "variant": "default", "resource_kind": "shader", "hash": "01234567",
                "provenance": {
                    "source_repo": "github.com/example/catalog",
                    "commit": "owned-fixture", "path": "gimi/owned.json"
                }
            }]
        }] });
        write_catalog_fixture(root.path(), &serde_json::to_vec(&catalog).unwrap());
        let pack = CatalogPack::load_from_root(root.path().to_path_buf()).unwrap();

        assert!(pack
            .review()
            .unwrap_err()
            .to_string()
            .contains("shader hash"));
    }

    fn assert_reader_error_parity(pack: &CatalogPack) {
        let expected = legacy_entries_for(pack, 0)
            .expect_err("legacy reader must reject damaged fixture")
            .to_string();
        let actual = pack
            .entries_for(0)
            .expect_err("reused reader must reject damaged fixture")
            .to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn catalog_reader_preserves_errors_and_avatar_integrity() {
        let root = tempfile::tempdir().expect("create catalog pack directory");
        for bytes in [
            b"{".to_vec(),
            vec![0xff],
            vec![b' '; super::MAX_CATALOG_BYTES as usize + 1],
        ] {
            write_catalog_fixture(root.path(), &bytes);
            let pack = CatalogPack::load_from_root(root.path().to_path_buf()).expect("load pack");
            assert_reader_error_parity(&pack);
        }

        write_catalog_fixture(root.path(), b"{");
        std::fs::write(root.path().join("gimi.json"), [0xff]).expect("damage catalog checksum");
        let pack = CatalogPack::load_from_root(root.path().to_path_buf()).expect("load pack");
        assert_reader_error_parity(&pack);

        let avatar_dir = root.path().join("assets/gimi/characters");
        std::fs::create_dir_all(&avatar_dir).expect("create avatar directory");
        let png_signature = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        for (bytes, matches_checksum) in [
            (png_signature, false),
            (b"not an image".to_vec(), true),
            (Vec::new(), true),
            (vec![0; super::MAX_AVATAR_BYTES as usize + 1], true),
        ] {
            std::fs::write(avatar_dir.join("amber.png"), &bytes).expect("write damaged avatar");
            let checksum = if matches_checksum {
                format!("{:x}", Sha256::digest(&bytes))
            } else {
                "0".repeat(64)
            };
            let catalog = serde_json::to_vec(&serde_json::json!({ "entries": [{
                "name": "Amber",
                "object_type": "Character",
                "thumbnail_path": "assets/gimi/characters/amber.png",
                "metadata": { "thumbnail_asset_sha256": checksum }
            }] }))
            .expect("serialize damaged-avatar catalog");
            write_catalog_fixture(root.path(), &catalog);
            let pack = CatalogPack::load_from_root(root.path().to_path_buf()).expect("load pack");
            assert_reader_error_parity(&pack);
        }
    }

    #[test]
    #[ignore = "manual catalog reader profile with owned PNG assets"]
    fn profile_verified_catalog_byte_reuse_with_owned_avatars() {
        const ENTRY_COUNT: usize = 512;
        const ROUNDS: usize = 7;
        let root = tempfile::tempdir().expect("create owned catalog pack directory");
        let avatar_dir = root.path().join("assets/gimi/characters");
        std::fs::create_dir_all(&avatar_dir).expect("create avatar directory");
        let image = image::RgbaImage::from_fn(128, 128, |x, y| {
            let value = x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663);
            image::Rgba([value as u8, (value >> 8) as u8, (value >> 16) as u8, 255])
        });
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .expect("encode owned avatar");
        let avatar = encoded.into_inner();
        let avatar_checksum = format!("{:x}", Sha256::digest(&avatar));
        let entries = (0..ENTRY_COUNT)
            .map(|index| {
                let filename = format!("character-{index}.png");
                std::fs::write(avatar_dir.join(&filename), &avatar).expect("write owned avatar");
                serde_json::json!({
                    "name": format!("Character {index}"),
                    "object_type": "Character",
                    "aliases": [format!("char-{index}"), format!("hero-{index}")],
                    "thumbnail_path": format!("assets/gimi/characters/{filename}"),
                    "metadata": {
                        "thumbnail_asset_sha256": avatar_checksum,
                        "description": "Owned catalog metadata and representative character tags. ".repeat(16)
                    },
                    "runtime_targets": [{
                        "variant": "default",
                        "component": "body",
                        "resource_kind": "position_vb",
                        "hash": format!("{:08x}", index + 1),
                        "slot": "0",
                        "provenance": {
                            "source_repo": "github.com/example/catalog",
                            "commit": "owned-fixture",
                            "path": format!("gimi/character-{index}.json")
                        }
                    }]
                })
            })
            .collect::<Vec<_>>();
        let catalog = serde_json::to_vec(&serde_json::json!({ "entries": entries }))
            .expect("serialize owned catalog");
        write_catalog_fixture(root.path(), &catalog);
        let pack = CatalogPack::load_from_root(root.path().to_path_buf()).expect("load pack");
        let expected =
            serde_json::to_value(legacy_entries_for(&pack, 0).expect("warm legacy reader"))
                .expect("serialize expected entries");
        assert_eq!(
            serde_json::to_value(pack.entries_for(0).expect("warm reused reader")).unwrap(),
            expected
        );
        let mut baseline_ms = Vec::with_capacity(ROUNDS);
        let mut reused_ms = Vec::with_capacity(ROUNDS);
        for round in 0..ROUNDS {
            for legacy in if round % 2 == 0 {
                [true, false]
            } else {
                [false, true]
            } {
                let started = std::time::Instant::now();
                let entries = if legacy {
                    legacy_entries_for(&pack, 0)
                } else {
                    pack.entries_for(0)
                }
                .expect("read owned catalog including avatar validation");
                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                assert_eq!(entries.len(), ENTRY_COUNT);
                assert_eq!(serde_json::to_value(entries).unwrap(), expected);
                if legacy {
                    baseline_ms.push(elapsed_ms);
                } else {
                    reused_ms.push(elapsed_ms);
                }
            }
        }
        println!(
            "catalog_reader_profile entries={ENTRY_COUNT} catalog_bytes={} avatar_files={ENTRY_COUNT} avatar_total_bytes={} rounds={ROUNDS} baseline_ms={baseline_ms:?} reused_ms={reused_ms:?} avatar_validation=full cache=warm profile=debug",
            catalog.len(),
            avatar.len() * ENTRY_COUNT,
        );
    }

    #[test]
    fn catalog_parses_the_verified_bytes_when_the_file_changes_after_checksum() {
        let root = tempfile::tempdir().expect("create catalog pack directory");
        super::super::test_support::write_catalog(root.path(), "fixture-catalog");
        let pack = CatalogPack::load_from_root(root.path().to_path_buf()).expect("load pack");
        let catalog_path = root.path().join("catalog/gimi.json");

        let entries = pack
            .read_entries_with(0, || {
                std::fs::write(
                    &catalog_path,
                    br#"{"entries":[{"name":"Unverified","object_type":"Character"}]}"#,
                )
                .expect("replace catalog after checksum");
            })
            .expect("parse the verified snapshot");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Amber");
        assert!(
            pack.entries_for(0).is_err(),
            "later reads must verify again"
        );
    }

    #[test]
    fn manifest_checksum_fingerprints_the_bytes_used_to_load_the_pack() {
        let root = tempfile::tempdir().expect("create catalog pack directory");
        write_catalog_fixture(
            root.path(),
            br#"{"entries":[{"name":"Amber","object_type":"Character"}]}"#,
        );
        let manifest_path = root.path().join("manifest.json");
        let verified_bytes = std::fs::read(&manifest_path).expect("read fixture manifest");
        let pack = CatalogPack::load_from_root(root.path().to_path_buf()).expect("load pack");
        let mut replacement: serde_json::Value =
            serde_json::from_slice(&verified_bytes).expect("parse fixture manifest");
        replacement["version"] = serde_json::json!("2.0.0");
        std::fs::write(
            &manifest_path,
            serde_json::to_vec(&replacement).expect("serialize replacement manifest"),
        )
        .expect("replace manifest");

        assert_eq!(pack.version(), "1.0.0");
        assert_eq!(
            pack.manifest_checksum(),
            format!("{:x}", Sha256::digest(&verified_bytes))
        );
        assert_eq!(pack.entries_for(0).expect("read loaded catalog").len(), 1);
        let replacement_pack =
            CatalogPack::load_from_root(root.path().to_path_buf()).expect("load replacement pack");
        assert_eq!(replacement_pack.version(), "2.0.0");
        assert_ne!(
            pack.manifest_checksum(),
            replacement_pack.manifest_checksum()
        );
    }

    #[test]
    fn avatar_paths_are_limited_to_categorized_game_scoped_files() {
        assert!(is_avatar_path(Path::new(
            "assets/gimi/characters/albedo.png"
        )));
        assert!(is_avatar_path(Path::new(
            "assets/gimi/weapons/aquila-favonia.webp"
        )));
        assert!(!is_avatar_path(Path::new("assets/gimi/albedo.png")));
        assert!(!is_avatar_path(Path::new(
            "assets/gimi/characters/albedo.svg"
        )));
        assert!(!is_avatar_path(Path::new(
            "images/gimi/characters/albedo.png"
        )));
    }

    #[test]
    fn image_signatures_reject_text_disguised_as_an_avatar() {
        assert!(is_supported_avatar(&[
            0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a
        ]));
        assert!(!is_supported_avatar(b"not an image"));
    }

    #[test]
    fn catalog_entries_resolve_verified_avatar_assets_to_absolute_paths() {
        let root = tempfile::tempdir().expect("create catalog pack directory");
        let catalog_dir = root.path().join("catalog");
        let avatar_dir = root.path().join("assets/gimi/characters");
        std::fs::create_dir_all(&catalog_dir).expect("create catalog directory");
        std::fs::create_dir_all(&avatar_dir).expect("create avatar directory");
        let avatar = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let avatar_checksum = format!("{:x}", Sha256::digest(avatar));
        std::fs::write(avatar_dir.join("amber.png"), avatar).expect("write avatar");
        let catalog = serde_json::json!({
            "entries": [{
                "name": "Amber",
                "object_type": "Character",
                "thumbnail_path": "assets/gimi/characters/amber.png",
                "metadata": { "thumbnail_asset_sha256": avatar_checksum },
                "runtime_targets": []
            }]
        });
        let catalog_bytes = serde_json::to_vec(&catalog).expect("serialize catalog");
        std::fs::write(catalog_dir.join("gimi.json"), &catalog_bytes).expect("write catalog");
        let catalog_checksum = format!("{:x}", Sha256::digest(catalog_bytes));
        let manifest = serde_json::json!({
            "id": "fixture-catalog",
            "version": "1.3.0",
            "author": "EMMM test",
            "source": "https://example.invalid/catalog",
            "catalogs": { "gimi": { "path": "catalog/gimi.json", "sha256": catalog_checksum } }
        });
        std::fs::write(
            root.path().join("manifest.json"),
            serde_json::to_vec(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");

        let entries = CatalogPack::load_from_root(root.path().to_path_buf())
            .expect("load pack")
            .entries_for(0)
            .expect("load catalog entries");

        assert_eq!(entries.len(), 1);
        let expected_avatar_path = avatar_dir
            .join("amber.png")
            .canonicalize()
            .expect("canonicalize avatar path");
        assert_eq!(
            entries[0].thumbnail_path.as_deref(),
            expected_avatar_path.to_str()
        );
    }

    #[test]
    fn rejects_deprecated_manifest_format_versions() {
        let root = tempfile::tempdir().expect("create catalog pack directory");
        let manifest = serde_json::json!({
            "format_version": 2,
            "id": "fixture-catalog",
            "version": "1.3.0",
            "author": "EMMM test",
            "source": "https://example.invalid/catalog",
            "catalogs": {}
        });
        std::fs::write(
            root.path().join("manifest.json"),
            serde_json::to_vec(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");

        assert!(CatalogPack::load_from_root(root.path().to_path_buf()).is_err());
    }
}
