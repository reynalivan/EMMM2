use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
pub enum WorkspaceRefreshScope {
    #[serde(rename = "workspaceChanged")]
    Workspace,
    #[serde(rename = "objectRowsChanged")]
    ObjectRows,
    #[serde(rename = "folderStructureChanged")]
    FolderStructure,
    #[serde(rename = "previewChanged")]
    Preview,
    #[serde(rename = "conflictsChanged")]
    Conflicts,
    #[serde(rename = "runtimeStateChanged")]
    RuntimeState,
    #[serde(rename = "collectionsChanged")]
    Collections,
    #[serde(rename = "dashboardChanged")]
    Dashboard,
    #[serde(rename = "activeKeybindingsChanged")]
    ActiveKeybindings,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_refresh_scope_wire_contracts() {
        for (scope, expected) in [
            (WorkspaceRefreshScope::Workspace, "workspaceChanged"),
            (WorkspaceRefreshScope::ObjectRows, "objectRowsChanged"),
            (
                WorkspaceRefreshScope::FolderStructure,
                "folderStructureChanged",
            ),
            (WorkspaceRefreshScope::Preview, "previewChanged"),
            (WorkspaceRefreshScope::Conflicts, "conflictsChanged"),
            (WorkspaceRefreshScope::RuntimeState, "runtimeStateChanged"),
            (WorkspaceRefreshScope::Collections, "collectionsChanged"),
            (WorkspaceRefreshScope::Dashboard, "dashboardChanged"),
            (
                WorkspaceRefreshScope::ActiveKeybindings,
                "activeKeybindingsChanged",
            ),
        ] {
            assert_eq!(
                serde_json::to_value(scope).unwrap(),
                serde_json::json!(expected)
            );
        }
    }
}
