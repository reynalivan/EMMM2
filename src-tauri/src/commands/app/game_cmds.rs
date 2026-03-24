use crate::database::models::GameType;
use crate::services::config::{ConfigService, GameConfig};
use crate::services::game::validator;
use crate::services::path_key::folder_path_key;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn canonical_game_path_key(path: &str) -> String {
    folder_path_key(path, None)
}

/// Auto-detect games by scanning an XXMI root folder.
/// Validates each subfolder, saves valid games to ConfigService.
#[specta::specta]
#[tauri::command]
pub async fn auto_detect_games(
    state: tauri::State<'_, ConfigService>,
    root_path: String,
) -> Result<Vec<GameConfig>, String> {
    auto_detect_games_inner(&state, &root_path).await
}

pub async fn auto_detect_games_inner(
    service: &ConfigService,
    root_path: &str,
) -> Result<Vec<GameConfig>, String> {
    let root = Path::new(root_path);
    if !root.exists() {
        return Err(format!("Path does not exist: {root_path}"));
    }

    let found = validator::scan_xxmi_root(root);
    if found.is_empty() {
        return Err("No valid 3DMigoto instances found in standard XXMI folders.".to_string());
    }

    let mut new_games: Vec<GameConfig> = Vec::new();
    let settings = service.get_settings();

    for (info, warnings, game_type_str, display_name) in &found {
        let id = Uuid::new_v4().to_string();
        let game = GameConfig {
            id,
            name: display_name.to_string(),
            game_type: game_type_str.parse().unwrap_or(GameType::GIMI),
            mod_path: PathBuf::from(&info.mods_path),
            game_exe: PathBuf::from(&info.path),
            loader_exe: Some(PathBuf::from(&info.launcher_path)),
            launch_args: None,
            warnings: warnings.clone(),
        };

        // Check for duplicates
        let normalized_path = canonical_game_path_key(&game.game_exe.to_string_lossy());
        let is_duplicate = settings
            .games
            .iter()
            .any(|g| canonical_game_path_key(&g.game_exe.to_string_lossy()) == normalized_path);

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
) -> Result<GameConfig, String> {
    add_game_manual_inner(&state, &game_type, &path).await
}

pub async fn add_game_manual_inner(
    service: &ConfigService,
    game_type: &str,
    path: &str,
) -> Result<GameConfig, String> {
    // Parse game type
    let gt: GameType = game_type.parse().map_err(|e: String| e)?;
    let folder = Path::new(path);

    // Validate folder structure (returns warnings, not hard errors, for missing files)
    let (info, warnings) = validator::validate_instance(folder)?;

    let settings = service.get_settings();

    // Duplicate path check (TC-1.5-01, NC-1.3-02)
    let normalized_path = canonical_game_path_key(&info.path);
    for g in &settings.games {
        let existing_normalized = canonical_game_path_key(&g.game_exe.to_string_lossy());
        if existing_normalized == normalized_path {
            return Err(format!(
                "This game path is already registered as '{}'.",
                g.name
            ));
        }
    }

    let id = Uuid::new_v4().to_string();
    let game = GameConfig {
        id,
        name: gt.display_name().to_string(),
        game_type: gt,
        mod_path: PathBuf::from(&info.mods_path),
        game_exe: PathBuf::from(&info.path),
        loader_exe: Some(PathBuf::from(&info.launcher_path)),
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
    games: Vec<GameConfig>,
) -> Result<(), String> {
    save_onboarding_games_inner(&state, games).await
}

pub async fn save_onboarding_games_inner(
    service: &ConfigService,
    games: Vec<GameConfig>,
) -> Result<(), String> {
    let mut settings = service.get_settings();

    let mut added_count = 0;
    for game in games {
        // Double check for duplicates
        let normalized_path = canonical_game_path_key(&game.game_exe.to_string_lossy());

        let is_duplicate = settings
            .games
            .iter()
            .any(|g| canonical_game_path_key(&g.game_exe.to_string_lossy()) == normalized_path);

        if !is_duplicate {
            settings.games.push(game);
            added_count += 1;
        }
    }

    service.save_settings(settings)?;
    log::info!("Onboarding complete: saved {} game(s)", added_count);

    Ok(())
}

/// Remove a game from the database.
/// Used during onboarding if a user wants to discard an auto-detected game.
#[specta::specta]
#[tauri::command]
pub async fn remove_game(
    state: tauri::State<'_, ConfigService>,
    game_id: String,
) -> Result<(), String> {
    crate::services::game::game_service::remove_game_service(&state, &game_id).await
}

/// Get all configured games.
#[specta::specta]
#[tauri::command]
pub async fn get_games(state: tauri::State<'_, ConfigService>) -> Result<Vec<GameConfig>, String> {
    Ok(state.get_settings().games)
}

/// Launch the 3DMigoto Loader (if not running) and then the Game.
/// Covers: US-10.1, TC-10.1-01
#[specta::specta]
#[tauri::command]
pub async fn launch_game(
    state: tauri::State<'_, ConfigService>,
    game_id: String,
) -> Result<(), String> {
    use sysinfo::System;

    // 1. Get Game Config
    let games = get_games(state).await?;
    let game = games
        .into_iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| "Game config not found".to_string())?;

    // 2. Check if Game is valid
    let game_path = &game.game_exe;
    if !game_path.exists() {
        return Err(format!(
            "Game executable not found at: {}",
            game_path.display()
        ));
    }

    // 3. Process Check & Loader Launch (if loader is configured)
    if let Some(launcher_path) = &game.loader_exe {
        if !launcher_path.as_os_str().is_empty() {
            if !launcher_path.exists() {
                return Err(format!("Loader not found at: {}", launcher_path.display()));
            }

            let mut sys = System::new_all();
            sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

            let launcher_name = launcher_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();

            let is_loader_running = sys.processes().values().any(|p| {
                p.name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&launcher_name)
            });

            // 4. Launch Loader if needed
            if !is_loader_running {
                log::info!("Starting Loader: {}", launcher_path.display());

                let launcher_dir = launcher_path.parent().unwrap_or(launcher_path);

                #[cfg(target_os = "windows")]
                {
                    // Use PowerShell to elevate privileges on Windows (US-10.1 requirement)
                    std::process::Command::new("powershell")
                        .arg("-NoProfile")
                        .arg("-Command")
                        .arg(format!(
                            "Start-Process -FilePath '{}' -WorkingDirectory '{}' -Verb RunAs",
                            launcher_path.display(),
                            launcher_dir.display()
                        ))
                        .spawn()
                        .map_err(|e| format!("Failed to start loader as Admin: {e}"))?;
                }

                #[cfg(not(target_os = "windows"))]
                {
                    std::process::Command::new(launcher_path)
                        .current_dir(launcher_dir)
                        .spawn()
                        .map_err(|e| format!("Failed to start loader: {e}"))?;
                }

                // Small delay to let loader initialize
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            } else {
                log::info!("Loader already running: {}", launcher_name);
            }
        }
    }

    // 5. Launch Game
    log::info!("Starting Game: {}", game_path.display());
    let game_dir = game_path.parent().unwrap_or(game_path);

    let mut cmd = std::process::Command::new(game_path);
    cmd.current_dir(game_dir);

    // Apply args
    if let Some(args_str) = game.launch_args {
        if !args_str.trim().is_empty() {
            for arg in args_str.split_whitespace() {
                cmd.arg(arg);
            }
        }
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to start game: {e}"))?;

    Ok(())
}

#[cfg(test)]
#[path = "tests/game_cmds_tests.rs"]
mod tests;
