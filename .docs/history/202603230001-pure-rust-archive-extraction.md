# Pure Rust Archive Extraction Refactor (Task 15)

## Context

Replace fragile C-dependent archive crates with pure Rust alternatives and improve extraction reliability with RAII cleanup and throttled IPC progress.

## Changes

- `ArchiveFormat::from_path` → `ArchiveFormat::detect` using magic bytes (ZIP/7z/RAR4/RAR5) with extension fallback
- Added `TempDirGuard` RAII struct replacing 5 manual `fs::remove_dir_all` + `cleanup_temp_extract_parent` call sites
- Added `emit_throttled_progress()` helper with 250ms interval to prevent IPC flooding on 50k+ file archives
- Confirmed `rar` crate v0.4 is pure Rust (nom + native crypto), no C deps
- Deleted orphan `temp_rar_check.rs`

## Impacted Files

- `src-tauri/src/services/mods/archive/types.rs` (modified)
- `src-tauri/src/services/mods/archive/extract.rs` (modified)
- `src-tauri/src/services/mods/archive/analyze.rs` (modified)
- `src-tauri/src/services/mods/archive/tests/mod_tests.rs` (modified)
- `src-tauri/src/commands/mods/mod_import_cmds.rs` (modified)
- `src-tauri/src/temp_rar_check.rs` (removed)

## Goal

Reliable, cross-platform archive extraction with automatic cleanup on failure/panic and smooth frontend progress bars.

## Impact

- No breaking API changes (same `ExtractionEvent` shape)
- Frontend progress bar will update ~4x/sec instead of per-file (smoother UX)
- Temp directories now guaranteed cleaned up even on panic paths
- RAR support retained via pure Rust crate
