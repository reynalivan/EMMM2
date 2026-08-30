use crate::shared::errors::AppError;
use crate::modules::games::adapters::sqlite::game;
use crate::modules::system::adapters::sqlite::settings;
use sqlx::SqlitePool;

use super::models::{config_to_game_row, game_row_to_config, AiConfig, AppSettings, SafetyConfig};
use super::ConfigService;

impl ConfigService {
    /// Load AppSettings from the SQLite database.
    pub(super) async fn load_from_db(pool: &SqlitePool) -> Result<AppSettings, AppError> {
        let kv = settings::get_all_settings(pool).await?;
        let games = game::get_all_games(pool)
            .await?
            .into_iter()
            .map(game_row_to_config)
            .collect();

        let theme = kv.get("theme").cloned().unwrap_or_else(|| "dark".into());
        let language = kv.get("language").cloned().unwrap_or_else(|| "en".into());
        let active_game_id = kv.get("active_game_id").cloned();
        let revision = kv
            .get("settings_revision")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);

        let safety: SafetyConfig = kv
            .get("safety_classification")
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

        Ok(AppSettings {
            revision,
            theme,
            language,
            games,
            active_game_id,
            safety,
            ai,
            auto_close_launcher,
            hotkeys,
            keyviewer,
        })
    }

    /// Write the full AppSettings to the database in a single transaction.
    pub(crate) async fn write_settings_to_db(
        pool: &SqlitePool,
        settings: &AppSettings,
        removed_game_ids: &[String],
    ) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;
        settings::set_setting(
            &mut *tx,
            "settings_revision",
            &settings.revision.to_string(),
        )
        .await?;
        settings::set_setting(&mut *tx, "theme", &settings.theme).await?;
        settings::set_setting(&mut *tx, "language", &settings.language).await?;

        if let Some(ref id) = settings.active_game_id {
            settings::set_setting(&mut *tx, "active_game_id", id).await?;
        } else {
            settings::delete_setting(&mut *tx, "active_game_id").await?;
        }

        settings::set_setting(
            &mut *tx,
            "auto_close_launcher",
            &settings.auto_close_launcher.to_string(),
        )
        .await?;

        let safety_json = serde_json::to_string(&settings.safety)?;
        settings::set_setting(&mut *tx, "safety_classification", &safety_json).await?;

        let ai_json = serde_json::to_string(&settings.ai)?;
        settings::set_setting(&mut *tx, "ai", &ai_json).await?;

        let hotkeys_json = serde_json::to_string(&settings.hotkeys)?;
        settings::set_setting(&mut *tx, "hotkeys", &hotkeys_json).await?;

        let keyviewer_json = serde_json::to_string(&settings.keyviewer)?;
        settings::set_setting(&mut *tx, "keyviewer", &keyviewer_json).await?;

        // Persist games
        for game in &settings.games {
            let row = config_to_game_row(game);
            game::upsert_game(&mut *tx, &row).await?;
        }
        game::delete_games_by_ids(&mut tx, removed_game_ids).await?;

        tx.commit().await?;
        Ok(())
    }
}
