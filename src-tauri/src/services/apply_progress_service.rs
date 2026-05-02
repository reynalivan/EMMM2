use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::domain::collection::ApplyProgressSnapshot;

fn progress_store() -> &'static Mutex<HashMap<String, ApplyProgressSnapshot>> {
    static STORE: OnceLock<Mutex<HashMap<String, ApplyProgressSnapshot>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn progress_key(game_id: &str, is_safe: bool) -> String {
    format!("{game_id}:{is_safe}")
}

pub fn start(game_id: &str, is_safe: bool) {
    let snapshot = ApplyProgressSnapshot {
        game_id: game_id.to_string(),
        is_safe,
        phase: "preparing".to_string(),
        completed: 0,
        total: 0,
        current_item: None,
        warnings: Vec::new(),
        final_state_name: None,
        final_mode: None,
        success: false,
    };
    if let Ok(mut store) = progress_store().lock() {
        store.insert(progress_key(game_id, is_safe), snapshot);
    }
}

pub fn update(
    game_id: &str,
    is_safe: bool,
    phase: &str,
    completed: usize,
    total: usize,
    current_item: Option<String>,
) {
    if let Ok(mut store) = progress_store().lock() {
        let entry = store
            .entry(progress_key(game_id, is_safe))
            .or_insert_with(|| ApplyProgressSnapshot {
                game_id: game_id.to_string(),
                is_safe,
                phase: phase.to_string(),
                completed,
                total,
                current_item: current_item.clone(),
                warnings: Vec::new(),
                final_state_name: None,
                final_mode: None,
                success: false,
            });
        entry.phase = phase.to_string();
        entry.completed = completed;
        entry.total = total;
        entry.current_item = current_item;
    }
}

pub fn set_warnings(game_id: &str, is_safe: bool, warnings: Vec<String>) {
    if let Ok(mut store) = progress_store().lock() {
        let entry = store
            .entry(progress_key(game_id, is_safe))
            .or_insert_with(|| ApplyProgressSnapshot {
                game_id: game_id.to_string(),
                is_safe,
                phase: "preparing".to_string(),
                completed: 0,
                total: 0,
                current_item: None,
                warnings: Vec::new(),
                final_state_name: None,
                final_mode: None,
                success: false,
            });
        entry.warnings = warnings;
    }
}

pub fn finish(
    game_id: &str,
    is_safe: bool,
    final_state_name: Option<String>,
    final_mode: Option<String>,
    warnings: Vec<String>,
    success: bool,
) {
    if let Ok(mut store) = progress_store().lock() {
        let entry = store
            .entry(progress_key(game_id, is_safe))
            .or_insert_with(|| ApplyProgressSnapshot {
                game_id: game_id.to_string(),
                is_safe,
                phase: "done".to_string(),
                completed: 0,
                total: 0,
                current_item: None,
                warnings: warnings.clone(),
                final_state_name: final_state_name.clone(),
                final_mode: final_mode.clone(),
                success,
            });
        entry.phase = if success {
            "done".to_string()
        } else {
            "failed".to_string()
        };
        entry.current_item = None;
        entry.final_state_name = final_state_name;
        entry.final_mode = final_mode;
        entry.warnings = warnings;
        entry.success = success;
        entry.completed = entry.total.max(entry.completed);
    }
}

pub fn get(game_id: &str, is_safe: bool) -> Option<ApplyProgressSnapshot> {
    progress_store()
        .lock()
        .ok()
        .and_then(|store| store.get(&progress_key(game_id, is_safe)).cloned())
}

pub fn clear(game_id: &str, is_safe: bool) {
    if let Ok(mut store) = progress_store().lock() {
        store.remove(&progress_key(game_id, is_safe));
    }
}
