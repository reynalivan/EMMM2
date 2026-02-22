use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub game_id: String,
    pub is_safe_context: bool,
    pub member_count: usize,
    pub is_last_unsaved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionDetails {
    pub collection: Collection,
    pub mod_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionPreviewMod {
    pub id: String,
    pub actual_name: String,
    pub folder_path: String,
    pub is_safe: bool,
    pub object_id: Option<String>,
    pub object_name: Option<String>,
    pub object_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCollectionInput {
    pub name: String,
    pub game_id: String,
    pub is_safe_context: bool,
    pub auto_snapshot: Option<bool>,
    pub mod_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCollectionInput {
    pub id: String,
    pub game_id: String,
    pub name: Option<String>,
    pub is_safe_context: Option<bool>,
    pub mod_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyCollectionResult {
    pub changed_count: usize,
    pub warnings: Vec<String>,
}

pub(crate) struct ModState {
    pub id: String,
    pub folder_path: String,
    pub status: String,
}
