use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::modules::collections::domain::collection::ApplyProgressSnapshot;

fn progress_store() -> &'static Mutex<HashMap<String, ApplyProgressSnapshot>> {
    static STORE: OnceLock<Mutex<HashMap<String, ApplyProgressSnapshot>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn new_snapshot(game_id: &str) -> ApplyProgressSnapshot {
    ApplyProgressSnapshot {
        game_id: game_id.to_string(),
        phase: "preparing".to_string(),
        ..Default::default()
    }
}

pub fn start(game_id: &str) {
    if let Ok(mut store) = progress_store().lock() {
        store.insert(game_id.to_string(), new_snapshot(game_id));
    }
}

/// Mutate the snapshot for one game, creating it if absent.
/// A poisoned lock is treated as "no progress to report" rather than a panic.
fn with_entry(game_id: &str, mutate: impl FnOnce(&mut ApplyProgressSnapshot)) {
    let Ok(mut store) = progress_store().lock() else {
        return;
    };
    let entry = store
        .entry(game_id.to_string())
        .or_insert_with(|| new_snapshot(game_id));
    mutate(entry);
}

pub fn update(
    game_id: &str,
    phase: &str,
    completed: usize,
    total: usize,
    current_item: Option<String>,
) {
    with_entry(game_id, |entry| {
        entry.phase = phase.to_string();
        entry.completed = completed;
        entry.total = total;
        entry.current_item = current_item;
    });
}

pub fn set_warnings(game_id: &str, warnings: Vec<String>) {
    with_entry(game_id, |entry| entry.warnings = warnings);
}

pub fn finish(
    game_id: &str,
    final_state_name: Option<String>,
    warnings: Vec<String>,
    success: bool,
) {
    if let Ok(mut store) = progress_store().lock() {
        let entry = store
            .entry(game_id.to_string())
            .or_insert_with(|| new_snapshot(game_id));
        entry.phase = if success {
            "done".to_string()
        } else {
            "failed".to_string()
        };
        entry.current_item = None;
        entry.final_state_name = final_state_name;
        entry.warnings = warnings;
        entry.success = success;
        entry.completed = entry.total.max(entry.completed);
    }
}

pub fn get(game_id: &str) -> Option<ApplyProgressSnapshot> {
    progress_store()
        .lock()
        .ok()
        .and_then(|store| store.get(game_id).cloned())
}
