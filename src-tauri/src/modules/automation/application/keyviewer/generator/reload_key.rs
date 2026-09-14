use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::modules::library::application::ini::document::decode_ini_bytes;
use crate::modules::settings::application::config::GameConfig;
use crate::shared::errors::AppError;

/// Keyboard binding for the 3DMigoto configuration reload operation.
#[derive(Debug, Clone)]
pub struct ReloadKeyConfig {
    pub reload_config_key: String,
}

/// Resolve the package config from the configured Mods directory. XXMI
/// installs the package away from the game executable.
pub fn resolve_d3dx_ini_path(game: &GameConfig) -> Option<PathBuf> {
    let instance_config = game.instance_path.join("d3dx.ini");
    if instance_config.is_file() {
        return Some(instance_config);
    }
    game.mod_path
        .parent()
        .map(|parent| parent.join("d3dx.ini"))
        .filter(|path| path.is_file())
}

/// Discover `reload_config` from the `[Hunting]` section. KeyViewer changes
/// INI resources, so shader-fix reload is neither necessary nor sufficient.
pub fn discover_reload_key(d3dx_ini_path: &Path) -> Result<ReloadKeyConfig, AppError> {
    let bytes = match fs::read(d3dx_ini_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(AppError::Validation(format!(
                "NeedsManualReload: 3DMigoto config was not found at {}",
                d3dx_ini_path.display()
            )));
        }
        Err(error) => return Err(error.into()),
    };
    let (content, _had_bom, clean) = decode_ini_bytes(&bytes);
    if !clean {
        return Err(AppError::Validation(format!(
            "Cannot safely decode reload config: {}",
            d3dx_ini_path.display()
        )));
    }

    let mut in_hunting_section = false;
    let mut reload_config_key = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_hunting_section = trimmed
                .find(']')
                .map(|end| trimmed[1..end].trim().eq_ignore_ascii_case("hunting"))
                .unwrap_or(false);
            continue;
        }
        if !in_hunting_section {
            continue;
        }
        let Some((key_part, value_part)) = trimmed.split_once('=') else {
            continue;
        };
        let value = value_part
            .split([';', '#'])
            .next()
            .unwrap_or_default()
            .trim();
        if key_part.trim().eq_ignore_ascii_case("reload_config") {
            reload_config_key = Some(normalize_reload_binding(value)?);
        }
    }

    Ok(ReloadKeyConfig {
        reload_config_key: reload_config_key.ok_or_else(|| {
            AppError::Validation(format!(
                "NeedsManualReload: [Hunting] reload_config is not bound in {}",
                d3dx_ini_path.display()
            ))
        })?,
    })
}

pub fn discover_reload_key_for_game(game: &GameConfig) -> Result<ReloadKeyConfig, AppError> {
    let Some(path) = resolve_d3dx_ini_path(game) else {
        return Err(AppError::Validation(
            "NeedsManualReload: could not locate the active 3DMigoto d3dx.ini".to_string(),
        ));
    };
    discover_reload_key(&path)
}

fn normalize_reload_binding(value: &str) -> Result<String, AppError> {
    let mut modifiers: Vec<&str> = Vec::new();
    let mut main_key: Option<String> = None;

    for raw_token in value
        .split(|character: char| character.is_whitespace() || character == '+')
        .filter(|token| !token.is_empty())
    {
        let token = raw_token.to_ascii_lowercase();
        if token.starts_with("no_") {
            continue;
        }

        let modifier = match token.as_str() {
            "ctrl" | "control" => Some("Ctrl"),
            "shift" => Some("Shift"),
            "alt" | "menu" => Some("Alt"),
            "meta" | "win" | "windows" | "super" => Some("Meta"),
            _ => None,
        };
        if let Some(modifier) = modifier {
            if !modifiers.contains(&modifier) {
                modifiers.push(modifier);
            }
            continue;
        }

        let key = token.strip_prefix("vk_").unwrap_or(&token);
        if key.starts_with("xb_") || key.starts_with("gamepad_") {
            return Err(AppError::Validation(format!(
                "Controller-only reload binding is not replayable: {value}"
            )));
        }
        if main_key.replace(key.to_ascii_uppercase()).is_some() {
            return Err(AppError::Validation(format!(
                "Reload binding must contain exactly one keyboard key: {value}"
            )));
        }
    }

    let main_key = main_key.ok_or_else(|| {
        AppError::Validation(format!(
            "Reload binding does not contain a replayable keyboard key: {value}"
        ))
    })?;
    let mut canonical: Vec<String> = modifiers.into_iter().map(str::to_string).collect();
    canonical.push(main_key);
    Ok(canonical.join("+"))
}
