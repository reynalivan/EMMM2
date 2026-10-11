use super::{LockDiscovery, LockProcess, LockResources};
use std::{collections::BTreeMap, ffi::c_void, os::windows::ffi::OsStrExt, path::PathBuf};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_MORE_DATA, FILETIME, HANDLE, HWND, LPARAM, WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    Security::{GetLengthSid, GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER},
    System::{
        RemoteDesktop::ProcessIdToSessionId,
        RestartManager::{
            RmCritical, RmEndSession, RmGetList, RmRegisterResources, RmService, RmStartSession,
            RM_PROCESS_INFO,
        },
        SystemInformation::GetWindowsDirectoryW,
        Threading::{
            GetCurrentProcess, GetCurrentProcessId, GetProcessInformation, GetProcessTimes,
            IsProcessCritical, OpenProcess, OpenProcessToken, ProcessProtectionLevelInfo,
            QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
            PROCESS_PROTECTION_LEVEL_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION,
            PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, PROTECTION_LEVEL_NONE,
        },
    },
    UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CLOSE,
    },
};

const REGISTRATION_BATCH: usize = 128;
const MAX_PROCESSES: u32 = 4_096;
const EXIT_WAIT_MS: u32 = 3_000;

struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
struct Session(u32);
impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            RmEndSession(self.0);
        }
    }
}

fn api_error(operation: &str, code: u32) -> String {
    format!("{operation} failed (Windows error {code})")
}
fn last_error(operation: &str) -> String {
    format!("{operation}: {}", std::io::Error::last_os_error())
}
fn text_wide(value: &[u16]) -> String {
    String::from_utf16_lossy(&value[..value.iter().position(|&ch| ch == 0).unwrap_or(value.len())])
}
fn filetime(value: FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}

pub(super) fn discover(resources: LockResources) -> LockDiscovery {
    if resources.files.is_empty() {
        return LockDiscovery { status: if resources.partial { "partial" } else { "unknown" }, message: Some("No regular files could be registered. Directory-only or system handles may remain unknown.".into()), processes: Vec::new() };
    }
    match discover_inner(&resources) {
        Ok(processes) => LockDiscovery {
            status: if resources.partial { "partial" } else if processes.is_empty() { "unknown" } else { "complete" },
            message: resources.partial.then(|| "Discovery reached its file/time limit or skipped inaccessible/reparse entries; additional file users may remain unknown.".into()),
            processes,
        },
        Err(message) => LockDiscovery { status: "failed", message: Some(message), processes: Vec::new() },
    }
}

fn discover_inner(resources: &LockResources) -> Result<Vec<LockProcess>, String> {
    let mut handle = 0;
    let mut key = [0_u16; 33];
    let status = unsafe { RmStartSession(&mut handle, 0, key.as_mut_ptr()) };
    if status != 0 {
        return Err(api_error("Restart Manager startup", status));
    }
    let session = Session(handle);
    for files in resources.files.chunks(REGISTRATION_BATCH) {
        let encoded: Vec<Vec<u16>> = files
            .iter()
            .map(|path| path.as_os_str().encode_wide().chain(Some(0)).collect())
            .collect();
        let paths: Vec<*const u16> = encoded.iter().map(|path| path.as_ptr()).collect();
        let status = unsafe {
            RmRegisterResources(
                session.0,
                paths.len() as u32,
                paths.as_ptr(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
            )
        };
        if status != 0 {
            return Err(api_error("Restart Manager resource registration", status));
        }
    }
    let mut needed = 0;
    let mut count = 0;
    let mut reboot = 0;
    let first = unsafe {
        RmGetList(
            session.0,
            &mut needed,
            &mut count,
            std::ptr::null_mut(),
            &mut reboot,
        )
    };
    if first != 0 && first != ERROR_MORE_DATA {
        return Err(api_error("Restart Manager discovery", first));
    }
    if needed == 0 {
        return Ok(Vec::new());
    }
    for _ in 0..3 {
        if needed > MAX_PROCESSES {
            return Err("Restart Manager returned too many processes".into());
        }
        count = needed;
        let mut entries: Vec<RM_PROCESS_INFO> =
            (0..count).map(|_| unsafe { std::mem::zeroed() }).collect();
        let status = unsafe {
            RmGetList(
                session.0,
                &mut needed,
                &mut count,
                entries.as_mut_ptr(),
                &mut reboot,
            )
        };
        if status == ERROR_MORE_DATA {
            continue;
        }
        if status != 0 {
            return Err(api_error("Restart Manager discovery", status));
        }
        let mut unique = BTreeMap::new();
        for entry in entries.into_iter().take(count as usize) {
            let mut process = LockProcess {
                pid: entry.Process.dwProcessId,
                started_at: filetime(entry.Process.ProcessStartTime).to_string(),
                name: text_wide(&entry.strAppName),
                executable: None,
                application_type: entry.ApplicationType,
                session_id: entry.TSSessionId,
                service: text_wide(&entry.strServiceShortName),
                can_close: false,
                can_terminate: false,
                unavailable_reason: None,
            };
            match validated_process(&process, false) {
                Ok((handle, executable)) => {
                    process.executable = executable
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned());
                    process.can_terminate = validated_process(&process, true).is_ok();
                    process.can_close = !process_windows(process.pid).is_empty();
                    if !process.can_close {
                        process.unavailable_reason = Some(
                            "No eligible application window is available for graceful close".into(),
                        );
                    }
                    if !process.can_terminate && process.can_close {
                        process.unavailable_reason = Some("Forced termination permission could not be established; graceful close remains available".into());
                    }
                    drop(handle);
                }
                Err(reason) => {
                    if let Ok(handle) = open_process(process.pid, false) {
                        process.executable = executable_path(handle.0).ok().and_then(|path| {
                            path.file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                        });
                    }
                    process.unavailable_reason = Some(reason);
                }
            }
            unique.insert((process.pid, process.started_at.clone()), process);
        }
        return Ok(unique.into_values().collect());
    }
    Err("Process list changed repeatedly; refresh diagnostics".into())
}

fn open_process(pid: u32, force: bool) -> Result<OwnedHandle, String> {
    let rights = PROCESS_QUERY_LIMITED_INFORMATION
        | PROCESS_SYNCHRONIZE
        | if force { PROCESS_TERMINATE } else { 0 };
    let handle = unsafe { OpenProcess(rights, 0, pid) };
    if handle.is_null() {
        Err(last_error("Cannot access process"))
    } else {
        Ok(OwnedHandle(handle))
    }
}

fn executable_path(handle: HANDLE) -> Result<PathBuf, String> {
    let mut buffer = vec![0_u16; 32_768];
    let mut size = buffer.len() as u32;
    if unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size) } == 0 {
        return Err(last_error("Cannot identify executable"));
    }
    Ok(PathBuf::from(String::from_utf16_lossy(
        &buffer[..size as usize],
    )))
}

fn user_sid(process: HANDLE) -> Result<Vec<u8>, String> {
    let mut token = std::ptr::null_mut();
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return Err(last_error("Cannot verify process owner"));
    }
    let token = OwnedHandle(token);
    let mut needed = 0;
    unsafe {
        GetTokenInformation(token.0, TokenUser, std::ptr::null_mut(), 0, &mut needed);
    }
    if needed == 0 || needed > 65_536 {
        return Err("Cannot verify process owner".into());
    }
    // TOKEN_USER requires pointer alignment; pointer-sized storage preserves it.
    let mut buffer = vec![0_usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    } == 0
    {
        return Err(last_error("Cannot verify process owner"));
    }
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    let length = unsafe { GetLengthSid(user.User.Sid) } as usize;
    if length == 0 || length > 256 {
        return Err("Invalid process owner identity".into());
    }
    Ok(unsafe { std::slice::from_raw_parts(user.User.Sid.cast::<u8>(), length) }.to_vec())
}

fn denied_executable(path: &std::path::Path, windows_root: &std::path::Path) -> bool {
    let executable = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    let normalized = path
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    let windows = windows_root
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    normalized.starts_with(&(windows.trim_end_matches('\\').to_string() + "\\"))
        || matches!(
            executable.as_str(),
            "explorer.exe"
                | "msedgewebview2.exe"
                | "emmm.exe"
                | "emmm2.exe"
                | "msmpeng.exe"
                | "nissrv.exe"
                | "securityhealthservice.exe"
                | "smartscreen.exe"
        )
}

fn validated_process(process: &LockProcess, force: bool) -> Result<(OwnedHandle, PathBuf), String> {
    if process.pid <= 4
        || process.pid == unsafe { GetCurrentProcessId() }
        || process.application_type == RmCritical
        || process.application_type == RmService
        || !process.service.is_empty()
    {
        return Err("System, service or this application process cannot be controlled".into());
    }
    let handle = open_process(process.pid, force)?;
    if unsafe { WaitForSingleObject(handle.0, 0) } == WAIT_OBJECT_0 {
        return Err("The reported process has already exited".into());
    }
    let mut created: FILETIME = unsafe { std::mem::zeroed() };
    let mut exited = created;
    let mut kernel = created;
    let mut user = created;
    if unsafe { GetProcessTimes(handle.0, &mut created, &mut exited, &mut kernel, &mut user) } == 0
        || filetime(created).to_string() != process.started_at
    {
        return Err("The process identity changed; refresh diagnostics".into());
    }
    let mut critical = 0;
    if unsafe { IsProcessCritical(handle.0, &mut critical) } == 0 || critical != 0 {
        return Err("Critical-process safety could not be established".into());
    }
    let mut protection = PROCESS_PROTECTION_LEVEL_INFORMATION { ProtectionLevel: 0 };
    if unsafe {
        GetProcessInformation(
            handle.0,
            ProcessProtectionLevelInfo,
            (&mut protection as *mut PROCESS_PROTECTION_LEVEL_INFORMATION).cast::<c_void>(),
            std::mem::size_of_val(&protection) as u32,
        )
    } == 0
        || protection.ProtectionLevel != PROTECTION_LEVEL_NONE
    {
        return Err("Protected-process safety could not be established".into());
    }
    let mut session = 0;
    let mut own_session = 0;
    if unsafe { ProcessIdToSessionId(process.pid, &mut session) } == 0
        || unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut own_session) } == 0
        || session != own_session
        || session != process.session_id
    {
        return Err(
            "The process belongs to another session or its session cannot be verified".into(),
        );
    }
    if user_sid(handle.0)? != user_sid(unsafe { GetCurrentProcess() })? {
        return Err("The process belongs to another user".into());
    }
    let executable = executable_path(handle.0)?;
    let mut windows = [0_u16; 32_768];
    let count =
        unsafe { GetWindowsDirectoryW(windows.as_mut_ptr(), windows.len() as u32) } as usize;
    if count == 0 || count >= windows.len() {
        return Err("Windows process safety could not be established".into());
    }
    let windows = PathBuf::from(String::from_utf16_lossy(&windows[..count]));
    if denied_executable(&executable, &windows)
        || std::env::current_exe()
            .ok()
            .is_none_or(|own| own == executable)
    {
        return Err(
            "Windows shell, security or application runtime processes cannot be controlled".into(),
        );
    }
    Ok((handle, executable))
}

struct WindowQuery {
    pid: u32,
    windows: Vec<HWND>,
}
unsafe extern "system" fn collect_window(window: HWND, parameter: LPARAM) -> i32 {
    let query = unsafe { &mut *(parameter as *mut WindowQuery) };
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    if pid == query.pid && unsafe { IsWindowVisible(window) } != 0 {
        query.windows.push(window);
    }
    1
}
fn process_windows(pid: u32) -> Vec<HWND> {
    let mut query = WindowQuery {
        pid,
        windows: Vec::new(),
    };
    if unsafe {
        EnumWindows(
            Some(collect_window),
            (&mut query as *mut WindowQuery) as LPARAM,
        )
    } == 0
    {
        return Vec::new();
    }
    query.windows
}

pub(super) fn control(process: &LockProcess, force: bool) -> Result<String, String> {
    let (handle, _) = validated_process(process, force)?;
    if force {
        if unsafe { TerminateProcess(handle.0, 1) } == 0 {
            return Err(last_error("Process termination was denied"));
        }
    } else {
        let windows = process_windows(process.pid);
        if windows.is_empty() {
            return Err("No eligible application window is available for graceful close".into());
        }
        for window in windows {
            let mut current_pid = 0;
            unsafe {
                GetWindowThreadProcessId(window, &mut current_pid);
            }
            if current_pid != process.pid || unsafe { PostMessageW(window, WM_CLOSE, 0, 0) } == 0 {
                return Err(last_error("Could not request application close"));
            }
        }
    }
    match unsafe { WaitForSingleObject(handle.0, EXIT_WAIT_MS) } {
        WAIT_OBJECT_0 => Ok("The selected process exited. Retry the file operation to verify the lock is released.".into()),
        WAIT_TIMEOUT if force => Ok("Termination was requested, but process exit was not confirmed within 3 seconds. Retry the operation to verify the lock.".into()),
        WAIT_TIMEOUT => Ok("The process is still running or awaiting its own save/close confirmation.".into()),
        _ => Err(last_error("Could not confirm process exit")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_shell_security_and_runtime_are_denied() {
        let root = std::path::Path::new("C:\\Windows");
        assert!(denied_executable(
            std::path::Path::new("c:\\WINDOWS\\System32\\notepad.exe"),
            root
        ));
        assert!(denied_executable(
            std::path::Path::new("D:\\browser\\msedgewebview2.exe"),
            root
        ));
        assert!(!denied_executable(
            std::path::Path::new("D:\\WindowsGame\\game.exe"),
            root
        ));
    }
    #[test]
    fn own_process_cannot_be_controlled_even_if_attributed() {
        let process = LockProcess {
            pid: unsafe { GetCurrentProcessId() },
            started_at: "0".into(),
            name: "fixture".into(),
            executable: None,
            application_type: 1,
            session_id: 0,
            service: String::new(),
            can_close: true,
            can_terminate: true,
            unavailable_reason: None,
        };
        assert!(control(&process, true).is_err());
    }

    struct OwnedFixture(std::process::Child);
    impl Drop for OwnedFixture {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn owned_file_holder_is_discovered_identity_checked_and_force_terminated() {
        use std::{
            io::{BufRead, BufReader},
            os::windows::process::CommandExt,
            process::{Command, Stdio},
        };
        let fixture = tempfile::tempdir().unwrap();
        let source = fixture.path().join("lock_fixture.rs");
        let executable = fixture.path().join("emmm-owned-lock-fixture.exe");
        let locked_file = fixture.path().join("asset.ini");
        std::fs::write(&locked_file, b"owned native file-lock fixture").unwrap();
        std::fs::write(
            &source,
            r#"
use std::{fs::OpenOptions, io::{self, Write}, os::windows::fs::OpenOptionsExt};
fn main() {
    let path = std::env::args_os().nth(1).unwrap();
    let _file = OpenOptions::new().read(true).share_mode(3).open(path).unwrap();
    println!("READY");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}
"#,
        )
        .unwrap();
        let compile = Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let mut holder = OwnedFixture(
            Command::new(&executable)
                .arg(&locked_file)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(0x08000000)
                .spawn()
                .unwrap(),
        );
        let output = holder.0.stdout.take().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut ready = String::new();
            let result = BufReader::new(output).read_line(&mut ready).map(|_| ready);
            let _ = sender.send(result);
        });
        assert_eq!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap()
                .unwrap()
                .trim(),
            "READY"
        );
        let discovery = super::super::discover_lock_processes(&locked_file);
        let process = discovery
            .processes
            .into_iter()
            .find(|process| process.pid == holder.0.id())
            .expect("Restart Manager should report the owned file holder");
        assert!(process.can_terminate, "{:?}", process.unavailable_reason);
        assert!(!process.can_close);
        let mut forged = process.clone();
        forged.started_at = "0".into();
        assert!(control(&forged, true).is_err());
        assert!(holder.0.try_wait().unwrap().is_none());
        let outcome = control(&process, true).unwrap();
        assert!(outcome.contains("exited"), "{outcome}");
        assert!(holder.0.wait().unwrap().code().is_some());
        let discovery = super::super::discover_lock_processes(&locked_file);
        assert!(!discovery
            .processes
            .iter()
            .any(|current| current.pid == process.pid && current.started_at == process.started_at));
    }
}
