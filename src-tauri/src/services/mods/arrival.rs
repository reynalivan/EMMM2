//! Where a freshly extracted mod folder lands.
//!
//! Two import paths reach this rule — the manual drop/extract command and the
//! browser download pipeline — and each had its own copy. The browser side
//! used `standardize_prefix` and uniquified collisions; the manual side
//! hand-built `format!("{DISABLED_PREFIX}{name}")` behind an
//! `is_disabled_folder` guard and silently kept the enabled path when the
//! rename failed.

use std::path::{Path, PathBuf};

use crate::domain::errors::AppError;
use crate::services::fs_utils::file_utils::rename_cross_drive_fallback;
use crate::services::mods::core_ops::standardize_prefix;

/// Move an extracted folder into `target_root`, disabled.
///
/// New arrivals land disabled so nothing activates in-game before the user has
/// looked at it. `standardize_prefix` is idempotent against the legacy
/// `disabled_` / `DISABLED-` spellings, so an archive that already contained a
/// disabled folder is not double-prefixed.
///
/// Returns the folder's final path.
pub fn land_disabled(extracted: &Path, target_root: &Path) -> Result<PathBuf, AppError> {
    let name = extracted
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "mod".to_string());

    // Identity first, then collisions: a folder already sitting at its landing
    // name collides with *itself*, and uniquifying would rename it to " (2)".
    let landed = target_root.join(standardize_prefix(&name, false));
    if landed == extracted {
        return Ok(landed);
    }
    let dest = unique_destination(landed);

    // `rename_cross_drive_fallback` already tries the plain rename first.
    rename_cross_drive_fallback(extracted, &dest)?;
    Ok(dest)
}

/// The first free name at `dest`, suffixing " (2)", " (3)" … as needed.
pub fn unique_destination(dest: PathBuf) -> PathBuf {
    if !dest.exists() {
        return dest;
    }
    let base = dest
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "mod".to_string());
    let parent = dest.parent().unwrap_or(Path::new(".")).to_path_buf();

    for n in 2u32.. {
        let candidate = parent.join(format!("{base} ({n})"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("u32 range is exhausted long before the filesystem is")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn arrival_is_disabled_and_not_double_prefixed() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();

        let plain = root.join("Blue Dress");
        std::fs::create_dir(&plain).expect("create");
        let landed = land_disabled(&plain, root).expect("land");
        assert_eq!(
            landed.file_name().unwrap().to_string_lossy(),
            format!("{}Blue Dress", crate::DISABLED_PREFIX)
        );

        let already = root.join(format!("{}Red Dress", crate::DISABLED_PREFIX));
        std::fs::create_dir(&already).expect("create");
        let landed = land_disabled(&already, root).expect("land");
        assert_eq!(
            landed.file_name().unwrap().to_string_lossy(),
            format!("{}Red Dress", crate::DISABLED_PREFIX)
        );
    }

    #[test]
    fn a_taken_name_gets_a_suffix() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        std::fs::create_dir(root.join(format!("{}Boots", crate::DISABLED_PREFIX))).expect("create");

        let incoming = root.join("staged");
        std::fs::create_dir(&incoming).expect("create");
        std::fs::create_dir(incoming.join("Boots")).expect("create");

        let landed = land_disabled(&incoming.join("Boots"), root).expect("land");
        assert_eq!(
            landed.file_name().unwrap().to_string_lossy(),
            format!("{}Boots (2)", crate::DISABLED_PREFIX)
        );
    }
}
