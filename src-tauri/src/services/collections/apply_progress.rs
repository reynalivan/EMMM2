use super::types::{ApplyCollectionProgress, ApplyCollectionProgressPhase};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static APPLY_PROGRESS: LazyLock<Mutex<HashMap<String, ApplyCollectionProgress>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn with_progress_mut<F>(game_id: &str, update: F)
where
    F: FnOnce(&mut ApplyCollectionProgress),
{
    let mut guard = APPLY_PROGRESS
        .lock()
        .expect("apply progress mutex poisoned");
    let progress = guard
        .entry(game_id.to_string())
        .or_insert_with(ApplyCollectionProgress::idle);
    update(progress);
}

pub fn start_apply_progress(game_id: &str, collection_name: &str, is_safe: bool) {
    with_progress_mut(game_id, |progress| {
        *progress = ApplyCollectionProgress {
            phase: ApplyCollectionProgressPhase::Preparing,
            completed: 0,
            total: 0,
            current_item: None,
            collection_name: Some(collection_name.to_string()),
            is_safe: Some(is_safe),
            error: None,
        };
    });
}

pub fn add_apply_progress_total(game_id: &str, delta: usize) {
    if delta == 0 {
        return;
    }

    with_progress_mut(game_id, |progress| {
        progress.total += delta;
    });
}

pub fn set_apply_progress_phase(
    game_id: &str,
    phase: ApplyCollectionProgressPhase,
    current_item: Option<String>,
) {
    with_progress_mut(game_id, |progress| {
        progress.phase = phase;
        progress.current_item = current_item;
        progress.error = None;
    });
}

pub fn advance_apply_progress(game_id: &str, current_item: Option<String>) {
    with_progress_mut(game_id, |progress| {
        progress.phase = ApplyCollectionProgressPhase::Renaming;
        progress.completed += 1;
        progress.current_item = current_item;
    });
}

pub fn finish_apply_progress(game_id: &str) {
    with_progress_mut(game_id, |progress| {
        progress.phase = ApplyCollectionProgressPhase::Done;
        progress.completed = progress.total.max(progress.completed);
        progress.current_item = None;
        progress.error = None;
    });
}

pub fn fail_apply_progress(game_id: &str, error: &str) {
    with_progress_mut(game_id, |progress| {
        progress.phase = ApplyCollectionProgressPhase::Failed;
        progress.current_item = None;
        progress.error = Some(error.to_string());
    });
}

pub fn get_apply_progress(game_id: &str) -> ApplyCollectionProgress {
    APPLY_PROGRESS
        .lock()
        .expect("apply progress mutex poisoned")
        .get(game_id)
        .cloned()
        .unwrap_or_else(ApplyCollectionProgress::idle)
}
