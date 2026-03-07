use super::classify::{collect_loose_files_recursive, find_mod_roots, resolve_unique_dest};
use super::types::{ArchiveFormat, ExtractionResult};
use crate::services::fs_utils::file_utils::rename_cross_drive_fallback;
use std::fs;
use std::io;
use std::path::Path;

/// Extract any supported archive with smart mod root detection.
///
/// Pipeline:
/// 1. Extract to `{mods_dir}/.temp_extract/<uuid>/`
/// 2. Find mod roots (shallowest folders with valid 3DMigoto .ini)
/// 3. Collect loose files (readme, images) from wrapper layers
/// 4. Route based on classification:
///    - Single mod → move to `mods_dir/{name}/`
///    - Multi-mod pack → move each subfolder independently
///    - Invalid → delete temp, return error
/// 5. Move source archive to `{source_dir}/.extracted/`
///
/// # Covers: TC-2.1-01, TC-2.1-02, TC-2.1-04, TC-2.1-05, EC-2.07
pub fn extract_archive(
    archive_path: &Path,
    mods_dir: &Path,
    password: Option<&str>,
    _overwrite: bool,
) -> Result<ExtractionResult, String> {
    let format = ArchiveFormat::from_path(archive_path)
        .ok_or_else(|| format!("Unsupported archive format: {}", archive_path.display()))?;

    let archive_name = archive_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "extracted_mod".to_string());

    // Pre-extract disk space check
    let analysis = crate::services::mods::archive::analyze_archive(archive_path)?;
    let required_space = analysis.uncompressed_size + (50 * 1024 * 1024);
    check_disk_space(mods_dir, required_space)?;

    // Phase 1: Extract to temp staging
    let uuid = uuid::Uuid::new_v4().to_string();
    let temp_dir = mods_dir.join(".temp_extract").join(&uuid);
    fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {e}"))?;

    let files_extracted = match extract_to_dir(archive_path, &temp_dir, password, format) {
        Ok(count) => count,
        Err(e) => {
            fs::remove_dir_all(&temp_dir).ok();
            return Err(e);
        }
    };

    // Phase 2: Find mod roots
    let mod_roots = find_mod_roots(&temp_dir, 5);

    if mod_roots.is_empty() {
        fs::remove_dir_all(&temp_dir).ok();
        return Err("Not a valid 3DMigoto mod archive (no valid .ini found)".into());
    }

    // Phase 3: Collect loose files from wrapper layers above mod roots
    let loose_files = collect_loose_files_recursive(&temp_dir, &mod_roots);

    // Phase 4: Route based on classification
    let mut dest_paths = Vec::new();

    if mod_roots.len() == 1 && mod_roots[0] == temp_dir {
        // Case 1: Mod at temp root (already flat) → wrap in archive name
        let dest = resolve_unique_dest(mods_dir, &archive_name);
        rename_cross_drive_fallback(&temp_dir, &dest)
            .map_err(|e| format!("Failed to move mod to destination: {e}"))?;
        dest_paths.push(dest.to_string_lossy().to_string());
    } else {
        // Cases 2-6: Move each mod root independently
        for (i, root) in mod_roots.iter().enumerate() {
            let name = root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| archive_name.clone());
            let dest = resolve_unique_dest(mods_dir, &name);
            rename_cross_drive_fallback(root, &dest)
                .map_err(|e| format!("Failed to move mod '{}' to destination: {e}", name))?;

            // Copy loose files into first mod folder only
            if i == 0 {
                for lf in &loose_files {
                    if let Some(fname) = lf.file_name() {
                        let target = dest.join(fname);
                        if !target.exists() {
                            rename_cross_drive_fallback(lf, &target).ok();
                        }
                    }
                }
            }

            dest_paths.push(dest.to_string_lossy().to_string());
        }

        // Clean remaining temp junk (wrapper dirs, loose files already copied)
        fs::remove_dir_all(&temp_dir).ok();
    }

    // Phase 5: Move source archive to .extracted/
    if let Err(e) = move_to_extracted_dir(archive_path) {
        log::warn!("Failed to move archive to .extracted/ (non-fatal): {e}");
    }

    let mod_count = dest_paths.len();

    Ok(ExtractionResult {
        archive_name,
        dest_paths,
        files_extracted,
        mod_count,
        success: true,
        error: None,
    })
}

/// Extract archive contents to a destination directory.
fn extract_to_dir(
    archive_path: &Path,
    dest_path: &Path,
    password: Option<&str>,
    format: ArchiveFormat,
) -> Result<usize, String> {
    match format {
        ArchiveFormat::Zip => extract_zip_inner(archive_path, dest_path, password),
        ArchiveFormat::SevenZ => extract_7z_inner(archive_path, dest_path, password),
        ArchiveFormat::Rar => extract_rar_inner(archive_path, dest_path, password),
    }
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
            None => continue,
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

    let count = walkdir::WalkDir::new(dest_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .count();

    Ok(count)
}

/// Move source archive to `.extracted/` subfolder in its original location.
///
/// Example: `C:/Downloads/mod.zip` → `C:/Downloads/.extracted/mod.zip`
/// This is instant (same-drive rename) and hides the file from the user.
fn move_to_extracted_dir(archive_path: &Path) -> Result<(), String> {
    let parent = archive_path
        .parent()
        .ok_or("Archive has no parent directory")?;
    let extracted_dir = parent.join(".extracted");
    fs::create_dir_all(&extracted_dir)
        .map_err(|e| format!("Failed to create .extracted dir: {e}"))?;

    let file_name = archive_path.file_name().ok_or("No filename")?;
    let mut dest = extracted_dir.join(file_name);

    // Handle duplicate names in .extracted/
    if dest.exists() {
        let stem = archive_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = archive_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut counter = 2u32;
        loop {
            let new_name = if ext.is_empty() {
                format!("{} ({})", stem, counter)
            } else {
                format!("{} ({}).{}", stem, counter, ext)
            };
            let check = extracted_dir.join(&new_name);
            if !check.exists() {
                dest = check;
                break;
            }
            counter += 1;
        }
    }

    rename_cross_drive_fallback(archive_path, &dest)
        .map_err(|e| format!("Failed to move archive to .extracted: {e}"))?;

    Ok(())
}

static DISKS_CACHE: std::sync::OnceLock<std::sync::Mutex<sysinfo::Disks>> =
    std::sync::OnceLock::new();

/// Check available disk space at the target directory.
fn check_disk_space(mods_dir: &Path, required_space: u64) -> Result<(), String> {
    let mutex = DISKS_CACHE
        .get_or_init(|| std::sync::Mutex::new(sysinfo::Disks::new_with_refreshed_list()));
    let mut disks = mutex.lock().unwrap();
    disks.refresh(true);

    let search_path = mods_dir
        .canonicalize()
        .unwrap_or_else(|_| mods_dir.to_path_buf());

    let mut available_space = 0u64;
    let mut matched_len = 0usize;
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

    Ok(())
}
