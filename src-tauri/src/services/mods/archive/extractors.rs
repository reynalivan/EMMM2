use super::progress::emit_throttled_progress;
use super::types::{ArchiveFormat, ExtractionEvent};
use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::ipc::Channel;

pub(super) fn extract_to_dir(
    archive_path: &Path,
    dest_path: &Path,
    password: Option<&str>,
    format: ArchiveFormat,
    cancel_token: Option<Arc<AtomicBool>>,
    on_progress: Option<&Channel<ExtractionEvent>>,
) -> Result<usize, String> {
    match format {
        ArchiveFormat::Zip => {
            extract_zip_inner(archive_path, dest_path, password, cancel_token, on_progress)
        }
        ArchiveFormat::SevenZ => {
            extract_7z_inner(archive_path, dest_path, password, cancel_token, on_progress)
        }
        ArchiveFormat::Rar => {
            extract_rar_inner(archive_path, dest_path, password, cancel_token, on_progress)
        }
    }
}

pub(super) fn unpack_nested_archives(
    dir: &Path,
    current_depth: usize,
    max_depth: usize,
    cancel_token: &Option<Arc<AtomicBool>>,
) -> usize {
    if current_depth >= max_depth {
        log::warn!(
            "Max nested extraction depth ({}) reached in {:?}",
            max_depth,
            dir
        );
        return 0;
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(|entry| entry.ok()).collect::<Vec<_>>(),
        Err(_) => return 0,
    };
    let mut total_extracted = 0;

    for entry in entries {
        if is_cancelled(cancel_token) {
            return total_extracted;
        }

        let path = entry.path();
        if path.is_dir() {
            total_extracted +=
                unpack_nested_archives(&path, current_depth, max_depth, cancel_token);
            continue;
        }

        let Some(format) = ArchiveFormat::detect(&path) else {
            continue;
        };

        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let sub_dest = dir.join(stem);
        if sub_dest.exists() {
            continue;
        }
        if let Err(error) = fs::create_dir_all(&sub_dest) {
            log::warn!("Failed to create subfolder for nested archive: {error}");
            continue;
        }

        match extract_to_dir(&path, &sub_dest, None, format, cancel_token.clone(), None) {
            Ok(extracted) => {
                total_extracted += extracted;
                let _ = fs::remove_file(&path);
                total_extracted +=
                    unpack_nested_archives(&sub_dest, current_depth + 1, max_depth, cancel_token);
            }
            Err(error) => {
                log::warn!("Failed to extract nested archive {:?}: {}", path, error);
            }
        }
    }

    total_extracted
}

fn is_cancelled(cancel_token: &Option<Arc<AtomicBool>>) -> bool {
    cancel_token
        .as_ref()
        .map(|token| token.load(Ordering::SeqCst))
        .unwrap_or(false)
}

fn extract_zip_inner(
    archive_path: &Path,
    dest_path: &Path,
    password: Option<&str>,
    cancel_token: Option<Arc<AtomicBool>>,
    on_progress: Option<&Channel<ExtractionEvent>>,
) -> Result<usize, String> {
    let file =
        fs::File::open(archive_path).map_err(|error| format!("Failed to open archive: {error}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("Invalid or corrupt ZIP: {error}"))?;
    let total_entries = archive.len();
    let mut count = 0_usize;
    let mut last_progress = Instant::now();

    for i in 0..archive.len() {
        if cancel_token
            .as_ref()
            .map(|token| token.load(Ordering::SeqCst))
            .unwrap_or(false)
        {
            return Err("ABORTED".into());
        }

        let mut entry = match password {
            Some(value) => archive
                .by_index_decrypt(i, value.as_bytes())
                .map_err(|error| password_or_read_error(error, "decrypt", i))?,
            None => archive
                .by_index(i)
                .map_err(|error| password_or_read_error(error, "read", i))?,
        };

        let Some(entry_path) = entry.enclosed_name().map(|value| value.to_path_buf()) else {
            continue;
        };
        let output_path = dest_path.join(&entry_path);

        if entry.is_dir() {
            fs::create_dir_all(&output_path)
                .map_err(|error| format!("Failed to create dir: {error}"))?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create parent: {error}"))?;
        }

        let mut outfile = fs::File::create(&output_path)
            .map_err(|error| format!("Failed to create file: {error}"))?;
        io::copy(&mut entry, &mut outfile)
            .map_err(|error| format!("Failed to write file: {error}"))?;
        count += 1;

        if let Some(channel) = on_progress {
            let name = entry_path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_default();
            emit_throttled_progress(channel, &mut last_progress, name, count, total_entries);
        }
    }

    Ok(count)
}

fn password_or_read_error(error: zip::result::ZipError, action: &str, index: usize) -> String {
    let message = error.to_string();
    if message.contains("Password") || message.contains("password") {
        return "Password required to extract this archive".to_string();
    }
    format!("Failed to {action} entry {index}: {error}")
}

fn extract_7z_inner(
    archive_path: &Path,
    dest_path: &Path,
    password: Option<&str>,
    cancel_token: Option<Arc<AtomicBool>>,
    on_progress: Option<&Channel<ExtractionEvent>>,
) -> Result<usize, String> {
    if cancel_token
        .as_ref()
        .map(|token| token.load(Ordering::SeqCst))
        .unwrap_or(false)
    {
        return Err("ABORTED".into());
    }

    let file_counter = Arc::new(AtomicUsize::new(0));
    let mut last_progress = Instant::now();
    let extract_result = match password {
        Some(value) => {
            let file = fs::File::open(archive_path)
                .map_err(|error| format!("Failed to open archive: {error}"))?;
            let counter = file_counter.clone();
            sevenz_rust::decompress_with_extract_fn_and_password(
                file,
                dest_path,
                value.into(),
                |entry, reader, dest| {
                    extract_7z_entry(
                        entry,
                        reader,
                        dest,
                        &cancel_token,
                        &counter,
                        on_progress,
                        &mut last_progress,
                    )
                },
            )
        }
        None => {
            let counter = file_counter.clone();
            sevenz_rust::decompress_file_with_extract_fn(
                archive_path,
                dest_path,
                |entry, reader, dest| {
                    extract_7z_entry(
                        entry,
                        reader,
                        dest,
                        &cancel_token,
                        &counter,
                        on_progress,
                        &mut last_progress,
                    )
                },
            )
        }
    };

    extract_result.map_err(|error| extraction_error_7z(error.to_string()))?;

    let count = walkdir::WalkDir::new(dest_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .count();

    Ok(count)
}

fn extract_7z_entry(
    entry: &sevenz_rust::SevenZArchiveEntry,
    reader: &mut dyn std::io::Read,
    dest: &std::path::PathBuf,
    cancel_token: &Option<Arc<AtomicBool>>,
    counter: &Arc<AtomicUsize>,
    on_progress: Option<&Channel<ExtractionEvent>>,
    last_progress: &mut Instant,
) -> Result<bool, sevenz_rust::Error> {
    if is_cancelled(cancel_token) {
        return Err(sevenz_rust::Error::io(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "ABORTED",
        )));
    }

    let idx = counter.fetch_add(1, Ordering::Relaxed) + 1;
    if let Some(channel) = on_progress {
        emit_throttled_progress(channel, last_progress, entry.name().to_string(), idx, 0);
    }

    sevenz_rust::default_entry_extract_fn(entry, reader, dest)
}

fn extraction_error_7z(message: String) -> String {
    if message.contains("password") || message.contains("Password") || message.contains("decrypt") {
        return "Password required to extract this archive".to_string();
    }
    if message.contains("ABORTED") {
        return "ABORTED".to_string();
    }
    format!("Failed to extract 7z: {message}")
}

fn extract_rar_inner(
    archive_path: &Path,
    dest_path: &Path,
    password: Option<&str>,
    cancel_token: Option<Arc<AtomicBool>>,
    on_progress: Option<&Channel<ExtractionEvent>>,
) -> Result<usize, String> {
    if is_cancelled(&cancel_token) {
        return Err("ABORTED".into());
    }

    let path_str = archive_path
        .to_str()
        .ok_or("RAR path contains invalid UTF-8")?;
    let dest_str = dest_path
        .to_str()
        .ok_or("Dest path contains invalid UTF-8")?;
    let pw = password.unwrap_or("");
    rar::Archive::extract_all(path_str, dest_str, pw)
        .map_err(|error| format!("Failed to extract RAR: {error:?}"))?;

    let count = walkdir::WalkDir::new(dest_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .count();

    if let Some(channel) = on_progress {
        let _ = channel.send(ExtractionEvent::FileProgress {
            file_name: String::new(),
            file_index: count,
            total_files: count,
        });
    }

    Ok(count)
}
