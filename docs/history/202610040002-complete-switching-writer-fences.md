# Complete switching ownership and writer integration

## Context

Finish the approved disk-first plan without adding another scheduler, lock, dependency or DB schema.

## Changes

- Carry native identity through listing/switch/bulk; retain root/participant proofs across waits and retries.
- Fence structural writers and coherent collection/Safe Mode snapshots; reserve runtime scopes before releasing the lease.
- Heal exact owned rows and descendants; cap checkpoints at repair holes and stop terminal frontend polling. Validate repair closure through successor lineage.
- Use per-request destination lookup and atomic no-overwrite; remove duplicate organizer execution. Preserve copied-import ownership and ambiguous recovery evidence.

## Impacted Files

Paths are repository-relative; braces enumerate the completion slice. Earlier core changes are listed in `202610040001-stabilize-disk-first-switching.md`.

- `src-tauri/AGENT.md`
- `src-tauri/src/modules/automation/application/hotkeys/{cycle_preset,safe_mode}.rs`
- `src-tauri/src/modules/{catalog,library,collections}/api.rs`
- `src-tauri/src/modules/collections/adapters/tauri/tauri.rs`
- `src-tauri/src/modules/library/adapters/tauri/{conflict_cmds,trash_cmds,mod_core_cmds,mod_meta_cmds,mod_bulk_cmds}.rs`
- `src-tauri/src/modules/library/application/mods/{mod.rs,bulk/types.rs,core_ops/naming.rs,core_ops/toggle.rs,object_switch/mod.rs,object_switch/resolve.rs,organizer_move.rs,tests/organizer_move_tests.rs,organizer_duplicates.rs}` (last file removed)
- `src-tauri/src/modules/mutation/{admission,coordinator,journal,mod,native_tests}.rs`
- `src-tauri/src/modules/mutation/application/workspace_mutation/{import_commit,object_status,tests}.rs`
- `src-tauri/src/modules/reconciliation/{adapters/tauri/runtime_sync.rs,adapters/tauri/toggle_projection.rs,application/toggle_projection.rs}`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/{emit.rs,rename_healer.rs,reconcile_tests.rs,orchestrator/entry.rs,orchestrator/request.rs,orchestrator/tests.rs,tests/rename_healer_tests.rs}`
- `src-tauri/src/modules/workspace/{adapters/tauri/workspace_cmds.rs,application/explorer/types.rs,application/explorer/listing/builder.rs,application/explorer/listing/grid.rs,application/explorer/listing/paged.rs,application/workspace/switch.rs,application/workspace/tests/prepared_switch_tests.rs,application/workspace_read_model/explorer_mapper.rs,application/scanner/tests/watcher_tests.rs,application/scanner/tests/native_watcher_stress.rs,domain/workspace/nodes.rs,domain/workspace/switch.rs,domain/workspace/view.rs}`
- `src-tauri/src/platform/fs/file_utils.rs`
- `src/demo/workspace.ts`
- `src/features/mod-runtime/hooks/useBulkModMutations{.ts,.test.tsx}`
- `src/features/workspace-runtime/{actions/useWorkspaceSwitchActions.ts,actions/useWorkspaceSwitchActions.test.tsx,actions/workspaceSwitchOps.ts,actions/workspaceSwitchOps.test.ts,actions/workspaceProjectionTracker.ts,state/workspaceState.ts}`
- `src/shared/api/tauri/bindings.gen.ts`
- `src/widgets/mod-explorer/hooks/{useFolderGrid.ts,useFolderGridActions.ts,useFolderGridBulk.ts,useFolderGridBulk.test.ts}`
- `docs/plans/stable-mod-switching/implementation_plan.md`
- `docs/history/202610040002-complete-switching-writer-fences.md` (added)

## Goal and Impact

Verified disk rename remains the switch acknowledgement boundary. Projection/runtime lag does not become a new switch lock. Ambiguous ownership stays actionable repair work rather than guessed success or destructive rollback.

## Notes

Rust: 1,392 passed/14 ignored; frontend: 1,124 passed/1 skipped. Typecheck, ESLint, architecture lint, Vite build, rustfmt, Clippy all-targets with warnings denied, binding/permission tests and focused review passed. Organizer fixture now models indexed native identity; production guards remain intact. Frontend timing failure under concurrent load passed isolation and final full rerun without timeout changes.

Native watcher takeover and eight subprocess recovery boundaries passed. Full native UI/paint, mixed-bulk fairness/memory, cold/network storage and persistent sharing/ACL/power-loss acceptance remain explicit plan gates. Interrupted copied imports without durable target proof retain `FailedNeedsRepair`. No push or installer build this turn.

Native disk-ack benchmark: 100 samples/cell, warm local Windows debug fixture. Flat 10k p95/p99: 486.858/594.029 ms before, 22.600/25.050 ms final repeat. A nested first-run outlier remains documented; IPC/UI and projection are excluded. Full six-cell evidence is in the plan.
