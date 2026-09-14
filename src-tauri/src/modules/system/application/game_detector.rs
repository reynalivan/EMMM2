//! Small OS-backed detector shared by runtime hotkeys and overlay maintenance.
//!
//! The detector has one source of truth for the active game's executable. This
//! prevents the hotkey path and generation cleanup path from making different
//! decisions about which process is the game.

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundProcess {
    pub executable_name: String,
}

pub fn is_game_focused(game_exe: Option<&Path>) -> bool {
    let Some(expected_name) = executable_name(game_exe) else {
        return false;
    };
    foreground_process()
        .is_some_and(|process| same_executable_name(&process.executable_name, &expected_name))
}

pub fn is_game_running(game_exe: Option<&Path>) -> bool {
    let Some(expected_name) = executable_name(game_exe) else {
        return false;
    };

    let mut system = sysinfo::System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    system
        .processes()
        .values()
        .any(|process| same_executable_name(&process.name().to_string_lossy(), &expected_name))
}

fn executable_name(path: Option<&Path>) -> Option<String> {
    path.and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
}

fn same_executable_name(actual: &str, expected: &str) -> bool {
    actual.eq_ignore_ascii_case(expected)
}

#[cfg(target_os = "windows")]
fn foreground_process() -> Option<ForegroundProcess> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }

        let mut pid = 0_u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }

        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }

        let mut buffer = vec![0_u16; 1024];
        let mut length = buffer.len() as u32;
        let read = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length);
        CloseHandle(process);
        if read == 0 || length == 0 {
            return None;
        }

        buffer.truncate(length as usize);
        let path = String::from_utf16(&buffer).ok()?;
        let executable_name = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())?
            .to_ascii_lowercase();
        Some(ForegroundProcess { executable_name })
    }
}

#[cfg(not(target_os = "windows"))]
fn foreground_process() -> Option<ForegroundProcess> {
    None
}

#[cfg(test)]
mod tests {
    use super::{executable_name, same_executable_name};
    use std::path::Path;

    #[test]
    fn executable_name_is_case_insensitive_and_uses_the_file_name() {
        assert_eq!(
            executable_name(Some(Path::new("C:/Games/GIMI/Game.EXE"))).as_deref(),
            Some("game.exe")
        );
    }

    #[test]
    fn process_name_matching_ignores_case_but_not_different_executables() {
        assert!(same_executable_name("Game.EXE", "game.exe"));
        assert!(!same_executable_name("Launcher.exe", "game.exe"));
    }
}
