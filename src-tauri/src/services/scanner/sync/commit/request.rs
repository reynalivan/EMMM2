//! Request and row types shared across the commit phases.

use sqlx::SqlitePool;
use std::path::Path;

use crate::domain::models::ItemStatus;
use crate::services::scanner::sync::types::ConfirmedScanItem;

pub struct CommitScanRequest<'a> {
    pub pool: &'a SqlitePool,
    pub game_id: &'a str,
    pub game_name: &'a str,
    pub game_type: &'a str,
    pub mods_path: &'a str,
    pub items: Vec<ConfirmedScanItem>,
    pub resource_dir: Option<&'a Path>,
    pub safe_mode_keywords: &'a [String],
    pub preserve_existing_mappings: bool,
}

/// Snapshot row shape returned by `mod_repo::get_all_mods_sync_info_tx`.
pub(super) type DbModRow = (
    String,
    String,
    ItemStatus,
    Option<String>,
    bool,
    Option<String>,
);

/// Per-commit settings threaded through every phase.
pub(super) struct CommitCtx<'a> {
    pub game_id: &'a str,
    pub mods_path: &'a str,
    pub safe_mode_keywords: &'a [String],
    pub preserve_existing_mappings: bool,
}
