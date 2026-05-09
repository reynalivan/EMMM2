# Remove Deprecated `undo_collection` Command & Specta Type Cleanup

## Context

`undo_collection` was deprecated in favor of explicit Collection Snapshot creation. The compiler emitted `#[warn(deprecated)]` on every `lib.rs` command registration. Secondary `BigIntForbidden` panics blocked `specta_tests` from generating valid TypeScript bindings.

## Changes

### Deprecated Command Removal

- Removed `undo_collection` fn from `commands/collections/cmds.rs`
- Removed `collection_service::undo_collection` stub from `services/collection_service.rs`
- Removed `undo_collection` from both Tauri invoke handler and `specta_tests` export list in `lib.rs`
- Removed `useUndoCollection` hook from `src/features/collections/hooks/useCollections.ts`

### Specta `BigIntForbidden` Fixes (type annotation pass)

Applied `#[specta(type = f64)]` to all `u64`/`usize`/`i64` fields in exported specta types:

- `services/app/dashboard.rs` — `duplicate_waste_bytes: i64`
- `commands/mods/conflict_cmds.rs` — `FolderDetail.total_size: u64`, `FolderDetail.file_count: usize`, `FileEntry.size: u64`
- `commands/mods/mod_core_cmds.rs` — `FolderContentInfo` usize fields (item_count, ini_count, image_count, nested_folder_count)
- `commands/mods/preview_cmds.rs` — `IniLineUpdate.line_idx: usize`
- `services/ini/document.rs` — `IniVariable.line_idx: usize`, `KeyBinding.key_line_idx/back_line_idx: Option<usize>`
- `services/hotkeys/mod.rs` — `HotkeyConfig.cooldown_ms: u64`
- `types/errors.rs` — Changed `ObjectHasMods` field from `i64` to `i32`
- `services/objects/mutate.rs` — Cast `count` to `i32` at `ObjectHasMods` instantiation

### Global BigInt Export Strategy

Added `.bigint(specta_typescript::BigIntExportBehavior::Number)` to the `specta_tests` export config in `lib.rs` as a global catch-all for remaining BigInt types not individually annotated.

### Bindings

Reverted `src/lib/bindings.ts` to the committed hand-crafted version (specta auto-generated format is incompatible with current frontend consumers — migration is a separate task). `undo_collection` entry was already absent from committed bindings.

## Impacted Files

**Backend (Rust):**
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/types/errors.rs` (modified)
- `src-tauri/src/services/objects/mutate.rs` (modified)
- `src-tauri/src/services/app/dashboard.rs` (modified)
- `src-tauri/src/commands/mods/conflict_cmds.rs` (modified)
- `src-tauri/src/commands/mods/mod_core_cmds.rs` (modified)
- `src-tauri/src/commands/mods/preview_cmds.rs` (modified)
- `src-tauri/src/services/ini/document.rs` (modified)
- `src-tauri/src/services/hotkeys/mod.rs` (modified)

**Frontend (TypeScript):**
- `src/features/collections/hooks/useCollections.ts` (modified — hook removed)

## Goal

Zero compiler warnings for deprecated commands. `cargo test specta_tests` passes. `pnpm tsc --noEmit` exits clean.

## Impact

- No breaking changes to runtime behavior — `undo_collection` was a stub returning `Err`
- Frontend callers of `useUndoCollection` must be verified (none found via grep)
- `specta_tests` now catches future `BigInt` type export errors globally via `BigIntExportBehavior::Number`
- Auto-generated `bindings.ts` migration to specta format is a separate follow-up task

## Notes

- `#[specta(type = f64)]` pattern chosen over type alias or newtype to minimize diff surface
- `i64 → i32` for `ObjectHasMods.count` is safe: mod counts in practice fit comfortably in i32
- `BigIntExportBehavior::Number` added only to the `specta_tests` export call; production `run()` builder does not export bindings
