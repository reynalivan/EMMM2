# Collection runtime descriptor

## Context

Global collection status reads were carrying full runtime members and preview trees.

## Changes

- Added a compact descriptor service that reads all status inputs in one SQLite read transaction.
- Added transaction-scoped live-state and collection lookup helpers.
- Kept the legacy full snapshot and IPC unchanged for the frontend migration.

## Impacted Files

- `src-tauri/src/domain/runtime_state.rs`
- `src-tauri/src/repo/collection_repo/crud.rs`
- `src-tauri/src/repo/collection_repo/live.rs`
- `src-tauri/src/services/collection_runtime_service.rs`
- `src-tauri/src/services/collection_service/live_state.rs`
- `src-tauri/src/services/tests/collection_runtime_service_tests.rs`

## Goal

Topbar/global consumers can move to a bounded runtime descriptor while previews remain lazy.

## Impact

No IPC or frontend contract changed in this backend slice. The frontend must register and consume the descriptor in a follow-up.
