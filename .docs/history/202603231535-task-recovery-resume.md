### Title

Implement task recovery resume functionality

### Context

The application boot dialog for interrupted tasks only allowed clearing the tasks, missing the "Resume or Abort" functionality. Resuming Safe Mode switches was impossible because `switch_corridor` tasks were not saving the target mode.

### Changes

- Updated `switch_pipeline.rs` to persist the boolean targeting safe mode as a string payload (`target_id`).
- Added "Resume Task" button to `RecoveryDialog.tsx` which invokes `apply_collection` or `switch_corridor` dynamically explicitly depending on task type.

### Impacted Files

- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src/features/collections/components/RecoveryDialog.tsx` (modified)

### Goal

Empower users to resume incomplete Collection applies or Corridor switches after an unexpected crash without having to clear and guess their state.

### Impact

- Target safe mode is now properly stored inside pipeline task tables.
- Interrupted state flow ensures graceful boot restarts for all critical operations.
