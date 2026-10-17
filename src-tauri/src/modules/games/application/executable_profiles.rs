use crate::modules::games::domain::{executable::*, models::GameType};
use crate::shared::errors::AppError;
use std::path::{Path, PathBuf};

pub const MAX_EXECUTABLE_PATH: usize = 4096;
pub const WWMI_WRAPPER: &str = "Wuthering Waves.exe";
pub const WWMI_PROCESS: &str = "Client/Binaries/Win64/Client-Win64-Shipping.exe";

pub fn executable_names(game_type: GameType) -> &'static [&'static str] {
    match game_type {
        GameType::Gimi => &["GenshinImpact.exe", "YuanShen.exe"],
        GameType::Srmi => &["StarRail.exe"],
        GameType::Zzmi => &["ZenlessZoneZero.exe", "ZenlessZoneZeroBeta.exe"],
        GameType::Wwmi => &[WWMI_WRAPPER],
        GameType::Efmi => &["Endfield.exe"],
    }
}

pub fn validate_local_path(value: &str) -> Result<PathBuf, AppError> {
    if value.is_empty()
        || value.len() > MAX_EXECUTABLE_PATH
        || value.contains(['\0', '\r', '\n', '"'])
        || value.starts_with("\\\\") && !value.starts_with("\\\\?\\")
        || value.starts_with("//")
        || value.to_ascii_lowercase().starts_with("\\\\?\\unc\\")
        || value.starts_with("\\\\.\\")
    {
        return Err(AppError::Validation(
            "Expected a local absolute file location.".into(),
        ));
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(AppError::Validation(
            "Expected a local absolute file location.".into(),
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::path::{Component, Prefix};
        use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;
        use windows_sys::Win32::System::WindowsProgramming::DRIVE_REMOTE;
        if !matches!(path.components().next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        {
            return Err(AppError::Validation(
                "Expected a local drive location.".into(),
            ));
        }
        let mut root = PathBuf::new();
        for component in path.components().take(2) {
            root.push(component);
        }
        let wide: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { GetDriveTypeW(wide.as_ptr()) } == DRIVE_REMOTE {
            return Err(AppError::Validation(
                "Remote discovery locations are unsupported.".into(),
            ));
        }
    }
    Ok(path)
}

pub fn validate_game_executable(
    game_type: GameType,
    executable_path: &str,
) -> Result<GameExecutableCandidate, AppError> {
    inspect_game_executable(game_type, executable_path).map_err(|error| error.error)
}

pub(crate) struct ExecutableProfileError {
    pub code: GameExecutableIssueCode,
    error: AppError,
}

impl From<AppError> for ExecutableProfileError {
    fn from(error: AppError) -> Self {
        Self {
            code: GameExecutableIssueCode::InvalidLocation,
            error,
        }
    }
}

impl From<std::io::Error> for ExecutableProfileError {
    fn from(error: std::io::Error) -> Self {
        let code = match error.kind() {
            std::io::ErrorKind::PermissionDenied => GameExecutableIssueCode::AccessDenied,
            std::io::ErrorKind::NotFound => GameExecutableIssueCode::InvalidLocation,
            _ => GameExecutableIssueCode::ReadFailed,
        };
        Self {
            code,
            error: AppError::Validation(
                "The selected game location is unavailable or unreadable. Detect or browse again."
                    .into(),
            ),
        }
    }
}

pub(crate) fn inspect_game_executable(
    game_type: GameType,
    executable_path: &str,
) -> Result<GameExecutableCandidate, ExecutableProfileError> {
    let requested = validate_local_path(executable_path)?;
    let executable = requested.canonicalize()?;
    validate_local_path(&executable.to_string_lossy())?;
    if !executable.metadata()?.is_file() {
        return Err(AppError::Validation("Select an existing game executable file.".into()).into());
    }
    let name = executable
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let mut root = executable
        .parent()
        .ok_or_else(|| AppError::Validation("Invalid game location.".into()))?
        .to_path_buf();
    let shipping =
        game_type == GameType::Wwmi && name.eq_ignore_ascii_case("Client-Win64-Shipping.exe");
    if shipping {
        root = executable
            .ancestors()
            .nth(4)
            .ok_or_else(|| AppError::Validation("Invalid WWMI installation.".into()))?
            .to_path_buf();
        if root.join(WWMI_PROCESS).canonicalize()? != executable {
            return Err(AppError::Validation("Invalid WWMI process location.".into()).into());
        }
    } else if !executable_names(game_type)
        .iter()
        .any(|expected| name.eq_ignore_ascii_case(expected))
    {
        return Err(AppError::Validation(
            "The executable does not match the selected game.".into(),
        )
        .into());
    }
    let markers: &[&str] = match game_type {
        GameType::Gimi => &["GenshinImpact_Data"],
        GameType::Srmi => &["StarRail_Data"],
        GameType::Zzmi => &["ZenlessZoneZero_Data"],
        GameType::Wwmi => &["Client", "Engine"],
        GameType::Efmi => &["Endfield_Data"],
    };
    for marker in markers {
        let marker = root.join(marker);
        if !marker.metadata()?.is_dir() || !marker.canonicalize()?.starts_with(&root) {
            return Err(AppError::Validation(
                "The selected game installation is incomplete or has moved.".into(),
            )
            .into());
        }
    }
    let process = if game_type == GameType::Wwmi {
        let wrapper = root.join(WWMI_WRAPPER);
        let process = root.join(WWMI_PROCESS).canonicalize()?;
        if !wrapper.metadata()?.is_file()
            || !process.metadata()?.is_file()
            || !process.starts_with(&root)
        {
            return Err(AppError::Validation("The WWMI installation is incomplete.".into()).into());
        }
        process
    } else {
        executable.clone()
    };
    Ok(GameExecutableCandidate {
        game_type,
        executable_path: executable.to_string_lossy().into_owned(),
        install_root: root.to_string_lossy().into_owned(),
        process_executable_path: process.to_string_lossy().into_owned(),
        sources: vec![GameExecutableSource::Manual],
        region: None,
        channel: name
            .eq_ignore_ascii_case("ZenlessZoneZeroBeta.exe")
            .then(|| "beta".into()),
    })
}

/// Registry/config discovery is never invoked by this runtime-only resolver.
pub fn resolve_process_executable(
    game_type: GameType,
    configured: Option<&Path>,
) -> Option<PathBuf> {
    let configured = configured?;
    if game_type == GameType::Wwmi
        && configured
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case(WWMI_WRAPPER))
    {
        return validate_game_executable(game_type, &configured.to_string_lossy())
            .ok()
            .map(|candidate| PathBuf::from(candidate.process_executable_path));
    }
    Some(configured.to_path_buf())
}
