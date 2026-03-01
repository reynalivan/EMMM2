//! Application-level log access service.

use std::io::BufRead;
use std::path::Path;

/// Read the last `n` lines from a log file at `log_path`.
/// Returns a single-element vec with an error message if the file does not exist.
pub fn read_last_n_lines(log_path: &Path, n: usize) -> Result<Vec<String>, String> {
    if !log_path.exists() {
        return Ok(vec!["Log file not found.".to_string()]);
    }

    let file = std::fs::File::open(log_path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);

    let all_lines: Result<Vec<String>, _> = reader.lines().collect();
    let all_lines = all_lines.map_err(|e| e.to_string())?;

    let count = all_lines.len();
    let skip = count.saturating_sub(n);

    Ok(all_lines.into_iter().skip(skip).collect())
}

/// Open the logs directory in the OS file explorer.
pub fn open_log_folder_service(log_dir: &Path) -> Result<(), String> {
    if !log_dir.exists() {
        std::fs::create_dir_all(log_dir).map_err(|e| e.to_string())?;
    }

    std::process::Command::new("explorer")
        .arg(log_dir)
        .spawn()
        .map_err(|e| format!("Failed to open log folder: {}", e))?;

    Ok(())
}
