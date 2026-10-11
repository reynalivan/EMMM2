use serde::Serialize;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum FileLockDiscoveryStatus {
    Complete,
    Partial,
    Unknown,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FileLockProcess {
    pub process_ref: String,
    pub pid: u32,
    pub name: String,
    pub executable: Option<String>,
    pub started_at: String,
    pub application_type: i32,
    pub session_id: u32,
    pub service: Option<String>,
    pub can_close: bool,
    pub can_terminate: bool,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FileLockReport {
    pub recovery_id: String,
    pub path: String,
    pub status: FileLockDiscoveryStatus,
    pub message: Option<String>,
    pub processes: Vec<FileLockProcess>,
    pub action_outcome: Option<String>,
}
