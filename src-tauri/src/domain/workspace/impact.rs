use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceRefreshScope {
    WorkspaceChanged,
    ObjectRowsChanged,
    FolderStructureChanged,
    PreviewChanged,
    ConflictsChanged,
    CorridorChanged,
    CollectionsChanged,
    DashboardChanged,
    ActiveKeybindingsChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WorkspacePathRewrite {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceImpact {
    pub rewrites: Vec<WorkspacePathRewrite>,
    pub changed_object_ids: Vec<String>,
    pub changed_folder_paths: Vec<String>,
    pub refresh_scopes: Vec<WorkspaceRefreshScope>,
    pub warnings: Vec<String>,
}
