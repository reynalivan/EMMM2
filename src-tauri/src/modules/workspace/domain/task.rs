use serde::{Deserialize, Serialize};

pub const TASK_TYPE_APPLY_COLLECTION: &str = "apply_collection";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Running => "RUNNING",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
        }
    }

    pub fn from_db_value(s: &str) -> Self {
        match s {
            "RUNNING" => Self::Running,
            "COMPLETED" => Self::Completed,
            "FAILED" => Self::Failed,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecoveryAction {
    Retry,
    Rollback,
    Ignore,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PipelineTask {
    pub id: String,
    pub game_id: String,
    pub task_type: String,
    pub status: TaskStatus,
    pub target_id: Option<String>,
    pub rollback_collection_id: Option<String>,
    pub rollback_active_collection_id: Option<String>,
    pub final_active_collection_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
