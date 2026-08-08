//! Conflict and duplicate bookkeeping shapes crossing IPC.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, sqlx::FromRow)]
pub struct IgnoredConflict {
    pub id: String,
    pub game_id: String,
    pub object_id: String,
    pub object_name: Option<String>,
    pub mod_ids: String, // JSON array
    #[sqlx(skip)]
    #[serde(default)]
    pub mod_names: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize, specta::Type, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct WhitelistEntry {
    pub id: String,
    pub folder_a_id: String,
    pub folder_b_id: String,
    pub folder_a_name: String,
    pub folder_b_name: String,
    pub reason: String,
    pub ignored_at: String,
}
