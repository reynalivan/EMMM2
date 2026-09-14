//! Browser download DTOs crossing IPC.

/// DTO for the frontend download list.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct BrowserDownloadDto {
    pub id: String,
    pub game_id: String,
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
    pub can_resume: Option<bool>,
    pub tab_label: Option<String>,
    pub queue_order: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
}

/// A verified GameBanana page that initiated a Discover download.
///
/// This is intentionally unavailable for ordinary download URLs: a CDN or
/// signed asset URL is not reliable submission identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserGameBananaProvenance {
    pub origin_page_url: String,
    pub item_type: String,
    pub item_id: u64,
}
