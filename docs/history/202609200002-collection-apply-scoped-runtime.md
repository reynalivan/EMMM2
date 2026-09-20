# Scoped collection apply and stable bulk-move confirmation

## Context

Collection application still generated KeyViewer artifacts inside the durable pipeline, so a slow runtime refresh delayed task finalization and journal commit. Its rename preparation and terminal projection also loaded or scanned unrelated rows and roots. Separately, Bulk Move displayed one explicit selection but rebuilt the command payload from live React state at submit time.

## Changes

- Removed synchronous KeyViewer work from the collection apply pipeline. Tauri commands, preset hotkeys, Safe Mode, restore, and recovery now enqueue a latest-wins runtime generation only after durable apply returns.
- Kept the runtime enqueue under the game mutation lease so concurrent committed mutations retain generation order, while publication and reload remain asynchronous.
- Added a shared rewrite-to-runtime-scope helper: leaf rewrites resolve stable mod IDs, parent/object rewrites fall back to compact root scopes, and no-op applies still publish updated status fields.
- Limited collection rename target reads to affected mod root keys and collection object IDs.
- Reused identity-validating toggle plans for object renames, added expected identities to mod rename plans and journal steps, and rejected a replaced source before mutation.
- Marked the successful collection projection as trusted and scoped; failure recovery remains a full reconcile.
- Avoided building and sorting a complete preview projection when collection apply only needs active root keys, and avoided loading objects with no enabled mod beneath them.
- Removed redundant full reconciles after restore, preset hotkeys, and Safe Mode apply.
- Froze Bulk Move's game, query, listing revision, and explicit paths when the dialog opens. Submit now uses that exact backend selection snapshot even if the grid selection changes before confirmation.

## Impacted Areas

- Collection apply pipeline, live-state projection, rename planning, Tauri commands, and hotkey callers
- Mutation runtime rename identity validation and journal evidence
- Reconciliation runtime queue helper
- Projected-state root-key derivation
- Folder-grid bulk move hook and focused tests
- Backend architecture data-flow documentation

## Validation

- `cargo check --lib` passed.
- Full Rust library suite: 1,203 passed, 11 ignored.
- Focused Collection (72), workspace-mutation (15), and projected-state (6) tests passed.
- Bulk Move hook tests: 10 passed.
- TypeScript typecheck and targeted ESLint passed.
- Production frontend build and `git diff --check` passed.
- `cargo clippy --lib -- -D warnings` remains blocked by 42 pre-existing warnings outside these changes.

## Notes

- Collection snapshots and the set of currently active roots still have to be read to compute an exact preset diff. The optimization removes unrelated global object/mod indexes and filesystem census work; it does not pretend an inherently large 100,000-change apply can be constant time.
- Full reconcile remains the recovery path for failed mutation rollback or untrusted external filesystem state.
