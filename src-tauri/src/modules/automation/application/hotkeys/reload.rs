use enigo::Key;
#[cfg(not(target_os = "windows"))]
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Keyboard, Settings,
};
#[cfg(any(target_os = "windows", test))]
use std::time::Duration;

use crate::modules::automation::application::hotkeys::focus;
use crate::modules::automation::application::hotkeys::manager::normalize_shortcut;
use crate::modules::automation::application::keyviewer::generator;
use crate::modules::settings::application::config::AppSettings;
use crate::shared::errors::AppError;

/// Reload INI configuration after publishing generated KeyViewer artifacts.
/// This intentionally does not claim that 3DMigoto has acknowledged the
/// reload; it only reports that the configured key was sent.
pub fn trigger_reload_config(settings: &AppSettings) -> Result<String, AppError> {
    trigger_reload_config_with_sender(settings, |binding| {
        trigger_binding_while_focused(settings, binding)
    })
}

fn trigger_reload_config_with_sender(
    settings: &AppSettings,
    send: impl FnOnce(&str) -> Result<(), AppError>,
) -> Result<String, AppError> {
    let Some(active_game) = settings.active_game() else {
        return Err(AppError::Internal("No active game configured".to_string()));
    };
    let binding = generator::discover_reload_key_for_game(active_game)?.reload_config_key;
    send(&binding)?;
    Ok(binding)
}

pub fn configured_reload_config_binding(settings: &AppSettings) -> Result<String, AppError> {
    let active_game = settings
        .active_game()
        .ok_or_else(|| AppError::Internal("No active game configured".to_string()))?;
    Ok(generator::discover_reload_key_for_game(active_game)?.reload_config_key)
}

/// Replays an already validated hotkey to the active game. The focus check is
/// intentionally repeated immediately before input is synthesized.
pub fn trigger_binding_while_focused(
    settings: &AppSettings,
    binding: &str,
) -> Result<(), AppError> {
    let (modifiers, main_key) = parse_chord(binding)?;
    send_chord(&modifiers, main_key, || {
        focus::is_active_game_focused(settings)
    })
}

fn parse_chord(key_str: &str) -> Result<(Vec<Key>, Key), AppError> {
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
        let modifier = parse_modifier(token)?;
        if !modifiers.contains(&modifier) {
            modifiers.push(modifier);
        }
    }

    Ok((modifiers, parse_main_key(tokens[tokens.len() - 1])?))
}

fn require_focus(focused: bool) -> Result<(), AppError> {
    if focused {
        Ok(())
    } else {
        Err(AppError::Validation(
            "NeedsManualReload: active game is not focused".to_string(),
        ))
    }
}

/// The Windows plugin's Released event covers the main key only. Its original
/// modifier key-up events must finish before synthetic modifier key-downs.
pub(super) async fn wait_for_input_release(binding: &str) -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        let (_, main_key) = parse_chord(binding)?;
        let main_key = windows_input::virtual_key(main_key)?;
        wait_until_released(
            || windows_input::keys_released(main_key),
            Duration::from_secs(1),
        )
        .await
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = binding;
        Ok(())
    }
}

#[cfg(any(target_os = "windows", test))]
async fn wait_until_released(
    mut released: impl FnMut() -> bool,
    timeout: Duration,
) -> Result<(), AppError> {
    tokio::time::timeout(timeout, async {
        while !released() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| AppError::Validation("NeedsManualReload: keyboard keys are still held".into()))
}

#[cfg(target_os = "windows")]
fn send_chord(
    modifiers: &[Key],
    main_key: Key,
    focused: impl FnOnce() -> bool,
) -> Result<(), AppError> {
    windows_input::send_chord(modifiers, main_key, focused)
}

#[cfg(not(target_os = "windows"))]
fn send_chord(
    modifiers: &[Key],
    main_key: Key,
    focused: impl FnOnce() -> bool,
) -> Result<(), AppError> {
    require_focus(focused())?;

    let mut enigo = Enigo::new(&Settings::default())?;

    let mut pressed = 0usize;
    let send_result = (|| -> Result<(), AppError> {
        for modifier in modifiers {
            enigo.key(*modifier, Press)?;
            pressed += 1;
        }
        enigo.key(main_key, Click)?;
        Ok(())
    })();

    let mut release_error = None;
    for modifier in modifiers[..pressed].iter().rev() {
        if let Err(error) = enigo.key(*modifier, Release) {
            release_error.get_or_insert(error);
        }
    }

    match (send_result, release_error) {
        (Ok(()), None) => Ok(()),
        (Err(error), None) => Err(error),
        (Ok(()), Some(error)) => Err(error.into()),
        (Err(error), Some(release_error)) => Err(AppError::Internal(format!(
            "Reload input failed ({error}); modifier release also failed ({release_error})"
        ))),
    }
}

#[cfg(target_os = "windows")]
mod windows_input {
    use super::{require_focus, AppError, Key};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_DELETE, VK_DOWN,
        VK_END, VK_HOME, VK_INSERT, VK_LEFT, VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RIGHT,
        VK_RWIN, VK_SHIFT, VK_UP,
    };

    const REPLAY_MARKER: usize = 100;

    pub(super) fn virtual_key(key: Key) -> Result<VIRTUAL_KEY, AppError> {
        virtual_key_with(key, VIRTUAL_KEY::try_from)
    }

    fn virtual_key_with(
        key: Key,
        convert: impl FnOnce(Key) -> Result<VIRTUAL_KEY, &'static str>,
    ) -> Result<VIRTUAL_KEY, AppError> {
        match key {
            // Registration uses VK_A..VK_Z/VK_0..VK_9, not locale-dependent
            // character conversion (which can include Shift/AltGr bits).
            Key::Unicode(character) if character.is_ascii_alphanumeric() => {
                Ok(VIRTUAL_KEY(character.to_ascii_uppercase() as u16))
            }
            _ => convert(key).map_err(|error| AppError::Internal(error.into())),
        }
    }

    pub(super) fn keys_released(main: VIRTUAL_KEY) -> bool {
        keys_released_with(main, |key| unsafe { GetAsyncKeyState(i32::from(key.0)) })
    }

    fn keys_released_with(main: VIRTUAL_KEY, mut state: impl FnMut(VIRTUAL_KEY) -> i16) -> bool {
        [main, VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
            .into_iter()
            .all(|key| state(key) >= 0)
    }

    fn event(key: VIRTUAL_KEY, up: bool) -> INPUT {
        let mut flags = KEYBD_EVENT_FLAGS::default();
        if matches!(
            key,
            VK_DELETE
                | VK_DOWN
                | VK_END
                | VK_HOME
                | VK_INSERT
                | VK_LEFT
                | VK_NEXT
                | VK_PRIOR
                | VK_RIGHT
                | VK_UP
        ) {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        if up {
            flags |= KEYEVENTF_KEYUP;
        }
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key,
                    dwFlags: flags,
                    dwExtraInfo: REPLAY_MARKER,
                    ..Default::default()
                },
            },
        }
    }

    fn chord_events(modifiers: &[Key], main: Key) -> Result<(Vec<INPUT>, Vec<INPUT>), AppError> {
        let mut keys = modifiers
            .iter()
            .copied()
            .map(virtual_key)
            .collect::<Result<Vec<_>, _>>()?;
        keys.push(virtual_key(main)?);
        let releases: Vec<_> = keys.iter().rev().map(|key| event(*key, true)).collect();
        let mut input: Vec<_> = keys.iter().map(|key| event(*key, false)).collect();
        input.extend_from_slice(&releases);
        Ok((input, releases))
    }

    pub(super) fn send_chord(
        modifiers: &[Key],
        main: Key,
        focused: impl FnOnce() -> bool,
    ) -> Result<(), AppError> {
        let (input, releases) = chord_events(modifiers, main)?;
        if !keys_released(virtual_key(main)?) {
            return Err(AppError::Validation(
                "NeedsManualReload: keyboard keys are still held".into(),
            ));
        }
        require_focus(focused())?;
        // One SendInput call keeps original key-up events out of this chord.
        send_batch(&input, &releases, |events| unsafe {
            SendInput(events, std::mem::size_of::<INPUT>() as i32)
        })
    }

    fn send_batch(
        input: &[INPUT],
        releases: &[INPUT],
        mut send: impl FnMut(&[INPUT]) -> u32,
    ) -> Result<(), AppError> {
        let sent = send(input) as usize;
        if sent == input.len() {
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        // This batch contains all key-downs followed by reverse key-ups.
        // Only the still-held prefix of the inserted downs belongs to us.
        let pressed = sent.min(releases.len());
        let already_released = sent.saturating_sub(releases.len());
        let pending = pressed.saturating_sub(already_released);
        let cleanup = &releases[releases.len() - pending..];
        let released = if cleanup.is_empty() {
            0
        } else {
            send(cleanup) as usize
        };
        Err(AppError::Internal(format!(
            "Keyboard replay inserted {sent}/{} events ({error}); cleanup inserted {released}/{} releases",
            input.len(), cleanup.len(),
        )))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn modifier_aliases_produce_one_balanced_modifier_pair() {
            let (modifiers, main) = super::super::parse_chord("Ctrl+Control+F7").unwrap();
            assert_eq!(modifiers, vec![Key::Control]);
            let (input, releases) = chord_events(&modifiers, main).unwrap();
            assert_eq!(input.len(), 4);
            assert_eq!(releases.len(), 2);
            assert_eq!(unsafe { input[0].Anonymous.ki.wVk }, VK_CONTROL);
            assert_eq!(unsafe { input[3].Anonymous.ki.wVk }, VK_CONTROL);
        }

        #[test]
        fn main_key_release_does_not_imply_modifier_release() {
            let main = virtual_key(Key::F16).unwrap();
            for held in [main, VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN] {
                assert!(!keys_released_with(main, |key| if key == held {
                    i16::MIN
                } else {
                    0
                }));
            }
            // The low "pressed since last query" bit is not a held-key signal.
            assert!(keys_released_with(main, |_| 1));
        }

        #[test]
        fn replay_is_one_balanced_batch_with_reverse_releases() {
            let (input, releases) =
                chord_events(&[Key::Control, Key::Alt, Key::Shift], Key::F16).unwrap();
            let actual: Vec<_> = input
                .iter()
                .map(|event| unsafe {
                    let key = event.Anonymous.ki;
                    (key.wVk.0, key.dwFlags.contains(KEYEVENTF_KEYUP))
                })
                .collect();
            assert_eq!(
                actual,
                vec![
                    (0x11, false),
                    (0x12, false),
                    (0x10, false),
                    (0x7f, false),
                    (0x7f, true),
                    (0x10, true),
                    (0x12, true),
                    (0x11, true)
                ]
            );
            let mut calls = 0;
            send_batch(&input, &releases, |batch| {
                calls += 1;
                batch.len() as u32
            })
            .unwrap();
            assert_eq!(calls, 1);
        }

        #[test]
        fn partial_replay_cleans_up_but_blocked_replay_does_not_release_user_keys() {
            let (input, releases) = chord_events(&[Key::Control], Key::F7).unwrap();
            for inserted in [0, 1, 3] {
                let mut calls = 0;
                let result = send_batch(&input, &releases, |batch| {
                    calls += 1;
                    if calls == 1 {
                        inserted
                    } else {
                        batch.len() as u32
                    }
                });
                assert!(result.is_err());
                assert_eq!(calls, if inserted == 0 { 1 } else { 2 });
            }
        }

        #[test]
        fn partial_replay_releases_only_synthetic_keys_still_down() {
            let (input, releases) =
                chord_events(&[Key::Control, Key::Alt, Key::Shift], Key::F16).unwrap();
            for inserted in 0..input.len() {
                let mut calls = 0;
                let mut cleanup = Vec::new();
                assert!(send_batch(&input, &releases, |batch| {
                    calls += 1;
                    if calls == 1 {
                        inserted as u32
                    } else {
                        cleanup = batch
                            .iter()
                            .map(|event| unsafe { event.Anonymous.ki.wVk.0 })
                            .collect();
                        batch.len() as u32
                    }
                })
                .is_err());
                let down_count = inserted.min(4) - inserted.saturating_sub(4);
                let expected: Vec<_> = [VK_CONTROL.0, VK_MENU.0, VK_SHIFT.0, 0x7f][..down_count]
                    .iter()
                    .rev()
                    .copied()
                    .collect();
                assert_eq!(cleanup, expected, "inserted={inserted}");
            }
        }

        #[test]
        fn ascii_replay_uses_registered_virtual_keys_not_character_layout_modifiers() {
            for character in 'a'..='z' {
                assert_eq!(
                    virtual_key(Key::Unicode(character)).unwrap().0,
                    character.to_ascii_uppercase() as u16
                );
            }
            for character in '0'..='9' {
                assert_eq!(
                    virtual_key(Key::Unicode(character)).unwrap().0,
                    character as u16
                );
            }
            let (_, digit) = super::super::parse_chord("Ctrl+1").unwrap();
            let mut used_character_layout = false;
            let registered = virtual_key_with(digit, |_| {
                used_character_layout = true;
                Ok(VIRTUAL_KEY(0x0131))
            })
            .unwrap();
            assert_eq!(registered.0, 0x31);
            assert!(!used_character_layout);
        }
    }
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
/// d3dx.ini may bind `reload_config` to any key, not just a function key — a
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

#[cfg(test)]
mod tests {
    use super::trigger_reload_config_with_sender;
    use crate::modules::games::domain::models::{GameType, LaunchMode};
    use crate::modules::settings::application::config::{AppSettings, GameConfig};

    #[tokio::test]
    async fn waits_until_the_held_modifier_is_released() {
        let mut observations = 0;
        super::wait_until_released(
            || {
                observations += 1;
                observations >= 3
            },
            std::time::Duration::from_secs(1),
        )
        .await
        .unwrap();
        assert_eq!(observations, 3);
    }

    #[tokio::test]
    async fn a_held_modifier_times_out_without_replay() {
        let error = super::wait_until_released(|| false, std::time::Duration::from_millis(25))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("keys are still held"));
    }

    fn settings_with_reload_binding(temp_dir: &tempfile::TempDir) -> AppSettings {
        let package_root = temp_dir.path().join("package");
        let mods_path = package_root.join("Mods");
        std::fs::create_dir_all(&mods_path).unwrap();
        std::fs::write(
            package_root.join("d3dx.ini"),
            "[Hunting]\nreload_config = Ctrl+F10\n",
        )
        .unwrap();

        AppSettings {
            games: vec![GameConfig {
                id: "game".to_string(),
                name: "Game".to_string(),
                game_type: GameType::Gimi,
                instance_path: package_root,
                mod_path: mods_path,
                ready_to_move_path: None,
                launch_mode: LaunchMode::Standalone,
                game_exe: None,
                loader_exe: None,
                xxmi_launcher_exe: None,
                launch_args: None,
                warnings: Vec::new(),
            }],
            active_game_id: Some("game".to_string()),
            ..AppSettings::default()
        }
    }

    #[test]
    fn reload_resolves_the_configured_binding_for_the_injected_sender() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let settings = settings_with_reload_binding(&temp_dir);
        let mut sent_binding = None;

        let binding = trigger_reload_config_with_sender(&settings, |binding| {
            sent_binding = Some(binding.to_string());
            Ok(())
        })
        .unwrap();

        assert_eq!(binding, "Ctrl+F10");
        assert_eq!(sent_binding.as_deref(), Some("Ctrl+F10"));
    }

    #[test]
    fn automatic_reload_rejects_an_unfocused_game_before_input_replay() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let settings = settings_with_reload_binding(&temp_dir);
        let error = super::trigger_reload_config(&settings).unwrap_err();
        assert!(error.to_string().contains("active game is not focused"));
    }
}
