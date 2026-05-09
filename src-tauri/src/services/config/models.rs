use crate::repo::game_repo;
use crate::services::hotkeys::{HotkeyConfig, KeyViewerConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct GameConfig {
    pub id: String,
    pub name: String,
    pub game_type: crate::database::models::GameType,
    pub mod_path: PathBuf,
    pub game_exe: PathBuf,
    pub loader_exe: Option<PathBuf>,
    pub launch_args: Option<String>,
    /// Transient warnings from path validation. NOT persisted to DB.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct SafeModeConfig {
    pub enabled: bool,
    pub pin_hash: Option<String>,
    pub recovery_code_hash: Option<String>,
    pub keywords: Vec<String>,
    pub force_exclusive_mode: bool,
    pub failed_attempts: Option<u8>,
    #[specta(type = Option<f64>)]
    pub lockout_until_ts: Option<u64>,
}

impl Default for SafeModeConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Default to Safe Mode ON for privacy
            pin_hash: None,
            recovery_code_hash: None,
            keywords: vec!["nsfw".into(), "nude".into(), "18+".into()],
            force_exclusive_mode: true,
            failed_attempts: None,
            lockout_until_ts: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, specta::Type)]
pub struct AiConfig {
    pub enabled: bool,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct AppSettings {
    pub theme: String, // "dark", "light", "system"
    pub language: String,
    pub games: Vec<GameConfig>,
    pub active_game_id: Option<String>,
    pub safe_mode: SafeModeConfig,
    pub ai: AiConfig,
    pub auto_close_launcher: bool,
    #[serde(default)]
    pub hotkeys: HotkeyConfig,
    #[serde(default)]
    pub keyviewer: KeyViewerConfig,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            language: "en".into(),
            games: Vec::new(),
            active_game_id: None,
            safe_mode: SafeModeConfig::default(),
            ai: AiConfig::default(),
            auto_close_launcher: false,
            hotkeys: HotkeyConfig::default(),
            keyviewer: KeyViewerConfig::default(),
        }
    }
}

pub fn game_row_to_config(row: game_repo::GameRow) -> GameConfig {
    GameConfig {
        id: row.id,
        name: row.name,
        game_type: row.game_type,
        mod_path: PathBuf::from(row.mods_path.unwrap_or_else(|| row.path.clone())),
        game_exe: PathBuf::from(row.game_exe.unwrap_or(row.path)),
        loader_exe: row.loader_exe.or(row.launcher_path).map(PathBuf::from),
        launch_args: row.launch_args,
        warnings: Vec::new(), // transient, never from DB
    }
}

pub fn config_to_game_row(config: &GameConfig) -> game_repo::GameRow {
    game_repo::GameRow {
        id: config.id.clone(),
        name: config.name.clone(),
        game_type: config.game_type,
        path: config.game_exe.to_string_lossy().to_string(),
        mods_path: Some(config.mod_path.to_string_lossy().to_string()),
        game_exe: Some(config.game_exe.to_string_lossy().to_string()),
        launcher_path: config
            .loader_exe
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        loader_exe: config
            .loader_exe
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        launch_args: config.launch_args.clone(),
    }
}
