use super::models::GameType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum GameExecutableSource {
    HoyoplayRegistry,
    HoyoplayConfig,
    KuroConfig,
    XxmiConfig,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum GameExecutableIssueCode {
    AccessDenied,
    MalformedConfig,
    ReadFailed,
    ResourceLimit,
    DeadlineExceeded,
    InvalidLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GameExecutableDiscoveryIssue {
    pub source: GameExecutableSource,
    pub code: GameExecutableIssueCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GameExecutableCandidate {
    pub game_type: GameType,
    pub executable_path: String,
    pub install_root: String,
    pub process_executable_path: String,
    pub sources: Vec<GameExecutableSource>,
    pub region: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DiscoverGameExecutablesRequest {
    pub game_types: Vec<GameType>,
    #[serde(default)]
    pub xxmi_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GameExecutableDiscoveryResult {
    pub candidates: Vec<GameExecutableCandidate>,
    pub issues: Vec<GameExecutableDiscoveryIssue>,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ValidateGameExecutableRequest {
    pub game_type: GameType,
    pub executable_path: String,
}
