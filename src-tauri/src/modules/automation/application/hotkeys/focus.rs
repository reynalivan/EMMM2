use crate::modules::settings::application::config::AppSettings;

pub fn is_active_game_focused(settings: &AppSettings) -> bool {
    let Some(active_game) = settings.active_game() else {
        return false;
    };
    let process = crate::modules::games::api::executable_profiles::resolve_process_executable(
        active_game.game_type,
        active_game.game_exe.as_deref(),
    );
    crate::modules::system::application::game_detector::is_game_focused(process.as_deref())
}
