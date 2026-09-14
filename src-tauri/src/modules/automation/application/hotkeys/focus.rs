use crate::modules::settings::application::config::AppSettings;

pub fn is_active_game_focused(settings: &AppSettings) -> bool {
    let Some(active_game) = settings.active_game() else {
        return false;
    };
    crate::modules::system::application::game_detector::is_game_focused(
        active_game.game_exe.as_deref(),
    )
}
