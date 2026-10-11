//! Small OS-backed detector shared by runtime hotkeys and overlay maintenance.
//!
//! The detector has one source of truth for the active game's executable. This
//! prevents the hotkey path and generation cleanup path from making different
//! decisions about which process is the game.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundProcess {
    pub executable_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameWindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

#[cfg(any(target_os = "windows", test))]
fn positive_extent(start: i32, end: i32) -> Option<u32> {
    u32::try_from(end.checked_sub(start)?)
        .ok()
        .filter(|extent| *extent > 0)
}

pub fn is_game_focused(game_exe: Option<&Path>) -> bool {
    let Some(expected_path) = game_exe else {
        return false;
    };
    foreground_process()
        .is_some_and(|process| same_executable_path(&process.executable_path, expected_path))
}

pub fn focused_game_window(game_exe: Option<&Path>) -> Option<GameWindowGeometry> {
    let expected_path = game_exe?;
    let (process, geometry) = foreground_window()?;
    same_executable_path(&process.executable_path, expected_path).then_some(geometry)
}

pub fn is_game_running(game_exe: Option<&Path>) -> bool {
    let Some(expected_path) = game_exe else {
        return false;
    };

    let mut system = sysinfo::System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    system.processes().values().any(|process| {
        process
            .exe()
            .is_some_and(|path| same_executable_path(path, expected_path))
    })
}

fn same_executable_path(actual: &Path, expected: &Path) -> bool {
    let actual = actual
        .canonicalize()
        .unwrap_or_else(|_| actual.to_path_buf());
    let expected = expected
        .canonicalize()
        .unwrap_or_else(|_| expected.to_path_buf());

    #[cfg(target_os = "windows")]
    {
        actual
            .to_string_lossy()
            .eq_ignore_ascii_case(&expected.to_string_lossy())
    }
    #[cfg(not(target_os = "windows"))]
    {
        actual == expected
    }
}

#[cfg(target_os = "windows")]
fn foreground_process() -> Option<ForegroundProcess> {
    foreground_window().map(|(process, _)| process)
}

#[cfg(target_os = "windows")]
fn foreground_window() -> Option<(ForegroundProcess, GameWindowGeometry)> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Graphics::Gdi::{
        ClientToScreen, GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() || IsIconic(hwnd) != 0 {
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
        let mut client = windows_sys::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetClientRect(hwnd, &mut client) == 0 {
            return None;
        }
        let mut origin = windows_sys::Win32::Foundation::POINT {
            x: client.left,
            y: client.top,
        };
        if ClientToScreen(hwnd, &mut origin) == 0 {
            return None;
        }
        let client_right = origin
            .x
            .saturating_add(client.right.saturating_sub(client.left));
        let client_bottom = origin
            .y
            .saturating_add(client.bottom.saturating_sub(client.top));
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: windows_sys::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            rcWork: windows_sys::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            dwFlags: 0,
        };
        if monitor.is_null() || GetMonitorInfoW(monitor, &mut monitor_info) == 0 {
            return None;
        }
        let left = origin.x.max(monitor_info.rcWork.left);
        let top = origin.y.max(monitor_info.rcWork.top);
        let right = client_right.min(monitor_info.rcWork.right);
        let bottom = client_bottom.min(monitor_info.rcWork.bottom);
        let width = positive_extent(left, right)?;
        let height = positive_extent(top, bottom)?;
        let dpi = GetDpiForWindow(hwnd);
        let scale_factor = if dpi == 0 { 1.0 } else { f64::from(dpi) / 96.0 };
        Some((
            ForegroundProcess {
                executable_path: PathBuf::from(path),
            },
            GameWindowGeometry {
                x: left,
                y: top,
                width,
                height,
                scale_factor,
            },
        ))
    }
}

#[cfg(not(target_os = "windows"))]
fn foreground_process() -> Option<ForegroundProcess> {
    None
}

#[cfg(not(target_os = "windows"))]
fn foreground_window() -> Option<(ForegroundProcess, GameWindowGeometry)> {
    None
}

#[cfg(test)]
mod tests {
    use super::{positive_extent, same_executable_path};
    use std::path::Path;

    #[test]
    fn visible_window_extent_rejects_disjoint_empty_and_overflowing_bounds() {
        assert_eq!(positive_extent(-100, 200), Some(300));
        assert_eq!(positive_extent(0, -400), None);
        assert_eq!(positive_extent(20, 20), None);
        assert_eq!(positive_extent(i32::MIN, i32::MAX), None);
    }

    #[test]
    fn process_matching_rejects_a_same_named_executable_in_another_directory() {
        assert!(!same_executable_path(
            Path::new("C:/Tools/Game.exe"),
            Path::new("C:/Games/Game.exe")
        ));
    }
}
