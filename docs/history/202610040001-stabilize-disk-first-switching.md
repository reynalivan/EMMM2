# Stabilize disk-first mod switching

## Context

Rapid toggles, delayed watcher echoes, and projection lag could disagree about identity, readiness, or latest intent.

## Changes

- Admit by physical identity/root epoch; revalidate before rename and preserve non-overlapping batch targets.
- Separate core readiness from projection lag; verify watcher successor/ancestor lineage.
- Reconciliation owns deferred projection; frontend shares snapshot/refresh/retry owners.
- Bound immutable transaction admission. Preserve acknowledged foreign-root commits as nonterminal `NeedsRepair` evidence.

## Impacted Files

Paths below are relative to the repository. Braces enumerate affected files.

- `src-tauri/{AGENT.md,src/lib.rs}`
- `src-tauri/src/modules/automation/application/hotkeys/{cycle_preset.rs,safe_mode.rs}`
- `src-tauri/src/modules/collections/adapters/tauri/tauri.rs`
- `src-tauri/src/modules/library/adapters/tauri/{conflict_cmds.rs,mod_bulk_cmds.rs}`
- `src-tauri/src/modules/library/application/mods/bulk/types.rs`
- `src-tauri/src/modules/mutation/{admission.rs,api.rs,coordinator.rs,journal.rs,mod.rs,recovery.rs,tests.rs}`
- `src-tauri/src/modules/reconciliation/{api.rs,adapters/tauri/mod.rs,adapters/tauri/toggle_projection.rs,application/mod.rs,application/toggle_projection.rs}`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/{disk_snapshot.rs,emit.rs,orchestrator/entry.rs,orchestrator/state.rs,orchestrator/tests.rs}`
- `src-tauri/src/modules/workspace/{adapters/tauri/workspace_cmds.rs,application/workspace/switch.rs,domain/workspace/switch.rs}`
- `src-tauri/src/modules/workspace/application/scanner/{tests/watcher_tests.rs,watcher/lifecycle.rs,watcher/mod.rs,watcher/suppressor.rs}`
- `src-tauri/src/{platform/fs/file_utils.rs,shared/path_key.rs}`
- `src/features/workspace-runtime/{index.ts,actions/useWorkspaceSwitchActions.ts,actions/useWorkspaceSwitchActions.test.tsx,actions/workspaceSwitchOps.ts,actions/workspaceSwitchOps.test.ts,actions/workspaceProjectionTracker.ts}`
- `src/shared/api/tauri/bindings.gen.ts`
- `src/widgets/mod-explorer/hooks/{useFolderGridBulk.ts,useFolderGridBulk.test.ts}`
- `docs/plans/stable-mod-switching/implementation_plan.md` (added)
- `docs/history/202610040001-stabilize-disk-first-switching.md` (added)

## Goal

Prioritize verified physical switching while derived consumers converge without overwriting newer intent.

## Impact

Journal format 3 accepts formats 1/2; downgrade requires compatible readers. Immutable admission allows four running/queued transactions per game; ordinary toggles are not capped by it.

## Notes

Rust: 1,352 passed, 12 ignored. Frontend: 1,117 passed, 1 skipped. Typecheck, ESLint, architecture lint, Vite build, rustfmt, Clippy, binding/permission tests, and focused review passed. Manual journal benchmark passed; it does not establish toggle latency.

Live native rage-click/debouncer/crash testing and paint/disk latency measurements remain pending. Full immutable-writer reference integration and collection runtime reservation extraction remain follow-ups documented in the plan. No push or installer build in this session.
