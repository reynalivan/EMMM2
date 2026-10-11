use serde::{Deserialize, Serialize};

use super::HotkeyConfig;
use crate::modules::reconciliation::api::disk_reconcile::types::RuntimeSyncStatus;

pub const HOTKEY_NOTICE_LIFETIME_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyNoticeKind {
    CollectionApplied,
    SafeModeChanged,
    NoPresets,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyActionOutcome {
    pub kind: HotkeyNoticeKind,
    pub previous_collection: Option<String>,
    pub collection: Option<String>,
    pub safe_mode: bool,
    pub message: Option<String>,
    pub runtime_generation: Option<u64>,
    pub log_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct HotkeyNotice {
    pub id: String,
    #[specta(type = f64)]
    pub sequence: u64,
    #[specta(type = f64)]
    pub settings_revision: u64,
    pub game_id: String,
    pub game_name: String,
    pub mods_path: String,
    pub root_identity: String,
    #[specta(type = f64)]
    pub created_at_ms: u64,
    #[specta(type = f64)]
    pub expires_at_ms: u64,
    pub kind: HotkeyNoticeKind,
    pub previous_collection: Option<String>,
    pub collection: Option<String>,
    pub safe_mode: bool,
    pub message: Option<String>,
    #[specta(type = Option<f64>)]
    pub runtime_generation: Option<u64>,
    pub runtime_status: Option<RuntimeSyncStatus>,
    pub reload_binding: Option<String>,
    pub hotkeys: HotkeyConfig,
    pub keyviewer_enabled: bool,
    pub language: String,
    pub theme: String,
}

pub fn unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
