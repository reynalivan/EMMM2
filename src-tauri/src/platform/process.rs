use std::path::Path;

use crate::shared::errors::AppError;

#[cfg(target_os = "windows")]
fn wide_null(value: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    value.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn shell_execute(
    verb: &str,
    path: &Path,
    working_directory: Option<&Path>,
) -> Result<(), AppError> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let verb = wide_null(std::ffi::OsStr::new(verb));
    let path_wide = wide_null(path.as_os_str());
    let working_directory_wide =
        working_directory.map(|directory| wide_null(directory.as_os_str()));
    let working_directory_ptr = working_directory_wide
        .as_ref()
        .map_or(std::ptr::null(), |directory| directory.as_ptr());

    // ShellExecuteW receives separately encoded values, so no user-controlled path is parsed by a shell.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            path_wide.as_ptr(),
            std::ptr::null(),
            working_directory_ptr,
            SW_SHOWNORMAL,
        )
    } as isize;

    if result > 32 {
        return Ok(());
    }

    let reason = if result == 5 && verb.starts_with(&['r' as u16, 'u' as u16, 'n' as u16]) {
        "Elevation was cancelled or denied".to_string()
    } else {
        format!("ShellExecuteW returned error code {result}")
    };
    Err(AppError::Io(format!(
        "Failed to open '{}': {reason}",
        path.display()
    )))
}

pub fn launch_elevated(executable: &Path, working_directory: &Path) -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        shell_execute("runas", executable, Some(working_directory))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (executable, working_directory);
        Err(AppError::Io(
            "Elevated launch is only supported on Windows".to_string(),
        ))
    }
}

pub fn open_path(path: &Path) -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        shell_execute("open", path, path.parent())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err(AppError::Io(
            "Opening files in their default application is only supported on Windows".to_string(),
        ))
    }
}

pub fn reveal_in_file_manager(path: &Path) -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        let mut command = std::process::Command::new("explorer");
        if path.is_file() {
            command.arg("/select,");
        }
        command.arg(path);
        command.spawn().map(|_| ()).map_err(|error| {
            AppError::Io(format!("Failed to reveal '{}': {error}", path.display()))
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err(AppError::Io(
            "Revealing files is only supported on Windows".to_string(),
        ))
    }
}
