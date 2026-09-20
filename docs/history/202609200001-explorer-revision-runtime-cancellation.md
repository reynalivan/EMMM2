# Revision-safe explorer and cancellable runtime composition

## Context

Cursor pagination and symbolic “all matching” selection reduced frontend load for very large mod folders, but a bulk action could still be resolved against a newer filesystem view than the one the user selected. Runtime coalescing also prevented stale publication while allowing an obsolete generation to spend unnecessary time harvesting or aggregating a large KeyViewer composition.

## Changes

- Bound explorer pages and symbolic bulk selection to one opaque, immutable listing revision held by the bounded backend snapshot cache.
- Added the typed `ExplorerSnapshotExpired` conflict for evicted, malformed, mismatched, or stale cursors/revisions.
- Reset only the affected infinite explorer query after pagination or bulk snapshot expiry; revision changes clear both explicit and all-matching selection before another bulk mutation.
- Resolved bulk selections from the stored snapshot instead of rescanning the directory or rebuilding a global database index.
- Added listing snapshot candidate and estimated-byte counters, a hard 500,000-candidate per-snapshot safety bound, and a manual 100,000-candidate sort benchmark.
- Added cooperative generation checkpoints to full, scoped, root-scoped, and aggregate KeyViewer work. Superseded scoped work returns a coherent warm composition for the merged successor request but cannot publish or acknowledge runtime output.
- Shared cached per-mod harvests through `Arc`, removed redundant target/keybind vector clones during aggregation, retained only active capability-specific entries, and exposed hit/miss/harvest/retained-entry counters.
- Evicted harvests from superseded Mods-root identities and bounded runtime-preflight snapshots with a small LRU.
- Added a manual 100,000-entry composition benchmark that measures one-mod take/update/store latency, non-empty payload aggregation, and estimated retained bytes.
- Kept all authority and caches process-local and reconstructible; no database migration or durable cache was added.

## Impacted Areas

- `src-tauri/src/modules/workspace/application/explorer/listing/paged.rs`
- `src-tauri/src/modules/workspace/domain/workspace/view.rs`
- `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs`
- `src-tauri/src/modules/automation/application/keyviewer/harvester.rs`
- `src-tauri/src/modules/system/application/app/post_apply.rs`
- `src-tauri/src/shared/errors.rs` and telemetry error classification
- `src/features/workspace-runtime/hooks/useWorkspaceExplorerPages.ts`
- `src/entities/workspace/model/explorerSelection.ts`
- `src/widgets/mod-explorer/hooks` explorer selection, runtime, and bulk hooks
- `src/shared/lib/appError.ts`, generated Tauri bindings, and common locale resources
- Focused Rust and frontend tests for snapshots, selection, cancellation, and harvest sharing

## Validation

- Rust library tests: 1,195 passed, 10 ignored.
- Frontend tests: 999 passed, 1 skipped across 196 files.
- Targeted explorer, selection, harvest-cache, and composition-cancellation tests passed.
- Specta binding export, TypeScript typecheck, `cargo check --lib`, Rust formatting, production frontend build, and `git diff --check` passed.
- ESLint passed with four unrelated Prettier warnings in collection-view and mod-inbox files.
- Manual 100,000-candidate listing benchmark: p50 3.1204 ms, p95 3.2335 ms, estimated snapshot memory 14,200,000 bytes.
- Manual 100,000-entry KeyViewer composition benchmark: one-entry update p50 3 µs/p95 5 µs; non-empty payload aggregation p50 132 ms/p95 143 ms; estimated composition memory 8,788,890 bytes.

## Notes

- Snapshot and composition memory values are structural estimates for regression comparison, not allocator/RSS measurements.
- Explorer snapshot eviction is intentionally recoverable: the client refetches a fresh first page and requires the user to reselect rather than broadening an old selection silently.
