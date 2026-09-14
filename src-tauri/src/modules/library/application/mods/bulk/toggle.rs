//! Bulk enable/disable across many mod folders.

use super::types::{BulkActionError, BulkProgressPayload, BulkResult};
use crate::modules::collections::domain::collection::CollectionReferenceImpact;
use crate::modules::library::application::mods::core_ops::{
    plan_toggle_rename_with_sibling_index, SiblingNameIndex, ToggleRenamePlan,
};
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::modules::workspace::domain::workspace::WorkspacePathRewrite;
use crate::shared::errors::AppError;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone)]
enum PreparedToggleState {
    Ready {
        sequence: u32,
        plan: ToggleRenamePlan,
    },
    Noop,
    Invalid(AppError),
}

#[derive(Debug, Clone)]
struct PreparedToggleItem {
    input_path: String,
    state: PreparedToggleState,
}

#[derive(Debug, Clone)]
pub struct PreparedBulkToggle {
    items: Vec<PreparedToggleItem>,
    enable: bool,
}

impl PreparedBulkToggle {
    pub fn resequence(&mut self, start: u32) -> u32 {
        let mut sequence = start;
        for item in &mut self.items {
            if let PreparedToggleState::Ready {
                sequence: item_sequence,
                ..
            } = &mut item.state
            {
                *item_sequence = sequence;
                sequence += 1;
            }
        }
        sequence
    }

    pub fn planned_steps(&self) -> Vec<(u32, PathBuf, PathBuf)> {
        self.items
            .iter()
            .filter_map(|item| match &item.state {
                PreparedToggleState::Ready { sequence, plan } => Some((
                    *sequence,
                    plan.old_path().to_path_buf(),
                    plan.new_path().to_path_buf(),
                )),
                PreparedToggleState::Noop | PreparedToggleState::Invalid(_) => None,
            })
            .collect()
    }

    pub fn planned_sequences(&self) -> Vec<u32> {
        self.planned_steps()
            .into_iter()
            .map(|(sequence, _, _)| sequence)
            .collect()
    }

    /// Rebase a planned toggle after an earlier ancestor rename in the same
    /// transaction. Planning still validates the source while it exists; this
    /// only changes the spelling that will exist when the batch executes.
    pub fn rebase_paths(&mut self, rewrites: &[(PathBuf, PathBuf)]) {
        for item in &mut self.items {
            item.input_path = rebase_path(PathBuf::from(&item.input_path), rewrites)
                .to_string_lossy()
                .into_owned();
            if let PreparedToggleState::Ready { plan, .. } = &mut item.state {
                plan.rebase_paths(rewrites);
            }
        }
    }

    /// Workspace Switch uses single-item batches to sequence parent renames.
    /// Surface preparation failures early so later paths are never rebased on
    /// an ancestor rename that cannot actually happen.
    pub fn single_planned_step(&self) -> Result<Option<(PathBuf, PathBuf)>, AppError> {
        let Some(item) = self.items.first() else {
            return Ok(None);
        };
        match &item.state {
            PreparedToggleState::Ready { plan, .. } => Ok(Some((
                plan.old_path().to_path_buf(),
                plan.new_path().to_path_buf(),
            ))),
            PreparedToggleState::Noop => Ok(None),
            PreparedToggleState::Invalid(error) => Err(error.clone()),
        }
    }

    fn suppression_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::with_capacity(self.items.len() * 2);
        for item in &self.items {
            match &item.state {
                PreparedToggleState::Ready { plan, .. } => {
                    paths.push(plan.old_path().to_path_buf());
                    paths.push(plan.new_path().to_path_buf());
                }
                PreparedToggleState::Noop | PreparedToggleState::Invalid(_) => {
                    paths.push(PathBuf::from(&item.input_path));
                }
            }
        }
        paths
    }
}

fn rebase_path(mut path: PathBuf, rewrites: &[(PathBuf, PathBuf)]) -> PathBuf {
    for (old_path, new_path) in rewrites {
        if let Ok(suffix) = path.strip_prefix(old_path) {
            path = new_path.join(suffix);
        }
    }
    path
}

pub struct BulkToggleExecution {
    pub result: BulkResult,
    pub applied_sequences: Vec<u32>,
}

pub fn prepare_bulk_toggle(paths: &[PathBuf], enable: bool) -> PreparedBulkToggle {
    let sibling_indexes = sibling_indexes_for_paths(paths);
    let mut sequence = 0_u32;
    let items = paths
        .iter()
        .map(|path| {
            let input_path = path.to_string_lossy().into_owned();
            let sibling_index = path
                .parent()
                .and_then(|parent| sibling_indexes.get(parent))
                .and_then(Option::as_ref);
            let state = match plan_toggle_rename_with_sibling_index(path, enable, sibling_index) {
                Ok(Some(plan)) => {
                    let current_sequence = sequence;
                    sequence += 1;
                    PreparedToggleState::Ready {
                        sequence: current_sequence,
                        plan,
                    }
                }
                Ok(None) => PreparedToggleState::Noop,
                Err(error) => PreparedToggleState::Invalid(error),
            };
            PreparedToggleItem { input_path, state }
        })
        .collect();

    PreparedBulkToggle { items, enable }
}

fn sibling_indexes_for_paths(paths: &[PathBuf]) -> HashMap<PathBuf, Option<SiblingNameIndex>> {
    let mut indexes = HashMap::new();
    for path in paths {
        let Some(parent) = path.parent() else {
            continue;
        };
        indexes
            .entry(parent.to_path_buf())
            .or_insert_with(|| SiblingNameIndex::read(parent));
    }
    indexes
}

/// Bulk toggle mods on disk. The DB converges via the trailing scoped
/// reconcile — the single writer of status/path columns.
///
/// Paths in `paths` are absolute and already validated by the command layer.
pub async fn bulk_toggle(
    app: &AppHandle,
    state: &WatcherState,
    paths: Vec<String>,
    enable: bool,
    cancel: &AtomicBool,
) -> Result<BulkResult, crate::shared::errors::AppError> {
    let prepared_paths = paths.iter().map(PathBuf::from).collect::<Vec<_>>();
    let prepared = prepare_bulk_toggle(&prepared_paths, enable);
    Ok(execute_prepared_bulk_toggle(app, state, &prepared, cancel).result)
}

pub fn execute_prepared_bulk_toggle(
    app: &AppHandle,
    state: &WatcherState,
    prepared: &PreparedBulkToggle,
    cancel: &AtomicBool,
) -> BulkToggleExecution {
    // One path-scoped guard across the whole batch covers both the original
    // and destination spelling for every planned rename.
    let suppression_paths = prepared.suppression_paths();
    let _suppression = state.suppressor.suppress_paths(suppression_paths.iter());

    let total = prepared.items.len();
    let action_label = if prepared.enable {
        "common:bulk_progress.enabling"
    } else {
        "common:bulk_progress.disabling"
    };

    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: action_label.to_string(),
            current: 0,
            total,
            active: true,
        },
    );

    let mut success = Vec::new();
    let mut failures = Vec::new();
    let collection_impact = CollectionReferenceImpact::default();
    let mut path_rewrites = Vec::new();
    let mut applied_sequences = Vec::new();

    // Opt-O: Batch progress — emit every N items to reduce IPC overhead
    let progress_interval = std::cmp::max(1, total / 10);

    let mut cancelled = false;
    let mut processed_count = 0;
    for (i, item) in prepared.items.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }

        if i % progress_interval == 0 || i == total - 1 {
            let _ = app.emit(
                "bulk-progress",
                BulkProgressPayload {
                    label: action_label.to_string(),
                    current: i + 1,
                    total,
                    active: true,
                },
            );
        }

        match &item.state {
            PreparedToggleState::Ready { sequence, plan } => match plan.apply("mod folder") {
                Ok(()) => {
                    let new_abs_path = plan.new_path().to_string_lossy().into_owned();
                    success.push(new_abs_path.clone());
                    applied_sequences.push(*sequence);
                    path_rewrites.push(WorkspacePathRewrite {
                        old_path: item.input_path.clone(),
                        new_path: new_abs_path,
                    });
                }
                Err(error) => failures.push(BulkActionError {
                    path: item.input_path.clone(),
                    error,
                }),
            },
            PreparedToggleState::Noop => success.push(item.input_path.clone()),
            PreparedToggleState::Invalid(error) => failures.push(BulkActionError {
                path: item.input_path.clone(),
                error: error.clone(),
            }),
        }
        processed_count += 1;
    }

    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: if cancelled {
                "common:bulk_progress.cancelled"
            } else {
                "common:bulk_progress.done"
            }
            .to_string(),
            current: processed_count,
            total,
            active: false,
        },
    );

    BulkToggleExecution {
        result: BulkResult::with_collection_impact(
            success,
            failures,
            collection_impact,
            path_rewrites,
        )
        .with_execution_state(cancelled, processed_count, total),
        applied_sequences,
    }
}

pub fn rollback_prepared_bulk_toggle(
    state: &WatcherState,
    prepared: &PreparedBulkToggle,
    applied_sequences: &[u32],
) -> Result<(), AppError> {
    let applied = applied_sequences.iter().copied().collect::<HashSet<_>>();
    let suppression_paths = prepared.suppression_paths();
    let _suppression = state.suppressor.suppress_paths(suppression_paths.iter());
    let mut failures = Vec::new();

    for item in prepared.items.iter().rev() {
        let PreparedToggleState::Ready { sequence, plan } = &item.state else {
            continue;
        };
        if applied.contains(sequence) {
            if let Err(error) = plan.rollback("mod folder") {
                failures.push(format!("{}: {error}", item.input_path));
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Io(format!(
            "Failed to roll back bulk toggle: {}",
            failures.join("; ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::prepare_bulk_toggle;

    #[test]
    fn sibling_index_keeps_per_item_collisions_and_safe_siblings() {
        let temp = tempfile::tempdir().expect("temp directory");
        let blocked = temp.path().join("DISABLED Alpha");
        let existing_target = temp.path().join("Alpha");
        let safe = temp.path().join("DISABLED Beta");
        let already_enabled = temp.path().join("Gamma");
        std::fs::create_dir_all(&blocked).expect("blocked source");
        std::fs::create_dir_all(&existing_target).expect("collision target");
        std::fs::create_dir_all(&safe).expect("safe source");
        std::fs::create_dir_all(&already_enabled).expect("noop source");

        let prepared = prepare_bulk_toggle(&[blocked, safe, already_enabled], true);
        let steps = prepared.planned_steps();

        assert_eq!(steps.len(), 1);
        assert_eq!(
            steps[0].2.file_name().and_then(|name| name.to_str()),
            Some("Beta")
        );
    }
}
