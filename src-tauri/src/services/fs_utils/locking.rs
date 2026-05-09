use std::path::Path;

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::RestartManager::{
    RmEndSession, RmGetList, RmRegisterResources, RmStartSession, RM_PROCESS_INFO,
};

#[cfg(target_os = "windows")]
pub fn get_locking_processes(path: &Path) -> Vec<String> {
    let mut session_handle = 0;
    let mut session_key = [0u16; 16]; // CCH_RM_SESSION_KEY is 16 on most systems

    unsafe {
        if RmStartSession(&mut session_handle, 0, session_key.as_mut_ptr()) != 0 {
            return Vec::new();
        }

        let path_wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let paths = [path_wide.as_ptr()];

        if RmRegisterResources(
            session_handle,
            1,
            paths.as_ptr(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
        ) != 0
        {
            RmEndSession(session_handle);
            return Vec::new();
        }

        let mut n_proc_info_needed = 0;
        let mut n_proc_info = 0;
        let mut reboot_reasons = 0;

        // First call to get required size
        RmGetList(
            session_handle,
            &mut n_proc_info_needed,
            &mut n_proc_info,
            std::ptr::null_mut(),
            &mut reboot_reasons,
        );

        if n_proc_info_needed == 0 {
            RmEndSession(session_handle);
            return Vec::new();
        }

        n_proc_info = n_proc_info_needed;
        let mut proc_info: Vec<RM_PROCESS_INFO> =
            (0..n_proc_info).map(|_| std::mem::zeroed()).collect();

        if RmGetList(
            session_handle,
            &mut n_proc_info_needed,
            &mut n_proc_info,
            proc_info.as_mut_ptr(),
            &mut reboot_reasons,
        ) == 0
        {
            let mut processes = Vec::new();
            for process in proc_info.iter().take(n_proc_info as usize) {
                let name_wide = &process.strAppName;
                // strAppName is [u16; 256]
                let end = name_wide.iter().position(|&c| c == 0).unwrap_or(256);
                if let Ok(name) = String::from_utf16(&name_wide[..end]) {
                    if !name.is_empty() {
                        processes.push(name);
                    }
                }
            }
            RmEndSession(session_handle);
            return processes;
        }

        RmEndSession(session_handle);
    }
    Vec::new()
}

#[cfg(not(target_os = "windows"))]
pub fn get_locking_processes(_path: &Path) -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
