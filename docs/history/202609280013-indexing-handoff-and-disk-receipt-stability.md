# Indexing handoff and disk receipt stability

## Context

Clean game revisits and startup could scan twice; repeated reconciliation also rewrote unchanged mod identities. Rapid bulk and conflict resolution needed stronger disk-identity checks before reporting success.

## Changes

- Clean watcher handoff preserves proven authority across active/inactive and startup sessions; prewarm no longer holds the activation lock during scanning. Unknown or late filesystem events still revoke proof.
- Reconcile compares snapshot-relative keys against the actual persisted key format. Known mod paths use the same relative key format for optional size scans; unchanged collection bindings avoid SQL updates.
- Multi-operation and bulk toggle projection scopes use all verified applied rename pairs. Bulk source bindings and final disk identities are rechecked before durable receipt.
- Folder conflict resolution, crash recovery, and collection rollback bind renames to physical identity and reject occupied destinations; unsupported non-Windows rename modes fail closed.
- UI readiness follows the selected game and activation proof, while stale game-switch and background progress responses cannot relock or overwrite the current game.

## Impacted Files

- Backend indexing/authority: `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/state.rs`, `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/tests.rs`, `src-tauri/src/modules/reconciliation/application/disk_reconcile/projection_writer/mods.rs`, `src-tauri/src/modules/reconciliation/application/disk_reconcile/projection_writer/write.rs`, `src-tauri/src/modules/reconciliation/application/disk_reconcile/reconcile.rs`, `src-tauri/src/modules/library/adapters/sqlite/mods/listing.rs`, `src-tauri/src/modules/collections/adapters/sqlite/references.rs` (modified).
- Backend switching/rollback: `src-tauri/src/modules/settings/adapters/tauri/settings_cmds.rs`, `src-tauri/src/modules/workspace/application/scanner/watcher/lifecycle.rs`, `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs`, `src-tauri/src/modules/library/adapters/tauri/mod_bulk_cmds.rs`, `src-tauri/src/modules/library/adapters/tauri/conflict_cmds.rs`, `src-tauri/src/modules/library/application/mods/core_ops/folder_conflict_resolution.rs`, `src-tauri/src/modules/collections/application/apply/steps/batch_rename.rs`, `src-tauri/src/modules/mutation/coordinator.rs`, `src-tauri/src/modules/mutation/recovery.rs`, `src-tauri/src/platform/fs/mod.rs` (modified); `src-tauri/src/platform/fs/rename.rs` (added).
- Frontend and tests: `src/app/entrypoint/App.tsx`, `src/app/entrypoint/App.test.tsx`, `src/entities/game/api/useActiveGame.ts`, `src/features/workspace-runtime/actions/useGameSwitch.ts`, `src/features/workspace-runtime/actions/useGameSwitch.test.ts`, `src/features/workspace-runtime/hooks/useBackgroundIndexingStatus.ts`, `src/features/workspace-runtime/hooks/useBackgroundIndexingStatus.test.tsx`, `src/widgets/mod-explorer/hooks/useFolderGridViewModel.ts`, `src/widgets/mod-explorer/hooks/useFolderGridViewModel.test.ts`, `src/pages/dashboard/hooks/useActiveGame.test.ts`, `src/pages/settings/modals/GameFormModal.test.tsx` (modified or added).
- Documentation: `src-tauri/AGENT.md`, `docs/plans/priority-game-indexing/implementation_plan.md` (modified).

## Goal

First-game readiness and clean revisits avoid redundant scans; rapid switches prioritize verified folder state before derived projections without stale UI locks.

## Impact

On isolated Windows fixtures, repeated full reconcile of 10,000 mods fell from 13.2 s to 3.0 s and stopped reporting false folder changes. Initial 1,000-mod reconcile measured about 0.21 s. No schema or dependency change. The benchmark is synthetic; real-library UI latency is not established by it.

## Notes

Windows no-replace behavior has native tests. Linux's no-replace branch cannot be exercised on this host; other platforms reject the operation. A separate process can still replace a source between identity check and OS rename, which would require handle-bound OS operations to eliminate completely.
