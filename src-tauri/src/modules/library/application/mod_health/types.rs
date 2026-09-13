use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ModHealthSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ModHealthSupportLevel {
    Basic,
    Supported,
    Experimental,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ModHealthIssue {
    pub severity: ModHealthSeverity,
    pub code: String,
    pub message: String,
    pub file_path: Option<String>,
    pub section: Option<String>,
    #[specta(type = Option<f64>)]
    pub line: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ModFileManifestEntry {
    pub relative_path: String,
    #[specta(type = f64)]
    pub size_bytes: u64,
    pub blake3: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ModAssetEntry {
    pub relative_path: String,
    #[specta(type = f64)]
    pub size_bytes: u64,
    pub source_files: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct ModAssetManifestCounts {
    #[specta(type = f64)]
    pub referenced: u64,
    #[specta(type = f64)]
    pub inactive_only: u64,
    #[specta(type = f64)]
    pub orphan: u64,
    #[specta(type = f64)]
    pub external_reference: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct ModAssetManifest {
    pub referenced: Vec<ModAssetEntry>,
    pub inactive_only: Vec<ModAssetEntry>,
    pub orphan: Vec<ModAssetEntry>,
    pub external_reference: Vec<ModAssetEntry>,
    pub counts: ModAssetManifestCounts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ModControlKind {
    KeyToggle,
    MenuToggle,
    Present,
    ShapeVariable,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ModControl {
    pub kind: ModControlKind,
    pub section: String,
    pub file_path: String,
    pub key: Option<String>,
    pub back: Option<String>,
    pub variable: Option<String>,
    pub values: Vec<String>,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ModHealthReport {
    pub support_level: ModHealthSupportLevel,
    pub issues: Vec<ModHealthIssue>,
    pub manifest: ModAssetManifest,
    pub file_manifest: Vec<ModFileManifestEntry>,
    pub controls: Vec<ModControl>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ModViewerLaunchReceipt {
    pub game_id: String,
    pub mod_folder: String,
    pub file_manifest: Vec<ModFileManifestEntry>,
}
