use crate::shared::errors::AppError;
use crate::shared::sync::lock;
use std::path::Path;

static DISKS_CACHE: std::sync::OnceLock<std::sync::Mutex<sysinfo::Disks>> =
    std::sync::OnceLock::new();

pub(super) const ARCHIVE_DISK_RESERVE_BYTES: u64 = 50 * 1024 * 1024;

pub(super) fn check_disk_space(mods_dir: &Path, required_space: u64) -> Result<(), AppError> {
    let Some(available_space) = available_disk_space(mods_dir) else {
        return Ok(());
    };

    if available_space < required_space {
        return Err(AppError::Internal(format!(
            "Insufficient disk space. Requires {} bytes, but only {} bytes available.",
            required_space, available_space
        )));
    }

    Ok(())
}

pub(super) fn effective_archive_limits(
    staging_dir: &Path,
    mut limits: super::security::ArchiveLimits,
) -> Result<super::security::ArchiveLimits, AppError> {
    let Some(available_space) = available_disk_space(staging_dir) else {
        return Ok(limits);
    };
    let usable_space = available_space.saturating_sub(ARCHIVE_DISK_RESERVE_BYTES);
    if usable_space == 0 {
        return Err(AppError::Internal(
            "Insufficient disk space after reserving workspace safety margin".to_string(),
        ));
    }
    limits.max_total_bytes = limits.max_total_bytes.min(usable_space);
    Ok(limits)
}

fn available_disk_space(path: &Path) -> Option<u64> {
    let mutex = DISKS_CACHE
        .get_or_init(|| std::sync::Mutex::new(sysinfo::Disks::new_with_refreshed_list()));
    let mut disks = lock(mutex);
    disks.refresh(true);

    let search_path = path
        .ancestors()
        .find_map(|ancestor| ancestor.canonicalize().ok())?;

    let mut available_space = 0_u64;
    let mut matched_len = 0_usize;
    for disk in disks.list() {
        let mount = disk
            .mount_point()
            .canonicalize()
            .unwrap_or_else(|_| disk.mount_point().to_path_buf());
        if !mount_contains_path(&search_path, &mount) {
            continue;
        }

        let mount_len = mount.components().count();
        if mount_len > matched_len {
            matched_len = mount_len;
            available_space = disk.available_space();
        }
    }

    (matched_len > 0).then_some(available_space)
}

#[cfg(not(windows))]
fn mount_contains_path(path: &Path, mount: &Path) -> bool {
    path.starts_with(mount)
}

#[cfg(windows)]
fn mount_contains_path(path: &Path, mount: &Path) -> bool {
    use std::path::{Component, Prefix};

    let mut components = path.components();
    mount.components().all(
        |mount_component| match (components.next(), mount_component) {
            (Some(Component::Prefix(path_prefix)), Component::Prefix(mount_prefix)) => {
                match (path_prefix.kind(), mount_prefix.kind()) {
                    (
                        Prefix::Disk(left) | Prefix::VerbatimDisk(left),
                        Prefix::Disk(right) | Prefix::VerbatimDisk(right),
                    ) => left.eq_ignore_ascii_case(&right),
                    (
                        Prefix::UNC(left_server, left_share)
                        | Prefix::VerbatimUNC(left_server, left_share),
                        Prefix::UNC(right_server, right_share)
                        | Prefix::VerbatimUNC(right_server, right_share),
                    ) => left_server == right_server && left_share == right_share,
                    (left, right) => left == right,
                }
            }
            (Some(path_component), mount_component) => path_component == mount_component,
            (None, _) => false,
        },
    )
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn disk_space_accepts_verbatim_drive_and_unc_namespaces() {
        for (path, mount) in [
            (r"\\?\C:\imports\staging", r"C:\"),
            (r"C:\imports\staging", r"\\?\C:\"),
            (r"\\?\UNC\server\share\imports", r"\\server\share\"),
        ] {
            assert!(mount_contains_path(Path::new(path), Path::new(mount)));
        }
        assert!(!mount_contains_path(
            Path::new(r"\\?\C:\imports\staging"),
            Path::new(r"D:\"),
        ));
        assert!(!mount_contains_path(
            Path::new(r"\\?\C:\imports-other\staging"),
            Path::new(r"C:\imports"),
        ));
    }

    #[test]
    fn disk_space_resolves_real_windows_staging_and_missing_descendants() {
        let directory = tempfile::tempdir().unwrap();
        let canonical = directory.path().canonicalize().unwrap();
        assert!(available_disk_space(&canonical).is_some());
        assert!(available_disk_space(&canonical.join("missing").join("staging")).is_some());
    }
}
