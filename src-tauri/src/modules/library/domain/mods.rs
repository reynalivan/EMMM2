use serde::{Deserialize, Serialize};

/// Info about an enabled duplicate/conflicting mod for a given object.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DuplicateModInfo {
    pub mod_id: String,
    pub object_id: String,
    pub folder_path: String,
    pub actual_name: String,
    pub is_variant: bool,
    pub parent_path: String,
}
