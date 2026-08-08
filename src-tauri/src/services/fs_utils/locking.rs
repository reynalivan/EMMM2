use std::path::Path;

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::RestartManager::{
    RmEndSession, RmGetList, RmRegisterResources, RmStartSession, RM_PROCESS_INFO,
};

/// `RmStartSession` writes a session key of up to `CCH_RM_SESSION_KEY` wide
/// chars **plus a terminating NUL**, so the buffer must hold one more than the
/// key itself. Undersizing it overflows the caller's stack slot.
#[cfg(target_os = "windows")]
const CCH_RM_SESSION_KEY: usize = 32;

#[cfg(target_os = "windows")]
pub fn get_locking_processes(path: &Path) -> Vec<String> {
    let mut session_handle = 0u32;
    let mut session_key = [0u16; CCH_RM_SESSION_KEY + 1];

    if unsafe { RmStartSession(&mut session_handle, 0, session_key.as_mut_ptr()) } != 0 {
        return Vec::new();
    }

    // Single exit point for the session: every early return inside the
    // collector still ends up here.
    let processes = collect_session_processes(session_handle, path);
    unsafe { RmEndSession(session_handle) };
    processes
}

/// Reads the processes holding `path` from an already-started Restart Manager
/// session. The caller owns the session and is responsible for ending it.
#[cfg(target_os = "windows")]
fn collect_session_processes(session_handle: u32, path: &Path) -> Vec<String> {
    let path_wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let paths = [path_wide.as_ptr()];

    let registered = unsafe {
        RmRegisterResources(
            session_handle,
            1,
            paths.as_ptr(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
        )
    };
    if registered != 0 {
        return Vec::new();
    }

    let mut n_proc_info_needed = 0;
    let mut n_proc_info = 0;
    let mut reboot_reasons = 0;

    // First call only sizes the buffer; the out-params carry the count.
    unsafe {
        RmGetList(
            session_handle,
            &mut n_proc_info_needed,
            &mut n_proc_info,
            std::ptr::null_mut(),
            &mut reboot_reasons,
        )
    };
    if n_proc_info_needed == 0 {
        return Vec::new();
    }

    n_proc_info = n_proc_info_needed;
    let mut proc_info: Vec<RM_PROCESS_INFO> = (0..n_proc_info)
        .map(|_| unsafe { std::mem::zeroed() })
        .collect();

    let listed = unsafe {
        RmGetList(
            session_handle,
            &mut n_proc_info_needed,
            &mut n_proc_info,
            proc_info.as_mut_ptr(),
            &mut reboot_reasons,
        )
    };
    if listed != 0 {
        return Vec::new();
    }

    proc_info
        .iter()
        .take(n_proc_info as usize)
        .filter_map(|process| {
            let name = &process.strAppName;
            let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
            String::from_utf16(&name[..end])
                .ok()
                .filter(|value| !value.is_empty())
        })
        .collect()
}

#[cfg(not(target_os = "windows"))]
pub fn get_locking_processes(_path: &Path) -> Vec<String> {
    Vec::new()
}
