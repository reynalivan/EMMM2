# Storage Optimizer session and report validation

## Context

Cancellation acknowledgement prematurely cleared UI state, delayed callbacks could affect a later scan, and the resolver could load a group from a previous Mods root.

## Changes

- Wait for a terminal scan event before clearing running state; disable Stop while cancellation IPC is pending.
- Bind callbacks to a local session generation and backend scan ID; reject overlapping starts, foreign events, and events after completion.
- Share report-root validation between report display and group authorization, including legacy reports without a root.
- Keep the validated root as a PathBuf until report serialization; remove the string-conversion helper.
- Give parsed INI signals named fields instead of a three-set tuple.

## Impacted Files

- `src/features/scanner/StorageOptimizerPage.tsx` and `StorageOptimizerPage.test.tsx`
- `src/features/scanner/stores/useDedupScanStore.ts` and `useDedupScanStore.test.ts`
- `src-tauri/src/modules/duplicates/adapters/sqlite/dedup/mod.rs` and `tests.rs`
- `src-tauri/src/modules/duplicates/adapters/tauri/tauri.rs`
- `src-tauri/src/modules/duplicates/application/dedup/tests/dedup_resolver_tests.rs`
- `src-tauri/src/modules/duplicates/application/dedup/snapshot.rs` and `tests/dedup_snapshot_tests.rs`
- This history file (added).

## Goal and Impact

Scan state stays owned by its session and stale reports cannot authorize resolution. No additional migration, dependency, or IPC payload change. Cancellation remains cooperative at existing backend checkpoints.

## Validation

- Frontend: 26 targeted tests, production build, and affected-file ESLint passed.
- Backend: 49 duplicate-module tests passed, 1 manual benchmark ignored, using SQLx offline metadata.
- Strict Rust Clippy (`-D warnings`), formatting, and diff whitespace checks passed.
- Regressions were reproduced before fixes for premature cancellation state, stale events, and stale group authorization.
