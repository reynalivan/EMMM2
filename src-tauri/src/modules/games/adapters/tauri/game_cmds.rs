use crate::modules::games::application::game::validator;
use crate::modules::games::domain::models::{GameType, LaunchMode};
use crate::modules::settings::application::config::{ConfigService, GameConfig};
use crate::shared::errors::AppError;
use crate::shared::path_key::folder_path_key;
use std::path::{Path, PathBuf};
use std::time::Instant;
use uuid::Uuid;

fn canonical_game_path_key(path: &str) -> String {
    folder_path_key(path, None)
}

#[specta::specta]
#[tauri::command]
pub fn resolve_game_folder(
    path: String,
) -> Result<crate::modules::games::domain::models::GameInfo, AppError> {
    validator::validate_instance(Path::new(&path)).map(|(info, _warnings)| info)
}

/// Auto-detect games by scanning an XXMI root folder.
/// Validates each subfolder, saves valid games to ConfigService.
#[specta::specta]
#[tauri::command]
pub async fn auto_detect_games(
    state: tauri::State<'_, ConfigService>,
    root_path: String,
) -> Result<Vec<GameConfig>, AppError> {
    auto_detect_games_inner(&state, &root_path).await
}

pub async fn auto_detect_games_inner(
    service: &ConfigService,
    root_path: &str,
) -> Result<Vec<GameConfig>, AppError> {
    let root = Path::new(root_path);
    if !root.exists() {
        return Err(AppError::NotFound(format!(
            "Path does not exist: {root_path}"
        )));
    }

    let found = validator::scan_xxmi_root(root);
    if found.is_empty() {
        return Err(AppError::NotFound(
            "No valid 3DMigoto instances found in standard XXMI folders.".to_string(),
        ));
    }

    let mut new_games: Vec<GameConfig> = Vec::new();
    let settings = service.get_settings();

    for detected in &found {
        let id = Uuid::new_v4().to_string();
        let game = GameConfig {
            id,
            name: detected.game_type.display_name().to_string(),
            game_type: detected.game_type,
            instance_path: PathBuf::from(&detected.info.path),
            mod_path: PathBuf::from(&detected.info.mods_path),
            ready_to_move_path: None,
            launch_mode: detected.launch_mode,
            game_exe: None,
            loader_exe: (detected.launch_mode == LaunchMode::Standalone)
                .then(|| detected.info.launcher_path.as_ref().map(PathBuf::from))
                .flatten(),
            xxmi_launcher_exe: detected.xxmi_launcher_exe.clone(),
            launch_args: None,
            warnings: detected.warnings.clone(),
        };

        // Check for duplicates
        let normalized_path = canonical_game_path_key(&game.instance_path.to_string_lossy());
        let is_duplicate = settings.games.iter().any(|g| {
            canonical_game_path_key(&g.instance_path.to_string_lossy()) == normalized_path
        });

        if !is_duplicate {
            new_games.push(game);
        }
    }

    log::info!(
        "Auto-detect complete: detected {} new game(s)",
        new_games.len()
    );

    Ok(new_games)
}

/// Manually add a single game by path and type.
/// Validates the folder, checks for duplicates, saves to ConfigService.
#[specta::specta]
#[tauri::command]
pub async fn add_game_manual(
    state: tauri::State<'_, ConfigService>,
    game_type: String,
    path: String,
) -> Result<GameConfig, AppError> {
    add_game_manual_inner(&state, &game_type, &path).await
}

pub async fn add_game_manual_inner(
    service: &ConfigService,
    game_type: &str,
    path: &str,
) -> Result<GameConfig, AppError> {
    // Parse game type
    let gt: GameType = game_type.parse().map_err(AppError::Validation)?;
    let folder = Path::new(path);

    // Validate folder structure (returns warnings, not hard errors, for missing files)
    let (info, mut warnings) = validator::validate_instance(folder)?;
    warnings.push(
        "Game executable is not configured. Select the game's .exe before using Play.".to_string(),
    );

    let settings = service.get_settings();

    // Duplicate path check (TC-1.5-01, NC-1.3-02)
    let normalized_path = canonical_game_path_key(&info.path);
    for g in &settings.games {
        let existing_normalized = canonical_game_path_key(&g.instance_path.to_string_lossy());
        if existing_normalized == normalized_path {
            return Err(AppError::Validation(format!(
                "This game path is already registered as '{}'.",
                g.name
            )));
        }
    }

    let id = Uuid::new_v4().to_string();
    let game = GameConfig {
        id,
        name: gt.display_name().to_string(),
        game_type: gt,
        instance_path: PathBuf::from(&info.path),
        mod_path: PathBuf::from(&info.mods_path),
        ready_to_move_path: None,
        launch_mode: LaunchMode::Standalone,
        game_exe: None,
        loader_exe: info.launcher_path.map(PathBuf::from),
        xxmi_launcher_exe: None,
        launch_args: None,
        warnings,
    };

    if game.warnings.is_empty() {
        log::info!("Game added manually: {} ({})", game.name, game.game_type);
    } else {
        log::warn!(
            "Game added manually with {} warning(s): {} ({})",
            game.warnings.len(),
            game.name,
            game.game_type
        );
    }

    Ok(game)
}

/// Save confirmed games from onboarding into the database/settings
#[specta::specta]
#[tauri::command]
pub async fn save_onboarding_games(
    state: tauri::State<'_, ConfigService>,
    telemetry: tauri::State<'_, crate::modules::system::application::telemetry::TelemetryStore>,
    games: Vec<GameConfig>,
) -> Result<(), AppError> {
    let diagnostics_enabled = state.get_settings().diagnostics.telemetry_enabled;
    let started_at = Instant::now();
    if diagnostics_enabled {
        let _ = telemetry
            .record_rollup(
                env!("CARGO_PKG_VERSION"),
                crate::modules::system::application::telemetry::TelemetryEvent::new(
                    crate::modules::system::application::telemetry::TelemetryOperation::Onboarding,
                    crate::modules::system::application::telemetry::TelemetryOutcome::Started,
                    crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                ),
                chrono::Utc::now(),
            )
            .await;
    }
    let result = save_onboarding_games_inner(&state, games).await;
    if diagnostics_enabled && result.is_ok() {
        let event = crate::modules::system::application::telemetry::TelemetryEvent::new(
            crate::modules::system::application::telemetry::TelemetryOperation::Onboarding,
            crate::modules::system::application::telemetry::TelemetryOutcome::Success,
            crate::modules::system::application::telemetry::TelemetryErrorCode::None,
        )
        .with_duration(started_at.elapsed());
        let _ = telemetry
            .record_rollup(env!("CARGO_PKG_VERSION"), event, chrono::Utc::now())
            .await;
    }
    result
}

pub async fn save_onboarding_games_inner(
    service: &ConfigService,
    games: Vec<GameConfig>,
) -> Result<(), AppError> {
    let added_count = service.update_settings(move |settings| {
        let mut added_count = 0;
        for game in games {
            // Double check against the latest committed list, not the snapshot
            // from when onboarding detection started.
            let normalized_path = canonical_game_path_key(&game.instance_path.to_string_lossy());

            let is_duplicate = settings.games.iter().any(|configured| {
                canonical_game_path_key(&configured.instance_path.to_string_lossy())
                    == normalized_path
            });

            if !is_duplicate {
                settings.games.push(game);
                added_count += 1;
            }
        }
        Ok(added_count)
    })?;
    log::info!("Onboarding complete: saved {} game(s)", added_count);

    Ok(())
}

/// Get all configured games.
#[specta::specta]
#[tauri::command]
pub async fn get_games(
    state: tauri::State<'_, ConfigService>,
) -> Result<Vec<GameConfig>, AppError> {
    Ok(state.get_settings().games)
}

/// Launch a game through its configured XXMI or standalone topology.
#[specta::specta]
#[tauri::command]
pub async fn launch_game(
    state: tauri::State<'_, ConfigService>,
    telemetry: tauri::State<'_, crate::modules::system::application::telemetry::TelemetryStore>,
    game_id: String,
) -> Result<(), AppError> {
    let diagnostics_enabled = state.get_settings().diagnostics.telemetry_enabled;
    let started_at = Instant::now();
    let games = get_games(state).await?;
    let game = games
        .into_iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| AppError::NotFound("Game config not found".to_string()))?;

    let result = match game.launch_mode {
        LaunchMode::XxmiManaged => launch_xxmi_managed_game(&game),
        LaunchMode::Standalone => launch_standalone_game(&game).await,
    };
    if diagnostics_enabled && result.is_ok() {
        let event = crate::modules::system::application::telemetry::TelemetryEvent::new(
            crate::modules::system::application::telemetry::TelemetryOperation::Launch,
            crate::modules::system::application::telemetry::TelemetryOutcome::Success,
            crate::modules::system::application::telemetry::TelemetryErrorCode::None,
        )
        .with_duration(started_at.elapsed());
        let _ = telemetry
            .record_rollup(env!("CARGO_PKG_VERSION"), event, chrono::Utc::now())
            .await;
    }
    result
}

fn launch_xxmi_managed_game(game: &GameConfig) -> Result<(), AppError> {
    let launcher_path = game.xxmi_launcher_exe.as_ref().ok_or_else(|| {
        AppError::Validation("XXMI launcher is not configured for this game.".to_string())
    })?;
    ensure_executable_file(launcher_path, "XXMI launcher")?;

    let launcher_dir = launcher_path.parent().unwrap_or(launcher_path);
    crate::platform::process::launch_elevated_with_args(
        launcher_path,
        launcher_dir,
        &xxmi_launch_args(game.game_type),
    )
    .map_err(|error| AppError::Io(format!("Failed to start XXMI launcher: {error}")))?;

    Ok(())
}

fn xxmi_launch_args(game_type: GameType) -> [String; 3] {
    [
        "--nogui".to_string(),
        "--xxmi".to_string(),
        game_type.to_string(),
    ]
}

async fn launch_standalone_game(game: &GameConfig) -> Result<(), AppError> {
    use sysinfo::System;

    let game_path = game.game_exe.as_ref().ok_or_else(|| {
        AppError::Validation(
            "Game executable is not configured. Select the game's .exe before using Play."
                .to_string(),
        )
    })?;
    ensure_executable_file(game_path, "Game executable")?;

    if let Some(launcher_path) = &game.loader_exe {
        ensure_executable_file(launcher_path, "Loader")?;

        let mut sys = System::new_all();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let launcher_name = launcher_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        let is_loader_running = sys.processes().values().any(|process| {
            process
                .name()
                .to_string_lossy()
                .eq_ignore_ascii_case(&launcher_name)
        });

        if !is_loader_running {
            log::info!("Starting Loader: {}", launcher_path.display());

            let launcher_dir = launcher_path.parent().unwrap_or(launcher_path);

            #[cfg(target_os = "windows")]
            {
                crate::platform::process::launch_elevated(launcher_path, launcher_dir)?;
            }

            #[cfg(not(target_os = "windows"))]
            {
                std::process::Command::new(launcher_path)
                    .current_dir(launcher_dir)
                    .spawn()
                    .map_err(|error| AppError::Io(format!("Failed to start loader: {error}")))?;
            }

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    log::info!("Starting Game: {}", game_path.display());
    let game_dir = game_path.parent().unwrap_or(game_path);

    let mut cmd = std::process::Command::new(game_path);
    cmd.current_dir(game_dir);

    // Apply args
    if let Some(args_str) = &game.launch_args {
        if !args_str.trim().is_empty() {
            for arg in args_str.split_whitespace() {
                cmd.arg(arg);
            }
        }
    }

    cmd.spawn()
        .map_err(|e| AppError::Io(format!("Failed to start game: {e}")))?;

    Ok(())
}

fn ensure_executable_file(path: &Path, label: &str) -> Result<(), AppError> {
    if !path.is_file() {
        return Err(AppError::NotFound(format!(
            "{label} executable not found at: {}",
            path.display()
        )));
    }
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return Err(AppError::Validation(format!(
            "{label} must be a .exe file: {}",
            path.display()
        )));
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests/game_cmds_tests.rs"]
mod tests;
