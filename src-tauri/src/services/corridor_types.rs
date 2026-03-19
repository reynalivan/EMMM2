use crate::services::collections::types::RuntimeObjectState;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CorridorPreview {
    pub leaving_mods: Vec<CorridorPreviewMod>,
    pub leaving_object_states: Vec<RuntimeObjectState>,
    pub leaving_state_name: String,
    pub leaving_state_kind: CorridorPreviewStateKind,
    pub target_mods: Vec<CorridorPreviewMod>,
    pub target_object_states: Vec<RuntimeObjectState>,
    pub target_state_name: Option<String>,
    pub target_state_kind: CorridorPreviewStateKind,
    pub target_description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CorridorPreviewMod {
    pub id: String,
    pub actual_name: String,
    pub folder_path: String,
    pub is_safe: bool,
    pub object_id: Option<String>,
    pub object_name: Option<String>,
    pub object_type: Option<String>,
    pub node_type: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CorridorPreviewStateKind {
    Named,
    Unsaved,
    None,
}
