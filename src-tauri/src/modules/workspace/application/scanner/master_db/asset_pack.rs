use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use image::ImageReader;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::Manager;

use crate::modules::matching::application::deep_matcher::{DbEntry, MasterDb};
use crate::shared::errors::ScannerError;

pub(crate) const PACK_DIRECTORY: &str = "asset-pack";
pub(crate) const MANIFEST_FILE: &str = "manifest.json";
const FORMAT_VERSION: u32 = 1;
const MAX_CATALOG_BYTES: u64 = 10 * 1024 * 1024;
const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 4096;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct CatalogPackStatus {
    pub state: String,
    pub pack_id: Option<String>,
    pub version: Option<String>,
    pub message: Option<String>,
    pub entries: usize,
    pub missing_assets: usize,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct CatalogPackRefreshResult {
    pub state: String,
    pub entries: usize,
    pub thumbnails_applied: usize,
    pub missing_assets: usize,
    pub skipped_invalid_files: usize,
}

#[derive(Deserialize)]
struct Manifest {
    format_version: u32,
    id: String,
    version: String,
    author: String,
    source: String,
    license: String,
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
        let manifest: Manifest =
            serde_json::from_slice(&std::fs::read(&manifest_path)?).map_err(|error| {
                ScannerError::Parse {
                    what: "catalog asset-pack manifest".to_string(),
                    detail: error.to_string(),
                }
            })?;
        if manifest.format_version != FORMAT_VERSION
            || manifest.id.trim().is_empty()
            || manifest.version.trim().is_empty()
            || manifest.author.trim().is_empty()
            || manifest.source.trim().is_empty()
            || manifest.license.trim().is_empty()
        {
            return Err(ScannerError::Parse {
                what: "catalog asset-pack manifest".to_string(),
                detail: "missing required metadata or unsupported format version".to_string(),
            });
        }
        Ok(Self { root, manifest })
    }

    pub(crate) fn id(&self) -> &str {
        &self.manifest.id
    }

    pub(crate) fn version(&self) -> &str {
        &self.manifest.version
    }

    pub(crate) fn entries_for(&self, game_type: i32) -> Result<Vec<DbEntry>, ScannerError> {
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
        if sha256_file(&path)? != catalog.sha256.to_ascii_lowercase() {
            return Err(ScannerError::Io(
                "Catalog checksum does not match its manifest".to_string(),
            ));
        }
        let json = std::fs::read_to_string(path)?;
        let mut entries = MasterDb::from_json(&json)?.entries;
        for entry in &mut entries {
            self.resolve_entry(entry)?;
        }
        Ok(entries)
    }

    fn resolve_entry(&self, entry: &mut DbEntry) -> Result<(), ScannerError> {
        entry.thumbnail_path = resolve_image(&self.root, entry.thumbnail_path.take())?;
        for skin in &mut entry.custom_skins {
            skin.thumbnail_skin_path = resolve_image(&self.root, skin.thumbnail_skin_path.take())?;
        }
        Ok(())
    }

    pub(crate) fn status(&self) -> Result<CatalogPackStatus, ScannerError> {
        let mut entries = 0;
        let mut missing_assets = 0;
        for game_type in 0..=4 {
            for entry in self.entries_for(game_type)? {
                entries += 1;
                if entry.thumbnail_path.is_none() {
                    missing_assets += 1;
                }
            }
        }
        Ok(CatalogPackStatus {
            state: if missing_assets == 0 {
                "ready"
            } else {
                "partial"
            }
            .to_string(),
            pack_id: Some(self.manifest.id.clone()),
            version: Some(self.manifest.version.clone()),
            message: None,
            entries,
            missing_assets,
        })
    }
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

fn resolve_image(root: &Path, path: Option<String>) -> Result<Option<String>, ScannerError> {
    let Some(path) = path else {
        return Ok(None);
    };
    let image = resolve_child(root, &path, "image")?;
    let extension = image
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if !matches!(extension.as_deref(), Some("png" | "jpg" | "jpeg" | "webp")) {
        return Err(ScannerError::Io(
            "Catalog image type is not supported".to_string(),
        ));
    }
    if std::fs::metadata(&image)?.len() > MAX_IMAGE_BYTES {
        return Err(ScannerError::Io(
            "Catalog image exceeds the size limit".to_string(),
        ));
    }
    let dimensions = ImageReader::open(&image)
        .map_err(|error| ScannerError::Io(error.to_string()))?
        .with_guessed_format()
        .map_err(|error| ScannerError::Io(error.to_string()))?
        .into_dimensions()
        .map_err(|error| ScannerError::Io(error.to_string()))?;
    if dimensions.0 > MAX_IMAGE_DIMENSION || dimensions.1 > MAX_IMAGE_DIMENSION {
        return Err(ScannerError::Io(
            "Catalog image dimensions exceed the limit".to_string(),
        ));
    }
    Ok(Some(image.to_string_lossy().into_owned()))
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
            missing_assets: 0,
        },
        Err(error) => CatalogPackStatus {
            state: "invalid".to_string(),
            pack_id: None,
            version: None,
            message: Some(error.to_string()),
            entries: 0,
            missing_assets: 0,
        },
    }
}
