use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};

use crate::domain::errors::AppError;
use crate::services::config::AppSettings;
use crate::services::hotkeys::manager::normalize_shortcut;
use crate::services::keyviewer::generator::{self, DEFAULT_RELOAD_KEY};

pub fn trigger_reload_fixes(settings: &AppSettings) -> Result<String, AppError> {
    let Some(active_game) = settings.active_game() else {
        return Err(AppError::Internal("No active game configured".to_string()));
    };

    let discovered_key = active_game
        .game_exe
        .parent()
        .map(|game_root| game_root.join("d3dx.ini"))
        .map(|d3dx_path| generator::discover_reload_key(&d3dx_path).reload_fixes_key)
        .unwrap_or_else(|| DEFAULT_RELOAD_KEY.to_string());

    send_reload_key(&discovered_key)?;
    Ok(discovered_key)
}

fn send_reload_key(key_str: &str) -> Result<(), AppError> {
    // Same normalization the OS registration uses, so a key we registered is a
    // key we can replay.
    let normalized = normalize_shortcut(key_str);
    let tokens: Vec<&str> = normalized
        .split('+')
        .filter(|token| !token.is_empty())
        .collect();
    if tokens.is_empty() {
        return Err(AppError::Internal(format!(
            "Invalid reload key '{key_str}'"
        )));
    }

    let mut modifiers = Vec::new();
    for token in &tokens[..tokens.len() - 1] {
        modifiers.push(parse_modifier(token)?);
    }

    let main_key = parse_main_key(tokens[tokens.len() - 1])?;

    let mut enigo = Enigo::new(&Settings::default())?;

    for modifier in &modifiers {
        enigo.key(*modifier, Press)?;
    }

    enigo.key(main_key, Click)?;

    for modifier in modifiers.iter().rev() {
        enigo.key(*modifier, Release)?;
    }

    Ok(())
}

fn parse_modifier(token: &str) -> Result<Key, AppError> {
    match token {
        "ctrl" | "control" => Ok(Key::Control),
        "shift" => Ok(Key::Shift),
        "alt" => Ok(Key::Alt),
        "meta" | "win" | "super" => Ok(Key::Meta),
        _ => Err(AppError::Internal(format!(
            "Unsupported reload modifier '{token}'"
        ))),
    }
}

/// `FUNCTION_KEYS[n - 1]` is `Fn`.
const FUNCTION_KEYS: [Key; 24] = [
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
    Key::F11,
    Key::F12,
    Key::F13,
    Key::F14,
    Key::F15,
    Key::F16,
    Key::F17,
    Key::F18,
    Key::F19,
    Key::F20,
    Key::F21,
    Key::F22,
    Key::F23,
    Key::F24,
];

/// Non-alphanumeric keys 3DMigoto accepts in a `key =` assignment, in the
/// spellings d3dx.ini uses (with or without the `vk_` prefix).
const NAMED_KEYS: &[(&str, Key)] = &[
    ("space", Key::Space),
    ("tab", Key::Tab),
    ("return", Key::Return),
    ("enter", Key::Return),
    ("escape", Key::Escape),
    ("esc", Key::Escape),
    ("back", Key::Backspace),
    ("backspace", Key::Backspace),
    ("delete", Key::Delete),
    ("insert", Key::Insert),
    ("home", Key::Home),
    ("end", Key::End),
    ("prior", Key::PageUp),
    ("pageup", Key::PageUp),
    ("next", Key::PageDown),
    ("pagedown", Key::PageDown),
    ("up", Key::UpArrow),
    ("down", Key::DownArrow),
    ("left", Key::LeftArrow),
    ("right", Key::RightArrow),
];

/// Resolve the non-modifier key of a reload binding.
///
/// d3dx.ini may bind `reload_fixes` to any key, not just a function key — a
/// letter, a digit, `VK_F5`, or a named key are all valid there. Rejecting
/// those would fail every preset cycle at press time, so all of them resolve.
fn parse_main_key(token: &str) -> Result<Key, AppError> {
    // d3dx.ini commonly spells keys as VK_ constants; the suffix is the name.
    let token = token.strip_prefix("vk_").unwrap_or(token);

    if let Some(key) = token
        .strip_prefix('f')
        .and_then(|digits| digits.parse::<usize>().ok())
        .and_then(|number| FUNCTION_KEYS.get(number.wrapping_sub(1)))
    {
        return Ok(*key);
    }

    if let Some((_, key)) = NAMED_KEYS.iter().find(|(name, _)| *name == token) {
        return Ok(*key);
    }

    // Single letter or digit — `normalize_shortcut` already lowercased it.
    let mut chars = token.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphanumeric() => Ok(Key::Unicode(c)),
        _ => Err(AppError::Internal(format!(
            "Unsupported reload key '{token}'"
        ))),
    }
}
