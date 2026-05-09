use super::types::{ExtractionEvent, ExtractionResult};
use std::time::Instant;
use tauri::ipc::Channel;

const PROGRESS_THROTTLE_MS: u128 = 250;

#[inline]
pub(super) fn emit_throttled_progress(
    channel: &Channel<ExtractionEvent>,
    last_emit: &mut Instant,
    file_name: String,
    file_index: usize,
    total_files: usize,
) {
    let is_final = total_files > 0 && file_index >= total_files;
    if is_final || last_emit.elapsed().as_millis() >= PROGRESS_THROTTLE_MS {
        let _ = channel.send(ExtractionEvent::FileProgress {
            file_name,
            file_index,
            total_files,
        });
        *last_emit = Instant::now();
    }
}

pub(super) fn aborted_result(archive_name: String, files_extracted: usize) -> ExtractionResult {
    ExtractionResult {
        archive_name,
        dest_paths: Vec::new(),
        files_extracted,
        mod_count: 0,
        success: false,
        error: None,
        aborted: true,
        collisions: Vec::new(),
    }
}
