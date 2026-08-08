//! Browser download and import DTOs crossing IPC.

/// DTO for the frontend download list.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow, specta::Type)]
pub struct BrowserDownloadDto {
    pub id: String,
    pub session_id: Option<String>,
    pub filename: String,
    pub file_path: Option<String>,
    pub source_url: Option<String>,
    pub status: String,
    #[specta(type = Option<f64>)]
    pub bytes_total: Option<i64>,
    #[specta(type = f64)]
    pub bytes_received: i64,
    pub error_msg: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

/// DTO returned to the frontend for import queue display.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct ImportJobDto {
    pub id: String,
    pub download_id: Option<String>,
    pub game_id: Option<String>,
    pub archive_path: String,
    pub status: String,
    pub match_category: Option<String>,
    pub match_entry_key: Option<String>,
    pub match_alias_name: Option<String>,
    pub match_confidence: Option<f64>,
    pub match_reason: Option<String>,
    pub placed_path: Option<String>,
    pub error_msg: Option<String>,
    pub is_duplicate: bool,
    pub created_at: String,
    pub updated_at: String,
}
