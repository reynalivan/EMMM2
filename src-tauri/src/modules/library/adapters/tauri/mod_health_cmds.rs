use crate::modules::library::api::mod_health::{service, types::ModHealthReport};
use crate::modules::settings::application::config::ConfigService;
use crate::platform::fs::guard::validate_path;
use crate::shared::errors::AppError;
use tauri::State;

#[specta::specta]
#[tauri::command]
pub async fn analyze_mod_health(
    config: State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<ModHealthReport, AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)?.into_path_buf();
    if !mod_root.is_dir() {
        return Err(AppError::Validation(
            "Mod Health target must be a mod folder".to_string(),
        ));
    }
    let game_type = config.with_settings(|settings| {
        settings
            .games
            .iter()
            .find(|game| game.id == game_id)
            .map(|game| game.game_type)
            .ok_or_else(|| AppError::NotFound(format!("Game not found: {game_id}")))
    })?;

    tokio::task::spawn_blocking(move || service::analyze_mod_health(&mod_root, game_type)).await?
}
