# Collection & Privacy Systems — Backend Refactoring

## Context

Code review identified 10 improvement items across Collection and Privacy systems. This session implements the 7 backend items (P1, P3-P8).

## Changes

### P1: Unified Dual Signature System

- Removed incompatible Format B (`compute_signature_from_mods`, `classify_signature_match`, `extract_mod_ids_from_signature`, `SignatureMatchType`) (~190 LOC)
- Only canonical `serialize_signature` remains as single source of truth

### P3: Consolidated Duplicated Functions

- Moved `normalize_object_states` → `types.rs` (was in `storage.rs` + `runtime_snapshot.rs`)
- Moved `is_collection_db_mod_target` → `types.rs` (was in `apply.rs` + `effective_state.rs`)

### P4: Non-Blocking Consistency Check

- `run_fs_db_consistency_warning_phase` → fire-and-forget `tokio::spawn` in `apply.rs`

### P5: Fixed Mutex Panics

- Replaced `.expect("poisoned")` with `let Ok(guard) = ... else { log::error!; return }` in `apply_progress.rs`

### P6: Removed Double SuppressionGuard

- Removed outer guard from `execute_switch_phases_with_rollback` in `privacy/mod.rs`

### P7: Fixed Incomplete Rollback

- Added undo pointer cleanup in `handle_restore_failure_with_rollback`

### P8: Corridor State Update After Switch

- Added `upsert_corridor_state` call after successful mode switch

## Impacted Files

- `src-tauri/src/services/collections/runtime_snapshot.rs` (modified)
- `src-tauri/src/services/collections/types.rs` (modified)
- `src-tauri/src/services/collections/storage.rs` (modified)
- `src-tauri/src/services/collections/apply.rs` (modified)
- `src-tauri/src/services/collections/effective_state.rs` (modified)
- `src-tauri/src/services/collections/apply_progress.rs` (modified)
- `src-tauri/src/services/privacy/mod.rs` (modified)

## Goal

Cleaner, more maintainable backend with single source of truth for shared logic, eliminated dead code, crash-resistant progress tracking, and correct privacy mode rollback/state management.

## Impact

- ~190 LOC dead code removed from signature system
- ~60 LOC duplicated functions consolidated
- Apply pipeline no longer blocks on O(N) consistency check
- App no longer crashes if apply-phase thread panics
- Undo system correctly cleaned up on failed corridor switches
- Workspace context correctly updated after successful corridor switches

## Notes

- P2 (ApplyContext struct) and P9/P10 (frontend decomposition) deferred for future sessions
- `Serialize`/`Deserialize` serde import removed from `runtime_snapshot.rs` (was only used by removed `SignatureMatchType`)
- `HashMap` import removed from `storage.rs` (was only used by removed `normalize_object_states`)
