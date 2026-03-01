use super::types::{ArchiveFormat, ExtractionResult};
use std::fs;
use std::io;
use std::path::Path;

const ARCHIVE_BACKUP_DIR: &str = ".archive_backup";

/// Extract any supported archive to a destination directory.
///
/// Steps:
/// 1. Create destination folder (named after archive, sans extension)
/// 2. Extract all files (with optional password for encrypted archives)
/// 3. Apply smart flattening if single wrapper folder detected
/// 4. Move original archive to `.archive_backup/`
///
/// # Covers: TC-2.1-01, TC-2.1-02, TC-2.1-04, TC-2.1-05, EC-2.07 (Overwrite)
pub fn extract_archive(
    archive_path: &Path,
    mods_dir: &Path,
    password: Option<&str>,
    overwrite: bool,
) -> Result<ExtractionResult, String> {
    let format = ArchiveFormat::from_path(archive_path)
        .ok_or_else(|| format!("Unsupported archive format: {}", archive_path.display()))?;

    let mut archive_name = archive_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "extracted_mod".to_string());

    let mut dest_path = mods_dir.join(&archive_name);

    // Pre-extract disk space check
    let analysis = crate::services::mods::archive::analyze::analyze_archive(archive_path)?;
    // Require uncompressed size + 50MB buffer
    let required_space = analysis.uncompressed_size + (50 * 1024 * 1024);

    let disks = sysinfo::Disks::new_with_refreshed_list();
    let search_path = mods_dir
        .canonicalize()
        .unwrap_or_else(|_| mods_dir.to_path_buf());

    let mut available_space = 0;
    let mut matched_len = 0;
    for disk in disks.list() {
        let mount = disk.mount_point();
        if search_path.starts_with(mount) {
            let mount_len = mount.as_os_str().len();
            if mount_len > matched_len {
                matched_len = mount_len;
                available_space = disk.available_space();
            }
        }
    }

    if matched_len > 0 && available_space < required_space {
        return Err(format!(
            "Insufficient disk space. Requires {} bytes, but only {} bytes available.",
            required_space, available_space
        ));
    }

    // Guard: destination already exists
    if dest_path.exists() {
        if !overwrite {
            let mut counter = 1;
            loop {
                let new_name = format!("{} ({})", archive_name, counter);
                let check_path = mods_dir.join(&new_name);
                if !check_path.exists() {
                    archive_name = new_name;
                    dest_path = check_path;
                    break;
                }
                counter += 1;
            }
        } else {
            log::info!("Overwriting destination: {}", dest_path.display());
            // We allow overwrite/merge
        }
    }

    fs::create_dir_all(&dest_path).map_err(|e| format!("Failed to create destination: {e}"))?;

    let files_extracted = match format {
        ArchiveFormat::Zip => extract_zip_inner(archive_path, &dest_path, password)?,
        ArchiveFormat::SevenZ => extract_7z_inner(archive_path, &dest_path, password)?,
        ArchiveFormat::Rar => extract_rar_inner(archive_path, &dest_path, password)?,
    };

    // Smart flattening
    if let Err(e) = flatten_if_needed(&dest_path) {
        log::warn!("Smart flattening failed (non-fatal): {e}");
    }

    // Move original to backup
    if let Err(e) = move_to_backup(archive_path, mods_dir) {
        log::warn!("Failed to move archive to backup (non-fatal): {e}");
    }

    Ok(ExtractionResult {
        archive_name,
        dest_path: dest_path.to_string_lossy().to_string(),
        files_extracted,
        success: true,
        error: None,
    })
}

fn extract_zip_inner(
    archive_path: &Path,
    dest_path: &Path,
    password: Option<&str>,
) -> Result<usize, String> {
    let file = fs::File::open(archive_path).map_err(|e| format!("Failed to open archive: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Invalid or corrupt ZIP: {e}"))?;

    let mut count: usize = 0;
    for i in 0..archive.len() {
        // Use password-aware read if password provided
        let mut entry = match password {
            Some(pw) => match archive.by_index_decrypt(i, pw.as_bytes()) {
                Ok(file) => file,
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("Password") || msg.contains("password") {
                        return Err("Password required to extract this archive".to_string());
                    } else {
                        return Err(format!("Failed to decrypt entry {i}: {e}"));
                    }
                }
            },
            None => match archive.by_index(i) {
                Ok(file) => file,
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("Password") || msg.contains("password") {
                        return Err("Password required to extract this archive".to_string());
                    } else {
                        return Err(format!("Failed to read entry {i}: {e}"));
                    }
                }
            },
        };

        let entry_path = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => continue, // Skip unsafe paths
        };

        let output_path = dest_path.join(&entry_path);

        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(|e| format!("Failed to create dir: {e}"))?;
        } else {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent: {e}"))?;
            }
            let mut outfile = fs::File::create(&output_path)
                .map_err(|e| format!("Failed to create file: {e}"))?;
            io::copy(&mut entry, &mut outfile).map_err(|e| format!("Failed to write file: {e}"))?;
            count += 1;
        }
    }
    Ok(count)
}

fn extract_7z_inner(
    archive_path: &Path,
    dest_path: &Path,
    password: Option<&str>,
) -> Result<usize, String> {
    let extract_result = match password {
        Some(pw) => sevenz_rust::decompress_file_with_password(archive_path, dest_path, pw.into()),
        None => sevenz_rust::decompress_file(archive_path, dest_path),
    };

    extract_result.map_err(|e| {
        let msg = e.to_string();
        if msg.contains("password") || msg.contains("Password") || msg.contains("decrypt") {
            "Password required to extract this archive".to_string()
        } else {
            format!("Failed to extract 7z: {e}")
        }
    })?;

    // Count extracted files
    let count = walkdir::WalkDir::new(dest_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .count();

    Ok(count)
}

fn extract_rar_inner(
    archive_path: &Path,
    dest_path: &Path,
    password: Option<&str>,
) -> Result<usize, String> {
    let path_str = archive_path
        .to_str()
        .ok_or("RAR path contains invalid UTF-8")?;
    let dest_str = dest_path
        .to_str()
        .ok_or("Dest path contains invalid UTF-8")?;

    let pw = password.unwrap_or("");
    rar::Archive::extract_all(path_str, dest_str, pw)
        .map_err(|e| format!("Failed to extract RAR: {e:?}"))?;

    // Count extracted files
    let count = walkdir::WalkDir::new(dest_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .count();

    Ok(count)
}

/// Smart flattening: if extracted folder contains a single subfolder,
/// move its contents up one level and remove the wrapper.
///
/// # Covers: TC-2.1-02
pub fn flatten_if_needed(dest_path: &Path) -> Result<(), String> {
    let entries: Vec<_> = fs::read_dir(dest_path)
        .map_err(|e| format!("Failed to read dest: {e}"))?
        .filter_map(|e| e.ok())
        .collect();

    if entries.len() != 1 {
        return Ok(());
    }

    let single_entry = &entries[0];
    if !single_entry.path().is_dir() {
        return Ok(());
    }

    let wrapper_path = single_entry.path();
    let wrapper_children: Vec<_> = fs::read_dir(&wrapper_path)
        .map_err(|e| format!("Failed to read wrapper: {e}"))?
        .filter_map(|e| e.ok())
        .collect();

    for child in wrapper_children {
        let child_name = child.file_name();
        let new_location = dest_path.join(&child_name);

        if new_location.exists() {
            log::warn!(
                "Skip flatten: {} already exists at destination",
                child_name.to_string_lossy()
            );
            continue;
        }

        crate::services::fs_utils::file_utils::rename_cross_drive_fallback(
            &child.path(),
            &new_location,
        )
        .map_err(|e| format!("Failed to move {}: {e}", child_name.to_string_lossy()))?;
    }

    // Remove empty wrapper
    if fs::read_dir(&wrapper_path)
        .map(|mut d| d.next().is_none())
        .unwrap_or(false)
    {
        let _ = fs::remove_dir(&wrapper_path);
    }

    Ok(())
}

/// Move archive to `.archive_backup/` directory.
fn move_to_backup(archive_path: &Path, mods_dir: &Path) -> Result<(), String> {
    let backup_dir = mods_dir.join(ARCHIVE_BACKUP_DIR);
    fs::create_dir_all(&backup_dir).map_err(|e| format!("Failed to create backup dir: {e}"))?;

    let file_name = archive_path.file_name().ok_or("No filename")?;
    let backup_path = backup_dir.join(file_name);

    crate::services::fs_utils::file_utils::rename_cross_drive_fallback(archive_path, &backup_path)
        .map_err(|e| format!("Failed to move archive to backup: {e}"))?;

    Ok(())
}
