use crate::modules::automation::application::hotkeys::{HotkeyConfig, KeyViewerConfig};
use crate::modules::games::adapters::sqlite::game;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct GameConfig {
    pub id: String,
    pub name: String,
    pub game_type: crate::modules::games::domain::models::GameType,
    pub mod_path: PathBuf,
    /// Optional per-game ReadyToMove inbox. When absent, the OS Downloads default is used.
    #[serde(default)]
    pub ready_to_move_path: Option<PathBuf>,
    pub game_exe: PathBuf,
    pub loader_exe: Option<PathBuf>,
    pub launch_args: Option<String>,
    /// Transient warnings from path validation. NOT persisted to DB.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct SafetyConfig {
    pub keywords: Vec<String>,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            keywords: vec!["nsfw".into(), "nude".into(), "18+".into()],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, specta::Type)]
pub struct AiConfig {
    pub enabled: bool,
    pub has_api_key: bool,
    pub base_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct AppSettings {
    /// Optimistic-concurrency token for whole-settings IPC saves.
    #[serde(default)]
    #[specta(type = f64)]
    pub revision: u64,
    pub theme: String, // "dark", "light", "system"
    pub language: String,
    pub games: Vec<GameConfig>,
    pub active_game_id: Option<String>,
    pub safety: SafetyConfig,
    pub ai: AiConfig,
    pub auto_close_launcher: bool,
    #[serde(default)]
    pub hotkeys: HotkeyConfig,
    #[serde(default)]
    pub keyviewer: KeyViewerConfig,
}

impl AppSettings {
    /// The game `active_game_id` points at, if it is still configured.
    /// The id and the games list can drift, so every caller must handle `None`.
    pub fn active_game(&self) -> Option<&GameConfig> {
        let active_game_id = self.active_game_id.as_ref()?;
        self.games.iter().find(|game| &game.id == active_game_id)
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            revision: 0,
            theme: "dark".into(),
            language: "en".into(),
            games: Vec::new(),
            active_game_id: None,
            safety: SafetyConfig::default(),
            ai: AiConfig::default(),
            auto_close_launcher: false,
            hotkeys: HotkeyConfig::default(),
            keyviewer: KeyViewerConfig::default(),
        }
    }
}

pub fn game_row_to_config(row: game::GameRow) -> GameConfig {
    GameConfig {
        id: row.id,
        name: row.name,
        game_type: row.game_type,
        mod_path: PathBuf::from(row.mods_path.unwrap_or_else(|| row.path.clone())),
        ready_to_move_path: row.ready_to_move_path.map(PathBuf::from),
        game_exe: PathBuf::from(row.game_exe.unwrap_or(row.path)),
        loader_exe: row.loader_exe.or(row.launcher_path).map(PathBuf::from),
        launch_args: row.launch_args,
        warnings: Vec::new(), // transient, never from DB
    }
}

pub fn config_to_game_row(config: &GameConfig) -> game::GameRow {
    game::GameRow {
        id: config.id.clone(),
        name: config.name.clone(),
        game_type: config.game_type,
        path: config.game_exe.to_string_lossy().to_string(),
        mods_path: Some(config.mod_path.to_string_lossy().to_string()),
        ready_to_move_path: config
            .ready_to_move_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
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
