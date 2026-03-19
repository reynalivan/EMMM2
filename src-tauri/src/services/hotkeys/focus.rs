use std::path::Path;

use crate::services::config::AppSettings;

pub fn is_active_game_focused(settings: &AppSettings) -> bool {
    let Some(active_game_id) = settings.active_game_id.as_ref() else {
        return false;
    };

    let Some(active_game) = settings
        .games
        .iter()
        .find(|game| &game.id == active_game_id)
    else {
        return false;
    };

    let expected_exe_name = active_game
        .game_exe
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase());

    let Some(expected_exe_name) = expected_exe_name else {
        return false;
    };

    let Some(foreground_exe_name) = foreground_executable_name() else {
        return false;
    };

    foreground_exe_name == expected_exe_name
}

#[cfg(target_os = "windows")]
fn foreground_executable_name() -> Option<String> {
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

        let mut pid: u32 = 0;
        let _thread_id = GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
        if pid == 0 {
            return None;
        }

        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }

        let mut buffer = vec![0u16; 1024];
        let mut len = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut len as *mut u32);
        CloseHandle(process);

        if ok == 0 || len == 0 {
            return None;
        }

        buffer.truncate(len as usize);
        let full_path = String::from_utf16(&buffer).ok()?;
        let exe_name = Path::new(&full_path)
            .file_name()
            .and_then(|name| name.to_str())?
            .to_ascii_lowercase();

        Some(exe_name)
    }
}

#[cfg(not(target_os = "windows"))]
fn foreground_executable_name() -> Option<String> {
    None
}
