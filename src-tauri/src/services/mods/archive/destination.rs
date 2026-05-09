use crate::services::fs_utils::file_utils::rename_cross_drive_fallback;
use std::fs;
use std::path::{Path, PathBuf};

static DISKS_CACHE: std::sync::OnceLock<std::sync::Mutex<sysinfo::Disks>> =
    std::sync::OnceLock::new();

pub(super) fn parent_dir_join(parent: &Path, name: &str) -> PathBuf {
    parent.join(name)
}

pub(super) fn remove_existing_dest(dest: &Path) -> Result<(), String> {
    if !dest.exists() {
        return Ok(());
    }

    fs::remove_dir_all(dest).map_err(|error| {
        format!(
            "Failed to remove existing destination '{}': {error}",
            dest.display()
        )
    })
}

pub(super) fn move_to_extracted_dir(archive_path: &Path) -> Result<(), String> {
    let parent = archive_path
        .parent()
        .ok_or("Archive has no parent directory")?;
    let extracted_dir = parent.join(".extracted");
    fs::create_dir_all(&extracted_dir)
        .map_err(|error| format!("Failed to create .extracted dir: {error}"))?;

    let file_name = archive_path.file_name().ok_or("No filename")?;
    let mut dest = extracted_dir.join(file_name);

    if dest.exists() {
        let stem = archive_path
            .file_stem()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = archive_path
            .extension()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut counter = 2_u32;

        loop {
            let new_name = if ext.is_empty() {
                format!("{} ({})", stem, counter)
            } else {
                format!("{} ({}).{}", stem, counter, ext)
            };
            let candidate = extracted_dir.join(new_name);
            if !candidate.exists() {
                dest = candidate;
                break;
            }
            counter += 1;
        }
    }

    rename_cross_drive_fallback(archive_path, &dest)
        .map_err(|error| format!("Failed to move archive to .extracted: {error}"))
}

pub(super) fn check_disk_space(mods_dir: &Path, required_space: u64) -> Result<(), String> {
    let mutex = DISKS_CACHE
        .get_or_init(|| std::sync::Mutex::new(sysinfo::Disks::new_with_refreshed_list()));
    let mut disks = mutex.lock().unwrap();
    disks.refresh(true);

    let search_path = mods_dir
        .canonicalize()
        .unwrap_or_else(|_| mods_dir.to_path_buf());

    let mut available_space = 0_u64;
    let mut matched_len = 0_usize;
    for disk in disks.list() {
        let mount = disk.mount_point();
        if !search_path.starts_with(mount) {
            continue;
        }

        let mount_len = mount.as_os_str().len();
        if mount_len > matched_len {
            matched_len = mount_len;
            available_space = disk.available_space();
        }
    }

    if matched_len > 0 && available_space < required_space {
        return Err(format!(
            "Insufficient disk space. Requires {} bytes, but only {} bytes available.",
            required_space, available_space
        ));
    }

    Ok(())
}
