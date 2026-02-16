use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub mod pin_guard;
use pin_guard::{validate_pin_format, PinGuardState, PinVerifyStatus};

const CONFIG_FILENAME: &str = "config.json";

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
}

impl Default for SafeModeConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Default to Safe Mode ON for privacy
            pin_hash: None,
            keywords: vec!["nsfw".into(), "nude".into(), "18+".into()],
            force_exclusive_mode: true,
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
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            language: "en".into(),
            games: Vec::new(),
            active_game_id: None,
            safe_mode: SafeModeConfig::default(),
        }
    }
}

pub struct ConfigService {
    config_path: PathBuf,
    settings: Mutex<AppSettings>,
    pin_guard: Mutex<PinGuardState>,
}

impl ConfigService {
    pub fn init(app_handle: &AppHandle) -> Self {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("Failed to get app data dir");

        Self::new(app_data_dir.join(CONFIG_FILENAME))
    }

    pub fn new(config_path: PathBuf) -> Self {
        let (settings, needs_save) = Self::load_from_disk(&config_path);

        // If we migrated from legacy format, save immediately to standardize "config.json"
        if needs_save {
            let _ = Self::write_settings_atomically(&config_path, &settings);
        }

        Self {
            config_path,
            settings: Mutex::new(settings),
            pin_guard: Mutex::new(PinGuardState::default()),
        }
    }

    fn load_from_disk(path: &Path) -> (AppSettings, bool) {
        if !path.exists() {
            return (AppSettings::default(), false);
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return (AppSettings::default(), false),
        };

        // 1. Try generic load
        if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
            return (settings, false);
        }

        // 2. Try Legacy Migration (Tauri Plugin Store format)
        #[derive(Deserialize)]
        struct LegacyGameConfig {
            id: String,
            name: String,
            game_type: String,
            path: PathBuf,                 // Mapped to game_exe
            mods_path: PathBuf,            // Mapped to mod_path
            launcher_path: Option<String>, // Mapped to loader_exe
            launch_args: Option<String>,
        }

        #[derive(Deserialize)]
        struct LegacyAppSettings {
            games: Option<Vec<LegacyGameConfig>>,
            active_game: Option<String>,
            safe_mode: Option<bool>,
        }

        if let Ok(legacy) = serde_json::from_str::<LegacyAppSettings>(&content) {
            let mut new_settings = AppSettings::default();

            if let Some(lg_games) = legacy.games {
                new_settings.games = lg_games
                    .into_iter()
                    .map(|lg| GameConfig {
                        id: lg.id,
                        name: lg.name,
                        game_type: lg.game_type,
                        mod_path: lg.mods_path,
                        game_exe: lg.path,
                        loader_exe: lg.launcher_path.map(PathBuf::from),
                        launch_args: lg.launch_args,
                    })
                    .collect();
            }

            if let Some(ag) = legacy.active_game {
                new_settings.active_game_id = Some(ag);
            }

            if let Some(sm) = legacy.safe_mode {
                new_settings.safe_mode.enabled = sm;
            }

            // Return migrated settings AND true to indicate "needs save"
            return (new_settings, true);
        }

        // If both fail, return default to avoid crashing, but log error
        eprintln!("Failed to parse config.json, returning default.");
        (AppSettings::default(), false)
    }

    pub fn get_settings(&self) -> AppSettings {
        self.settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn save_settings(&self, mut new_settings: AppSettings) -> Result<(), String> {
        new_settings.safe_mode.keywords = normalize_keywords(&new_settings.safe_mode.keywords);
        Self::write_settings_atomically(&self.config_path, &new_settings)?;

        // Update in-memory state
        *self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = new_settings;
        Ok(())
    }

    fn write_settings_atomically(config_path: &Path, settings: &AppSettings) -> Result<(), String> {
        let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        let tmp_path = config_path.with_extension("tmp");

        let mut file = fs::File::create(&tmp_path).map_err(|e| e.to_string())?;
        file.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;

        fs::rename(&tmp_path, config_path).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn verify_pin_status(&self, pin: &str) -> PinVerifyStatus {
        let pin_hash = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .safe_mode
            .pin_hash
            .clone();
        self.pin_guard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .verify(pin, pin_hash.as_deref())
    }

    pub fn verify_pin(&self, pin: &str) -> bool {
        self.verify_pin_status(pin).valid
    }

    pub fn set_pin(&self, pin: &str) -> Result<(), String> {
        validate_pin_format(pin)?;

        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(pin.as_bytes(), &salt)
            .map_err(|e| e.to_string())?
            .to_string();

        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        settings.safe_mode.pin_hash = Some(password_hash);

        self.save_settings(settings)?;
        self.pin_guard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .reset();

        Ok(())
    }

    pub fn set_active_game(&self, game_id: Option<String>) -> Result<(), String> {
        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        settings.active_game_id = game_id;
        self.save_settings(settings)
    }

    pub fn set_safe_mode_enabled(&self, enabled: bool) -> Result<(), String> {
        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        settings.safe_mode.enabled = enabled;
        self.save_settings(settings)
    }
}

fn normalize_keywords(keywords: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = Vec::new();
    for keyword in keywords {
        let next = keyword.trim().to_lowercase();
        if next.is_empty() || normalized.contains(&next) {
            continue;
        }
        normalized.push(next);
    }

    normalized
}
