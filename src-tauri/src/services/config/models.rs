use crate::database::settings_repo;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameConfig {
    pub id: String,
    pub name: String,
    pub game_type: String, // "Genshin", "StarRail", "ZZZ", "Wuthering"
    pub mod_path: PathBuf,
    pub game_exe: PathBuf,
    pub loader_exe: Option<PathBuf>,
    pub launch_args: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SafeModeConfig {
    pub enabled: bool,
    pub pin_hash: Option<String>,
    pub keywords: Vec<String>,
    pub force_exclusive_mode: bool,
    pub failed_attempts: Option<u8>,
    pub lockout_until_ts: Option<u64>,
}

impl Default for SafeModeConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Default to Safe Mode ON for privacy
            pin_hash: None,
            keywords: vec!["nsfw".into(), "nude".into(), "18+".into()],
            force_exclusive_mode: true,
            failed_attempts: None,
            lockout_until_ts: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AiConfig {
    pub enabled: bool,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            base_url: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    pub theme: String, // "dark", "light", "system"
    pub language: String,
    pub games: Vec<GameConfig>,
    pub active_game_id: Option<String>,
    pub safe_mode: SafeModeConfig,
    pub ai: AiConfig,
    pub auto_close_launcher: bool,
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
        }
    }
}

pub fn game_row_to_config(row: settings_repo::GameRow) -> GameConfig {
    GameConfig {
        id: row.id,
        name: row.name,
        game_type: row.game_type,
        mod_path: PathBuf::from(row.mod_path.unwrap_or_else(|| row.path.clone())),
        game_exe: PathBuf::from(row.game_exe.unwrap_or_else(|| row.path)),
        loader_exe: row.loader_exe.or(row.launcher_path).map(PathBuf::from),
        launch_args: row.launch_args,
    }
}

pub fn config_to_game_row(config: &GameConfig) -> settings_repo::GameRow {
    settings_repo::GameRow {
        id: config.id.clone(),
        name: config.name.clone(),
        game_type: config.game_type.clone(),
        path: config.game_exe.to_string_lossy().to_string(),
        mod_path: Some(config.mod_path.to_string_lossy().to_string()),
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
