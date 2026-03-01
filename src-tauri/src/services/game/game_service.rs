//! High-level game lifecycle service.
//!
//! Separates the combined "save settings + delete from DB" concern
//! that used to live directly inside game_cmds.

use crate::services::config::ConfigService;

/// Remove a game from both ConfigService (JSON settings) and the `games` DB table.
/// Returns Ok(()) on success; error string on failure.
pub async fn remove_game_service(config: &ConfigService, game_id: &str) -> Result<(), String> {
    // 1. Remove from in-memory settings + persist to JSON
    let mut settings = config.get_settings();
    settings.games.retain(|g| g.id != game_id);
    config.save_settings(settings)?;

    // 2. Remove from DB (the config file only contains shallow info;
    //    the DB row also stores object/mod relationships that need cleaning)
    crate::database::game_repo::delete_game(config.pool(), game_id)
        .await
        .map_err(|e| format!("Failed to remove game from db: {e}"))?;

    log::info!("Game removed: {}", game_id);
    Ok(())
}
