use crate::domain::errors::AppError;
use crate::repo::{game_repo, settings_repo};
use sqlx::SqlitePool;

use super::models::{
    config_to_game_row, game_row_to_config, AiConfig, AppSettings, SafeModeConfig,
};
use super::ConfigService;

impl ConfigService {
    /// Load AppSettings from the SQLite database.
    pub(super) async fn load_from_db(pool: &SqlitePool) -> AppSettings {
        let kv = match settings_repo::get_all_settings(pool).await {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to load settings from DB: {e}");
                return AppSettings::default();
            }
        };

        let games = match game_repo::get_all_games(pool).await {
            Ok(rows) => rows.into_iter().map(game_row_to_config).collect(),
            Err(e) => {
                log::error!("Failed to load games from DB: {e}");
                Vec::new()
            }
        };

        let theme = kv.get("theme").cloned().unwrap_or_else(|| "dark".into());
        let language = kv.get("language").cloned().unwrap_or_else(|| "en".into());
        let active_game_id = kv.get("active_game_id").cloned();

        let safe_mode: SafeModeConfig = kv
            .get("safe_mode")
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();

        let ai: AiConfig = kv
            .get("ai")
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();

        let auto_close_launcher = kv
            .get("auto_close_launcher")
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        let hotkeys = kv
            .get("hotkeys")
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();

        let keyviewer = kv
            .get("keyviewer")
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();

        AppSettings {
            theme,
            language,
            games,
            active_game_id,
            safe_mode,
            ai,
            auto_close_launcher,
            hotkeys,
            keyviewer,
        }
    }

    /// Write the full AppSettings to the database in a single transaction.
    pub(crate) async fn write_settings_to_db(
        pool: &SqlitePool,
        settings: &AppSettings,
    ) -> Result<(), AppError> {
        settings_repo::set_setting(pool, "theme", &settings.theme).await?;
        settings_repo::set_setting(pool, "language", &settings.language).await?;

        if let Some(ref id) = settings.active_game_id {
            settings_repo::set_setting(pool, "active_game_id", id).await?;
        }

        settings_repo::set_setting(
            pool,
            "auto_close_launcher",
            &settings.auto_close_launcher.to_string(),
        )
        .await?;

        let safe_mode_json = serde_json::to_string(&settings.safe_mode)?;
        settings_repo::set_setting(pool, "safe_mode", &safe_mode_json).await?;

        let ai_json = serde_json::to_string(&settings.ai)?;
        settings_repo::set_setting(pool, "ai", &ai_json).await?;

        let hotkeys_json = serde_json::to_string(&settings.hotkeys)?;
        settings_repo::set_setting(pool, "hotkeys", &hotkeys_json).await?;

        let keyviewer_json = serde_json::to_string(&settings.keyviewer)?;
        settings_repo::set_setting(pool, "keyviewer", &keyviewer_json).await?;

        // Persist games
        for game in &settings.games {
            let row = config_to_game_row(game);
            game_repo::upsert_game(pool, &row).await?;
        }

        Ok(())
    }
}
