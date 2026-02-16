//! Epic 9 duplicate scanner contracts.
//!
//! Namespace boundary:
//! - Commands must use `dup_scan_*` (Epic 9 only).
//! - Types must use `DupScan*` (Epic 9 only).
//! - `check_duplicate_*` remains reserved for Epic 5 collision checks.
//!
//! Schema boundary:
//! - Results are **group-based** (`DupScanGroup` + `DupScanMember`).
//! - A group can contain 2..N members; not limited to pair-only reports.

use serde::{Deserialize, Serialize};

/// Streaming event contract for Epic 9 duplicate scan progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum DupScanEvent {
    /// Emitted once when a scan session starts.
    #[serde(rename_all = "camelCase")]
    Started {
        scan_id: String,
        game_id: String,
        total_folders: usize,
    },
    /// Emitted after each processed folder.
    #[serde(rename_all = "camelCase")]
    Progress {
        scan_id: String,
        processed_folders: usize,
        total_folders: usize,
        current_folder: String,
        percent: u8,
    },
    /// Emitted when a duplicate group candidate is produced.
    #[serde(rename_all = "camelCase")]
    Match {
        scan_id: String,
        group: DupScanGroup,
    },
    /// Emitted when a scan session finishes normally.
    #[serde(rename_all = "camelCase")]
    Finished {
        scan_id: String,
        total_groups: usize,
        total_members: usize,
    },
    /// Emitted when a scan session is cancelled.
    #[serde(rename_all = "camelCase")]
    Cancelled {
        scan_id: String,
        processed_folders: usize,
        total_folders: usize,
    },
}

/// Scan report root.
///
/// Group-based by design: one report contains many groups,
/// and each group contains many members.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DupScanReport {
    pub scan_id: String,
    pub game_id: String,
    pub root_path: String,
    pub total_groups: usize,
    pub total_members: usize,
    pub groups: Vec<DupScanGroup>,
}

/// A cluster of potential duplicates with 2..N members.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DupScanGroup {
    pub group_id: String,
    pub confidence_score: u8,
    pub match_reason: String,
    pub signals: Vec<DupScanSignal>,
    pub members: Vec<DupScanMember>,
}

/// One mod folder inside a duplicate group.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DupScanMember {
    pub mod_id: Option<String>,
    pub folder_path: String,
    pub display_name: String,
    pub total_size_bytes: u64,
    pub file_count: u64,
    pub confidence_score: u8,
    pub signals: Vec<DupScanSignal>,
}

/// Normalized evidence signal used in both group and member scopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DupScanSignal {
    pub key: String,
    pub detail: String,
    pub score: u8,
}
