use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceRefreshScope {
    WorkspaceChanged,
    ObjectRowsChanged,
    FolderStructureChanged,
    FolderMetadataChanged,
    PreviewChanged,
    ThumbnailChanged,
    ConflictsChanged,
    CorridorChanged,
    CollectionsChanged,
    DashboardChanged,
    ActiveKeybindingsChanged,
    TrashChanged,
    SettingsChanged,
    BrowserDownloadsChanged,
    BrowserImportQueueChanged,
    BrowserHomepageChanged,
    DedupChanged,
    DedupReportChanged,
    ScannerChanged,
    PinsChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WorkspacePathRewrite {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceImpact {
    pub rewrites: Vec<WorkspacePathRewrite>,
    pub cleared_targets: Vec<String>,
    pub changed_object_ids: Vec<String>,
    pub changed_folder_paths: Vec<String>,
    pub refresh_scopes: Vec<WorkspaceRefreshScope>,
    pub projection_dirty: bool,
    pub warnings: Vec<String>,
}
