# Repairing Classification Regressions & Test Suite Sync

## Context
After implementing path-based disabled inheritance and the new `warnings` array for folder classification, several regressions occurred in the Rust backend (scope issues) and the TypeScript frontend (mock data mismatches in tests).

## Changes
- **Backend (Rust)**:
    - Restored `reasons` and `referenced_subs` variable declarations in `classifier.rs`.
    - Updated `classify_folder` call sites in `walker.rs`, `conflict/mod.rs`, and `dedup/scanner.rs` to handle 3-tuple return type (NodeType, Reasons, Warnings).
    - Removed unused `find_disabled_ancestor` shim in `commands/folder_grid/listing.rs`.
- **Frontend (TypeScript)**:
    - Synchronized all `ModFolder` mock data in `Dashboard.test.tsx`, `FolderCard.test.tsx`, `FolderGrid.test.tsx`, and `previewTargetResolver.test.ts` to include the required `warnings: []` field.

## Impacted Files
- `src-tauri/src/services/explorer/classifier.rs` (modified)
- `src-tauri/src/services/scanner/core/walker.rs` (modified)
- `src-tauri/src/services/scanner/conflict/mod.rs` (modified)
- `src-tauri/src/services/scanner/dedup/scanner.rs` (modified)
- `src-tauri/src/commands/folder_grid/listing.rs` (modified)
- `src/features/dashboard/Dashboard.test.tsx` (modified)
- `src/features/folder-grid/FolderCard.test.tsx` (modified)
- `src/features/folder-grid/FolderGrid.test.tsx` (modified)
- `src/features/preview/previewTargetResolver.test.ts` (modified)

## Goal
Restore system-wide compilation integrity and ensure test suite parity with the new schema.

## Impact
- System now compiles successfully on both Rust and TypeScript sides.
- Test suites are passing and reflect the latest data structures.
- No changes to runtime behavior beyond the intended feature functionality.
