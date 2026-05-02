## Title

Runtime projection foundation and switch impact contract

## Context

The workspace/runtime migration was stable, but object counts and switch refresh effects still relied on mixed recompute logic and frontend-side inference. This batch establishes a DB-backed runtime projection and starts moving switch refresh semantics into backend result contracts.

## Changes

- Added `object_runtime_projection` migration with initial backfill and indexes for runtime object counts/state.
- Added `runtime_projection_service` to rebuild or refresh projection rows after runtime-affecting mutations.
- Refactored object read queries to use projection-first data, with controlled fallback + self-heal for rows that do not yet have a projection entry.
- Rebuilt projection automatically after post-apply flows and disk reconcile, and refreshed projection for object create/update/delete.
- Added `WorkspaceImpact` and refresh-scope vocabulary to workspace switch results.
- Updated frontend workspace switch actions to consume backend impact refresh scopes instead of relying only on local mutation-class assumptions.

## Impacted Files

- DB / backend foundation:
  - `src-tauri/migrations/20260329133000_object_runtime_projection.sql` (added)
  - `src-tauri/src/services/runtime_projection_service.rs` (added)
  - `src-tauri/src/services/mod.rs` (modified)
- Backend query / mutation wiring:
  - `src-tauri/src/repo/object_repo.rs` (modified)
  - `src-tauri/src/services/app/post_apply.rs` (modified)
  - `src-tauri/src/services/objects/mutate.rs` (modified)
  - `src-tauri/src/services/disk_reconcile/reconcile.rs` (modified)
  - `src-tauri/src/domain/workspace.rs` (modified)
  - `src-tauri/src/commands/app/workspace_cmds.rs` (modified)
- Frontend contract / switch consumer:
  - `src/types/workspace.ts` (modified)
  - `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.ts` (modified)

## Goal

Runtime object/workspace reads now have a dedicated projection foundation, and workspace switch results now carry explicit backend refresh impact that frontend actions can consume directly.

## Impact

- Object/workspace reads are simpler and no longer depend only on raw in-memory mod counting.
- Core runtime-affecting flows now self-refresh projection after apply/switch/reconcile and object CRUD.
- Switch refresh semantics are less heuristic on the frontend.
- No DB schema rewrite was required beyond the additive projection table and indexes.

## Notes

- Projection-first is now established, but not every mutation family returns `WorkspaceImpact` yet.
- `object_repo` keeps a fallback path for missing projection rows so older tests/data setups remain compatible while the projection becomes universal.
