use crate::modules::automation::application::hotkeys::{HotkeyConfig, KeyViewerConfig};
use crate::modules::games::adapters::sqlite::game;
use crate::modules::games::domain::models::LaunchMode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct GameConfig {
    pub id: String,
    pub name: String,
    pub game_type: crate::modules::games::domain::models::GameType,
    #[serde(default)]
    pub instance_path: PathBuf,
    pub mod_path: PathBuf,
    /// Optional per-game ReadyToMove inbox. When absent, the OS Downloads default is used.
    #[serde(default)]
    pub ready_to_move_path: Option<PathBuf>,
    #[serde(default)]
    pub launch_mode: LaunchMode,
    pub game_exe: Option<PathBuf>,
    pub loader_exe: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xxmi_launcher_exe: Option<PathBuf>,
    pub launch_args: Option<String>,
    /// Transient warnings from path validation. NOT persisted to DB.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct SafetyConfig {
    pub keywords: Vec<String>,
    /// Per-game Safe Mode state. Classification remains unchanged; this only
    /// controls whether runtime apply paths may activate non-safe managed mods.
    #[serde(default)]
    pub runtime_safe_mode_by_game: BTreeMap<String, bool>,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            keywords: vec!["nsfw".into(), "nude".into(), "18+".into()],
            runtime_safe_mode_by_game: BTreeMap::new(),
        }
    }
}

impl SafetyConfig {
    pub fn runtime_safe_mode_for(&self, game_id: &str) -> bool {
        self.runtime_safe_mode_by_game
            .get(game_id)
            .copied()
            .unwrap_or(false)
    }

    pub fn set_runtime_safe_mode(&mut self, game_id: String, enabled: bool) {
        if enabled {
            self.runtime_safe_mode_by_game.insert(game_id, true);
        } else {
            self.runtime_safe_mode_by_game.remove(&game_id);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, specta::Type)]
pub struct AiConfig {
    pub enabled: bool,
    pub has_api_key: bool,
    pub base_url: Option<String>,
}

/// User-selected optional executables for integrations that EMMM does not
/// download, bundle, or manage.
#[derive(Serialize, Deserialize, Debug, Clone, Default, specta::Type)]
pub struct ExternalToolsConfig {
    #[serde(default)]
    pub mod_viewer_executable: Option<PathBuf>,
}

/// Explicit consent for the anonymous diagnostics channel. This contains no
/// endpoint, token, or user identity, all delivery configuration is build-time.
#[derive(Serialize, Deserialize, Debug, Clone, Default, specta::Type)]
pub struct DiagnosticsSettings {
    #[serde(default)]
    pub telemetry_enabled: bool,
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
    #[serde(default)]
    pub external_tools: ExternalToolsConfig,
    #[serde(default)]
    pub diagnostics: DiagnosticsSettings,
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
            external_tools: ExternalToolsConfig::default(),
            diagnostics: DiagnosticsSettings::default(),
        }
    }
}

pub fn game_row_to_config(row: game::GameRow) -> GameConfig {
    GameConfig {
        id: row.id,
        name: row.name,
        game_type: row.game_type,
        instance_path: PathBuf::from(row.path.clone()),
        mod_path: PathBuf::from(row.mods_path.unwrap_or_else(|| row.path.clone())),
        ready_to_move_path: row.ready_to_move_path.map(PathBuf::from),
        launch_mode: LaunchMode::from_persisted(&row.launch_mode),
        game_exe: row.game_exe.map(PathBuf::from),
        loader_exe: row.loader_exe.or(row.launcher_path).map(PathBuf::from),
        xxmi_launcher_exe: row.xxmi_launcher_exe.map(PathBuf::from),
        launch_args: row.launch_args,
        warnings: Vec::new(), // transient, never from DB
    }
}

pub fn config_to_game_row(config: &GameConfig) -> game::GameRow {
    let instance_path = if config.instance_path.as_os_str().is_empty() {
        &config.mod_path
    } else {
        &config.instance_path
    };

    game::GameRow {
        id: config.id.clone(),
        name: config.name.clone(),
        game_type: config.game_type,
        path: instance_path.to_string_lossy().to_string(),
        mods_path: Some(config.mod_path.to_string_lossy().to_string()),
        ready_to_move_path: config
            .ready_to_move_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        game_exe: config
            .game_exe
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        launcher_path: config
            .loader_exe
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        loader_exe: config
            .loader_exe
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        launch_mode: config.launch_mode.as_persisted().to_string(),
        xxmi_launcher_exe: config
            .xxmi_launcher_exe
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        launch_args: config.launch_args.clone(),
    }
}
