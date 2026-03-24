# Storage Optimizer (Dedup) Ignore System & Build Stabilization

## Context

Migration of the duplicate scanner to a dedicated feature required a persistent "Ignore" (whitelist) system to prevent repeat matching of known-safe duplicates. Additionally, recent backend changes caused several regressions in the hotkey and watcher subsystems that blocked clean builds.

## Changes

### Storage Optimizer & Dedup

- **Persistent Whitelist**: Added `duplicate_whitelist` table and `DedupRepository` to manage ignored pairs.
- **Dedicated UI**: Migrated the scanner to `/storage-optimizer` with a professional, compact dashboard.
- **Ignore Workflow**: Implemented `IgnoredPairsModal` for viewing and recovering whitelisted pairs.
- **Smarter Logic**: Enhanced `scanner.rs` to automatically exclude comparison of sibling mods within the same `VariantContainer` or `ModPackRoot`.
- **Result UI**: Removed member truncation in `DuplicateTable.tsx` and added structured reason badges.

### Build & Stability

- **Hotkey System**: Restored missing `NextVariantFolder` and `PrevVariantFolder` actions in `hotkey_cmds.rs` and `HotkeyManager`.
- **Regression Fixes**: Resolved argument mismatches in test suites and `trigger_overlay_refresh` calls.
- **Dependency Cleanup**: Replaced missing `date-fns` and `heroicons` with native `Intl` and `lucide-react` to ensure clean Vite builds.

## Impacted Files

- `src-tauri/src/services/scanner/dedup/scanner.rs` (modified)
- `src-tauri/src/repo/dedup_repo.rs` (modified)
- `src-tauri/src/commands/duplicates/*` (added/modified)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified)
- `src-tauri/src/commands/app/hotkey_cmds.rs` (modified)
- `src/features/scanner/StorageOptimizerPage.tsx` (modified)
- `src/features/scanner/components/IgnoredPairsModal.tsx` (added)
- `src/features/scanner/components/DuplicateTable.tsx` (modified)

## Goal

A production-ready, stable Storage Optimizer with persistent ignore management and zero build regressions.

## Impact

- **Stability**: Build is clean and test regressions are resolved.
- **UX**: Professional-grade duplicate management with full recovery support.
- **disk**: Variant-aware scanning correctly identifies 0 false positives for containerized mods.
