use crate::modules::games::domain::models::ItemStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Mod {
    pub id: String,
    pub actual_name: String,
    pub folder_path: String,
    pub status: ItemStatus,
}

/// An object-owned mod row used by the randomizer. The row deliberately
/// carries its owning game and Object metadata so callers can validate a
/// proposal again immediately before mutating the workspace.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RandomizerModCandidate {
    pub id: String,
    pub object_id: String,
    pub object_name: String,
    pub object_type: Option<String>,
    pub randomizer_mode: Option<String>,
    pub actual_name: String,
    pub folder_path: String,
    pub status: ItemStatus,
    pub is_safe: bool,
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
    pub size_bytes: i64,
}
