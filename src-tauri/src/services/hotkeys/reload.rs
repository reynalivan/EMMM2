use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};

use crate::services::config::AppSettings;
use crate::services::keyviewer::generator;

pub fn trigger_reload_fixes(settings: &AppSettings) -> Result<String, String> {
    let Some(active_game_id) = settings.active_game_id.as_ref() else {
        return Err("No active game selected".to_string());
    };

    let Some(active_game) = settings
        .games
        .iter()
        .find(|game| &game.id == active_game_id)
    else {
        return Err("Active game config not found".to_string());
    };

    let discovered_key = active_game
        .game_exe
        .parent()
        .map(|game_root| game_root.join("d3dx.ini"))
        .map(|d3dx_path| generator::discover_reload_key(&d3dx_path).reload_fixes_key)
        .unwrap_or_else(|| "F10".to_string());

    send_reload_key(&discovered_key)?;
    Ok(discovered_key)
}

fn send_reload_key(key_str: &str) -> Result<(), String> {
    let normalized = key_str.trim().replace(' ', "").to_ascii_lowercase();
    if normalized.is_empty() {
        return Err("Reload key is empty".to_string());
    }

    let tokens: Vec<&str> = normalized
        .split('+')
        .filter(|token| !token.is_empty())
        .collect();
    if tokens.is_empty() {
        return Err(format!("Invalid reload key '{key_str}'"));
    }

    let mut modifiers = Vec::new();
    for token in &tokens[..tokens.len() - 1] {
        modifiers.push(parse_modifier(token)?);
    }

    let main_key = parse_main_key(tokens[tokens.len() - 1])?;

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to initialize Enigo input sender: {e}"))?;

    for modifier in &modifiers {
        enigo
            .key(*modifier, Press)
            .map_err(|e| format!("Failed to press modifier key: {e}"))?;
    }

    enigo
        .key(main_key, Click)
        .map_err(|e| format!("Failed to send reload key '{key_str}': {e}"))?;

    for modifier in modifiers.iter().rev() {
        enigo
            .key(*modifier, Release)
            .map_err(|e| format!("Failed to release modifier key: {e}"))?;
    }

    Ok(())
}

fn parse_modifier(token: &str) -> Result<Key, String> {
    match token {
        "ctrl" | "control" => Ok(Key::Control),
        "shift" => Ok(Key::Shift),
        "alt" => Ok(Key::Alt),
        "meta" | "win" | "super" => Ok(Key::Meta),
        _ => Err(format!("Unsupported reload modifier '{token}'")),
    }
}

fn parse_main_key(token: &str) -> Result<Key, String> {
    match token {
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),
        "f13" => Ok(Key::F13),
        "f14" => Ok(Key::F14),
        "f15" => Ok(Key::F15),
        "f16" => Ok(Key::F16),
        "f17" => Ok(Key::F17),
        "f18" => Ok(Key::F18),
        "f19" => Ok(Key::F19),
        "f20" => Ok(Key::F20),
        "f21" => Ok(Key::F21),
        "f22" => Ok(Key::F22),
        "f23" => Ok(Key::F23),
        "f24" => Ok(Key::F24),
        _ => Err(format!("Unsupported reload key '{token}'")),
    }
}
