pub mod dup_ignore_cmds;
pub mod dup_resolve_cmds;
pub mod dup_scan_cmds;

use crate::types::dup_scan::DupScanGroup;

#[derive(Clone, serde::Serialize, tauri_specta::Event, specta::Type)]
pub struct DupScanMatchedEvent(pub DupScanGroup);

#[cfg(test)]
mod tests;
