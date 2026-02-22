use super::types::{ArchiveAnalysis, ArchiveFormat};
use std::fs;
use std::path::Path;

/// Analyze any supported archive without extracting.
pub fn analyze_archive(archive_path: &Path) -> Result<ArchiveAnalysis, String> {
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
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP: {e}"))?;

    let mut has_ini = false;
    let mut uncompressed_size: u64 = 0;
    let mut root_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();

    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read entry: {e}"))?;
        let name = entry.name().to_string();
        uncompressed_size += entry.size();

        if name.to_lowercase().ends_with(".ini") {
            has_ini = true;
        }
        if let Some(first) = name.split('/').next() {
            if !first.is_empty() {
                root_dirs.insert(first.to_string());
            }
        }
    }

    Ok(ArchiveAnalysis {
        format,
        file_count: archive.len(),
        has_ini,
        uncompressed_size,
        single_root_folder: if root_dirs.len() == 1 {
            root_dirs.into_iter().next()
        } else {
            None
        },
    })
}

fn analyze_7z(archive_path: &Path, format: ArchiveFormat) -> Result<ArchiveAnalysis, String> {
    let mut has_ini = false;
    let mut file_count: usize = 0;
    let mut uncompressed_size: u64 = 0;
    let mut root_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();

    sevenz_rust::decompress_with_extract_fn(
        fs::File::open(archive_path).map_err(|e| format!("Failed to open 7z: {e}"))?,
        ".",
        |entry, _, _| {
            let name = entry.name().to_string();
            file_count += 1;
            uncompressed_size += entry.size();

            if name.to_lowercase().ends_with(".ini") {
                has_ini = true;
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
    )
    .map_err(|e| format!("Failed to analyze 7z: {e}"))?;

    Ok(ArchiveAnalysis {
        format,
        file_count,
        has_ini,
        uncompressed_size,
        single_root_folder: if root_dirs.len() == 1 {
            root_dirs.into_iter().next()
        } else {
            None
        },
    })
}

fn analyze_rar(archive_path: &Path, format: ArchiveFormat) -> Result<ArchiveAnalysis, String> {
    let path_str = archive_path
        .to_str()
        .ok_or("RAR path contains invalid UTF-8")?;

    // Extract to temp dir to read structure
    // note: rar crate limitations often force extract_all for metadata
    let temp_dir = tempfile::tempdir()
        .map_err(|e| format!("Failed to create temp dir for RAR analysis: {e}"))?;
    let temp_str = temp_dir
        .path()
        .to_str()
        .ok_or("Temp path contains invalid UTF-8")?;

    let archive = rar::Archive::extract_all(path_str, temp_str, "")
        .map_err(|e| format!("Failed to parse RAR: {e:?}"))?;

    let mut has_ini = false;
    let mut file_count: usize = 0;
    let mut uncompressed_size: u64 = 0;
    let mut root_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();

    for entry in &archive.files {
        let name = entry.name.to_string();
        file_count += 1;
        uncompressed_size += entry.unpacked_size;

        if name.to_lowercase().ends_with(".ini") {
            has_ini = true;
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
        single_root_folder: if root_dirs.len() == 1 {
            root_dirs.into_iter().next()
        } else {
            None
        },
    })
}
