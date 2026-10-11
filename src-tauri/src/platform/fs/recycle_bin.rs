use crate::shared::errors::AppError;
use std::path::Path;

pub fn move_path_to_recycle_bin(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "Path does not exist: {}",
            path.display()
        )));
    }

    let original_identity = crate::platform::fs::file_utils::filesystem_identity(path);
    trash::delete(path).map_err(|error| {
        #[cfg(windows)]
        if let trash::Error::Os { code, .. } = &error {
            if let Some(os_error) = recycle_win32_error(*code) {
                if crate::platform::fs::retry::is_file_lock(&os_error) {
                    if let Some(identity) = original_identity.as_deref() {
                        return crate::platform::fs::retry::map_file_error_with_identity(
                            path,
                            identity,
                            "move folder to Recycle Bin",
                            os_error,
                        );
                    }
                }
            }
        }
        #[cfg(not(windows))]
        let _ = &original_identity;
        AppError::Io(format!(
            "Failed to move '{}' to the system Recycle Bin: {error}",
            path.display()
        ))
    })
}

#[cfg(windows)]
fn recycle_win32_error(code: i32) -> Option<std::io::Error> {
    const WIN32_HRESULT_PREFIX: u32 = 0x8007_0000;
    const HRESULT_FACILITY_MASK: u32 = 0xffff_0000;
    const HRESULT_ERROR_MASK: u32 = 0x0000_ffff;
    let hresult = code as u32;
    (hresult & HRESULT_FACILITY_MASK == WIN32_HRESULT_PREFIX)
        .then(|| std::io::Error::from_raw_os_error((hresult & HRESULT_ERROR_MASK) as i32))
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn recycle_hresult_preserves_sharing_violation_without_guessing_unknown_errors() {
        assert_eq!(
            super::recycle_win32_error(0x8007_0020_u32 as i32)
                .unwrap()
                .raw_os_error(),
            Some(32)
        );
        assert_eq!(
            super::recycle_win32_error(0x8007_0005_u32 as i32)
                .unwrap()
                .raw_os_error(),
            Some(5)
        );
        assert!(super::recycle_win32_error(0x8000_4005_u32 as i32).is_none());
    }
}
