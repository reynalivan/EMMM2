use super::types::{ArchiveAnalysis, ArchiveEntryInfo, ArchiveFormat};

const MAX_ENTRIES: usize = 500;
use std::fs;
use std::path::Path;

/// Analyze any supported archive without extracting.
pub fn analyze_archive(archive_path: &Path) -> Result<ArchiveAnalysis, String> {
    // AC-37.1.4: Block multi-volume archives early to prevent partial extractions
    let file_name = archive_path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if file_name.contains(".part")
        || file_name.contains(".z0")
        || file_name.contains(".r0")
        || file_name.ends_with(".001")
    {
        return Err("Multi-volume archives not supported".into());
    }

    let format = ArchiveFormat::from_path(archive_path)
        .ok_or_else(|| format!("Unsupported archive format: {}", archive_path.display()))?;

    match format {
        ArchiveFormat::Zip => analyze_zip(archive_path, format),
        ArchiveFormat::SevenZ => analyze_7z(archive_path, format),
        ArchiveFormat::Rar => analyze_rar(archive_path, format),
    }
}

fn analyze_zip(archive_path: &Path, format: ArchiveFormat) -> Result<ArchiveAnalysis, String> {
    let file = fs::File::open(archive_path).map_err(|e| format!("Failed to open archive: {e}"))?;
    let file_size_bytes = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP: {e}"))?;

    let mut has_ini = false;
    let mut is_encrypted = false;
    let mut uncompressed_size: u64 = 0;
    let mut root_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut contains_nested_archives = false;
    let mut entries: Vec<ArchiveEntryInfo> = Vec::new();

    for i in 0..archive.len() {
        match archive.by_index(i) {
            Ok(entry) => {
                let name = entry.name().to_string();
                let size = entry.size();
                let is_dir = entry.is_dir();
                uncompressed_size += size;

                if entries.len() < MAX_ENTRIES {
                    entries.push(ArchiveEntryInfo {
                        path: name.clone(),
                        is_dir,
                        size,
                    });
                }

                if name.to_lowercase().ends_with(".ini") {
                    has_ini = true;
                }

                let ext = name.split('.').last().unwrap_or("").to_lowercase();
                if ext == "zip" || ext == "rar" || ext == "7z" {
                    contains_nested_archives = true;
                }

                if let Some(first) = name.split('/').next() {
                    if !first.is_empty() {
                        root_dirs.insert(first.to_string());
                    }
                }
            }
            Err(e) => {
                let msg = e.to_string();
                // Encrypted entries can't be read without a password at analysis time.
                if msg.contains("Password") || msg.contains("password") || msg.contains("decrypt") {
                    is_encrypted = true;
                    continue;
                }
                return Err(format!("Failed to read entry: {e}"));
            }
        }
    }

    Ok(ArchiveAnalysis {
        format,
        file_count: archive.len(),
        has_ini,
        uncompressed_size,
        file_size_bytes,
        single_root_folder: if root_dirs.len() == 1 {
            root_dirs.into_iter().next()
        } else {
            None
        },
        is_encrypted,
        contains_nested_archives,
        entries,
    })
}

fn analyze_7z(archive_path: &Path, format: ArchiveFormat) -> Result<ArchiveAnalysis, String> {
    let mut has_ini = false;
    let mut file_count: usize = 0;
    let mut uncompressed_size: u64 = 0;
    let mut root_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut is_encrypted = false;
    let mut contains_nested_archives = false;
    let mut entries: Vec<ArchiveEntryInfo> = Vec::new();

    let file_size_bytes = fs::metadata(archive_path).map(|m| m.len()).unwrap_or(0);

    let result = sevenz_rust::decompress_with_extract_fn(
        fs::File::open(archive_path).map_err(|e| format!("Failed to open 7z: {e}"))?,
        ".",
        |entry, _, _| {
            let name = entry.name().to_string();
            let size = entry.size();
            let is_dir = entry.has_stream();
            file_count += 1;
            uncompressed_size += size;

            if entries.len() < MAX_ENTRIES {
                entries.push(ArchiveEntryInfo {
                    path: name.clone(),
                    is_dir: !is_dir, // has_stream = is a file, so invert
                    size,
                });
            }

            if name.to_lowercase().ends_with(".ini") {
                has_ini = true;
            }

            let ext = name.split('.').last().unwrap_or("").to_lowercase();
            if ext == "zip" || ext == "rar" || ext == "7z" {
                contains_nested_archives = true;
            }

            // Normalize path separators for root detection
            let normalized = name.replace('\\', "/");
            if let Some(first) = normalized.split('/').next() {
                if !first.is_empty() {
                    root_dirs.insert(first.to_string());
                }
            }
            Ok(true) // skip actual extraction
        },
    );

    if let Err(e) = result {
        let msg = e.to_string();
        if msg.contains("password") || msg.contains("Password") || msg.contains("decrypt") {
            is_encrypted = true;
            // Return partial analysis with encryption flag
        } else {
            return Err(format!("Failed to analyze 7z: {e}"));
        }
    }

    Ok(ArchiveAnalysis {
        format,
        file_count,
        has_ini,
        uncompressed_size,
        file_size_bytes,
        single_root_folder: if root_dirs.len() == 1 {
            root_dirs.into_iter().next()
        } else {
            None
        },
        is_encrypted,
        contains_nested_archives,
        entries,
    })
}

fn analyze_rar_cli(archive_path: &Path, format: ArchiveFormat) -> Result<ArchiveAnalysis, String> {
    use std::process::Command;
    // 7z l -slt <path>
    let output = Command::new("7z")
        .arg("l")
        .arg("-slt")
        .arg(archive_path)
        .output()
        .map_err(|e| format!("7z CLI not found or failed: {e}"))?;

    if !output.status.success() {
        return Err("7z CLI execution failed".into());
    }

    let file_size_bytes = fs::metadata(archive_path).map(|m| m.len()).unwrap_or(0);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut has_ini = false;
    let mut file_count = 0;
    let mut uncompressed_size = 0;
    let mut root_dirs = std::collections::HashSet::new();
    let mut is_encrypted = false;
    let mut contains_nested_archives = false;
    let mut entries: Vec<ArchiveEntryInfo> = Vec::new();

    let mut pending_name: Option<String> = None;
    let mut pending_size: u64 = 0;
    let mut pending_is_dir = false;

    for line in stdout.lines() {
        if line.starts_with("Encrypted = +") {
            is_encrypted = true;
        } else if line.starts_with("Path = ") {
            // Flush previous entry
            if let Some(prev_name) = pending_name.take() {
                if entries.len() < MAX_ENTRIES {
                    entries.push(ArchiveEntryInfo {
                        path: prev_name,
                        is_dir: pending_is_dir,
                        size: pending_size,
                    });
                }
            }
            pending_size = 0;
            pending_is_dir = false;

            let name = &line[7..].trim();
            if !name.is_empty() && *name != archive_path.to_string_lossy().as_ref() {
                file_count += 1;
                pending_name = Some(name.to_string());
                if name.to_lowercase().ends_with(".ini") {
                    has_ini = true;
                }
                let ext = name.split('.').last().unwrap_or("").to_lowercase();
                if ext == "zip" || ext == "rar" || ext == "7z" {
                    contains_nested_archives = true;
                }

                let normalized = name.replace('\\', "/");
                if let Some(first) = normalized.split('/').next() {
                    if !first.is_empty() {
                        root_dirs.insert(first.to_string());
                    }
                }
            }
        } else if line.starts_with("Size = ") {
            if let Ok(size) = line[7..].trim().parse::<u64>() {
                uncompressed_size += size;
                pending_size = size;
            }
        } else if line.starts_with("Folder = +") {
            pending_is_dir = true;
        }
    }
    // Flush last entry
    if let Some(prev_name) = pending_name.take() {
        if entries.len() < MAX_ENTRIES {
            entries.push(ArchiveEntryInfo {
                path: prev_name,
                is_dir: pending_is_dir,
                size: pending_size,
            });
        }
    }

    Ok(ArchiveAnalysis {
        format,
        file_count,
        has_ini,
        uncompressed_size,
        file_size_bytes,
        single_root_folder: if root_dirs.len() == 1 {
            root_dirs.into_iter().next()
        } else {
            None
        },
        is_encrypted,
        contains_nested_archives,
        entries,
    })
}

fn analyze_rar(archive_path: &Path, format: ArchiveFormat) -> Result<ArchiveAnalysis, String> {
    // 1. FAST PATH: Use 7z CLI if available on system (Sub-millisecond header read, 0 bytes extracted)
    if let Ok(fast_analysis) = analyze_rar_cli(archive_path, format) {
        return Ok(fast_analysis);
    }

    // 2. SLOW FALLBACK: Use `rar` crate (Forces full temp extraction to read contents)
    let path_str = archive_path
        .to_str()
        .ok_or("RAR path contains invalid UTF-8")?;

    // First try without password to detect encryption
    let temp_dir = tempfile::tempdir()
        .map_err(|e| format!("Failed to create temp dir for RAR analysis: {e}"))?;
    let temp_str = temp_dir
        .path()
        .to_str()
        .ok_or("Temp path contains invalid UTF-8")?;

    let file_size_bytes = fs::metadata(archive_path).map(|m| m.len()).unwrap_or(0);

    let mut is_encrypted = false;

    let archive_result = rar::Archive::extract_all(path_str, temp_str, "");
    let archive = match archive_result {
        Ok(a) => a,
        Err(e) => {
            let msg = format!("{e:?}");
            if msg.contains("password") || msg.contains("Password") || msg.contains("encrypted") {
                is_encrypted = true;
                // Return partial analysis with encryption flag
                return Ok(ArchiveAnalysis {
                    format,
                    file_count: 0,
                    has_ini: false,
                    uncompressed_size: 0,
                    file_size_bytes,
                    single_root_folder: None,
                    is_encrypted,
                    contains_nested_archives: false,
                    entries: Vec::new(),
                });
            }
            return Err(format!("Failed to parse RAR: {e:?}"));
        }
    };

    let mut has_ini = false;
    let mut file_count: usize = 0;
    let mut uncompressed_size: u64 = 0;
    let mut root_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut contains_nested_archives = false;
    let mut entries: Vec<ArchiveEntryInfo> = Vec::new();

    for entry in &archive.files {
        let name = entry.name.to_string();
        file_count += 1;
        uncompressed_size += entry.unpacked_size;

        if entries.len() < MAX_ENTRIES {
            entries.push(ArchiveEntryInfo {
                path: name.clone(),
                is_dir: name.ends_with('/') || name.ends_with('\\'),
                size: entry.unpacked_size,
            });
        }

        if name.to_lowercase().ends_with(".ini") {
            has_ini = true;
        }

        let ext = name.split('.').last().unwrap_or("").to_lowercase();
        if ext == "zip" || ext == "rar" || ext == "7z" {
            contains_nested_archives = true;
        }

        let normalized = name.replace('\\', "/");
        if let Some(first) = normalized.split('/').next() {
            if !first.is_empty() {
                root_dirs.insert(first.to_string());
            }
        }
    }

    Ok(ArchiveAnalysis {
        format,
        file_count,
        has_ini,
        uncompressed_size,
        file_size_bytes,
        single_root_folder: if root_dirs.len() == 1 {
            root_dirs.into_iter().next()
        } else {
            None
        },
        is_encrypted,
        contains_nested_archives,
        entries,
    })
}
