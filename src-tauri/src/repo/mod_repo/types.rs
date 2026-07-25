use crate::domain::models::ItemStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Mod {
    pub id: String,
    pub actual_name: String,
    pub folder_path: String,
    pub status: ItemStatus,
}

/// Mod row shape consumed by disk reconcile.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReconcileModRow {
    pub id: String,
    pub folder_path: String,
    pub folder_path_key: String,
    pub actual_name: String,
    pub status: ItemStatus,
    pub object_id: Option<String>,
    pub is_safe: bool,
    pub corridor_source: Option<String>,
    pub object_type: Option<String>,
}

/// Full row rewrite for one mod during scanner sync commit, addressed by its old path key.
pub struct SyncModRowUpdate<'a> {
    pub new_id: &'a str,
    pub folder_path: &'a str,
    pub mods_path: &'a str,
    pub actual_name: &'a str,
    pub status: ItemStatus,
    pub is_safe: bool,
    pub corridor_source: &'a str,
    pub disabled_reason: Option<&'a str>,
    pub object_id: &'a str,
    pub object_type: &'a str,
    pub old_folder_path: &'a str,
    pub game_id: &'a str,
}
