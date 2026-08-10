use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::domain::errors::AppError;
use crate::services::config::GameConfig;
use crate::services::ini::document::decode_ini_bytes;

// ─── d3dx.ini Reload Key Discovery ──────────────────────────────────────────

/// Key 3DMigoto binds `reload_fixes` to out of the box.
pub const DEFAULT_RELOAD_KEY: &str = "F10";

/// The discovered reload key configuration from d3dx.ini.
#[derive(Debug, Clone)]
pub struct ReloadKeyConfig {
    /// The key that triggers `reload_fixes` (e.g. "F10", "F5").
    pub reload_fixes_key: String,
    /// Whether this was auto-discovered or is the fallback default.
    pub is_fallback: bool,
}

impl Default for ReloadKeyConfig {
    fn default() -> Self {
        Self {
            reload_fixes_key: DEFAULT_RELOAD_KEY.to_string(),
            is_fallback: true,
        }
    }
}

/// Resolve the package config from the configured Mods directory first. XXMI
/// can install the package away from the game executable, so exe proximity is
/// only a compatibility fallback.
pub fn resolve_d3dx_ini_path(game: &GameConfig) -> Option<PathBuf> {
    let package_candidate = game.mod_path.parent().map(|parent| parent.join("d3dx.ini"));
    if package_candidate
        .as_ref()
        .is_some_and(|path| path.is_file())
    {
        return package_candidate;
    }

    let legacy_candidate = game.game_exe.parent().map(|parent| parent.join("d3dx.ini"));
    if legacy_candidate.as_ref().is_some_and(|path| path.is_file()) {
        return legacy_candidate;
    }

    package_candidate.or(legacy_candidate)
}

/// Discover the reload key from a d3dx.ini file.
///
/// Upstream defines this as `reload_fixes = ...` in `[Hunting]`. Missing files
/// or assignments use the loader default; malformed bindings fail explicitly.
pub fn discover_reload_key(d3dx_ini_path: &Path) -> Result<ReloadKeyConfig, AppError> {
    let bytes = match fs::read(d3dx_ini_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(ReloadKeyConfig::default()),
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
        if key_part.trim().eq_ignore_ascii_case("reload_fixes") {
            let value = value_part
                .split([';', '#'])
                .next()
                .unwrap_or_default()
                .trim();
            return Ok(ReloadKeyConfig {
                reload_fixes_key: normalize_reload_binding(value)?,
                is_fallback: false,
            });
        }
    }

    Ok(ReloadKeyConfig::default())
}

pub fn discover_reload_key_for_game(game: &GameConfig) -> Result<ReloadKeyConfig, AppError> {
    let Some(path) = resolve_d3dx_ini_path(game) else {
        return Ok(ReloadKeyConfig::default());
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
