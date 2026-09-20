# Regional runtime mutation and activation coordination

## Context

Mod toggles and game activation coupled durable disk/DB work to full-library recovery and KeyViewer publication. That made normal operations wait on unrelated folders and surfaced `Disk changes were applied, but runtime refresh is still pending` even when the durable mutation had already succeeded.

## Changes

- Added identity-checked regional preflight, durable rename journals, scoped projection, and full-scan escalation only when local authority cannot be proven.
- Made single, bulk, object, parent, and trash mutations collision-aware and rollback-safe, including exact per-operation cancellation and no-op handling.
- Moved KeyViewer/runtime publication to a per-game, monotonic, single-flight queue with latest-wins status events and bounded retry behavior.
- Centralized game activation in the backend with generation authority, read-only cached UI hydration, buffered watcher catch-up, stale-result rejection, and per-game inactive watcher continuity.
- Added in-memory authority and incremental KeyViewer caches that remain reconstructible from disk and the database.
- Added stage timings, scan counters, structural fallback tests, concurrency tests, and 100/1,000/10,000-folder benchmark fixtures.
- Hardened trusted watcher evidence for paired and split Windows rename events, partial bulk execution, ambiguous root events, explicit watcher shutdown, and runtime-config path changes.
- Added foreground-intent cancellation for inactive prewarm so a background scan releases the global operation lease as soon as user work arrives.
- Rejected duplicate game IDs before settings persistence and aligned single/batch object resolution under canonical folder-name collisions.
- Bounded runtime publication to two concurrent workers, skipped superseded work before it starts, and gated manual retry until activation recovery is ready.

## Impacted Areas

- `src-tauri/src/modules/reconciliation` (regional discovery, authority, activation, runtime queue)
- `src-tauri/src/modules/workspace` (atomic switches, parent confirmation, watcher lifecycle)
- `src-tauri/src/modules/library` (single/bulk toggle and trash identity safety)
- `src-tauri/src/modules/mutation` (journal identity and recovery behavior)
- `src-tauri/src/modules/automation/application/keyviewer` (incremental harvest/publication cache)
- `src/app/store` and workspace/object/mod UI hooks (read-only activation and async status)
- `src/shared/api/tauri` and `src-tauri/permissions` (generated command contracts)

## Validation

- Rust library tests: 1,162 passed, 8 ignored.
- Frontend tests: 983 passed, 1 skipped across 192 files.
- TypeScript typecheck, ESLint, Tauri command permission test, Specta binding export, production frontend build, and `git diff --check` passed.
- Manual snapshot benchmark passed for 100, 1,000, and 10,000 folders. At 10,000 folders, warm full scan p50/p95 was 508.5/510.8 ms; trusted regional scan p50/p95 was 15.1/15.3 ms and classified only 100 folders.
- Normal trusted toggle and object-batch paths now perform one regional filesystem scope and one DB index load, while runtime publication is queued with zero command-path wait. Bulk path preparation uses one batched object query, one game-root query, and one immediate root index instead of per-object queries/scans.

## Known Baseline Gates

- The repository-wide DAL audit still reports existing violations in `catalog/adapters/tauri/master_db_cmds.rs` and `automation/adapters/tauri/hotkey_cmds.rs`; neither file is changed by this work.
- Repository-wide `cargo fmt --check` remains blocked by unrelated dirty browser/catalog/import code. Strict Clippy reports 39 pre-existing errors outside this mitigation set. All Rust files changed for this work pass direct `rustfmt --check`, and `cargo check` passes without warnings.
