use crate::modules::games::domain::models::ItemStatus;
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
    pub safety_source: Option<String>,
    pub object_type: Option<String>,
    pub filesystem_identity: Option<String>,
}
