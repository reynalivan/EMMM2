# Switch Compatibility Cleanup And Runtime Audit Hardening

## Context

Workspace switch migration was mostly complete, but old switch commands, dead duplicate-check code, split duplicate payload types, and a corridor refresh gap were still leaving the app half-migrated.

## Changes

- Removed frontend-facing compatibility switch commands and dead duplicate-check hook.
- Removed Rust command exposure for legacy switch IPC while keeping `execute_workspace_switch` as the app-facing entrypoint.
- Unified duplicate warning UI to use the shared scanner duplicate type instead of a local shape.
- Expanded corridor switch refresh scopes to include dashboard, preview, and conflicts so enabled-state changes stay wired end to end.
- Moved remaining runtime cache helper imports away from public hook barrels where narrower infra modules already exist.
- Fixed `RandomizerModal` auto-roll effect so it no longer loops on empty state, and updated tests to the new switch command contract.
- Hardened runtime audit tests to block reintroduction of old switch commands.

## Impacted Files

- `src/lib/bindings.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/features/collections/hooks/useCorridorSwitch.ts` (modified)
- `src/features/folder-grid/DuplicateWarningModal.tsx` (modified)
- `src/features/preview/components/PreviewPanelModals.tsx` (modified)
- `src/features/randomizer/RandomizerModal.tsx` (modified)
- `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.ts` (modified)
- `src/features/mod-runtime/actions/useSharedModActions.ts` (modified)
- `src/features/mod-runtime/operations/sharedOperations.ts` (modified)
- `src/features/workspace-runtime/actions/useSharedObjectActions.ts` (modified)
- `src/features/workspace-runtime/optimistic/applyOptimisticEffects.ts` (modified)
- `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.test.ts` (modified)
- `src/features/folder-grid/DuplicateWarningModal.test.tsx` (modified)
- `src/features/mod-runtime/actions/useSharedModActions.test.ts` (modified)
- `src/features/workspace-runtime/actions/useSharedObjectActions.test.ts` (modified)
- `src/features/workspace-runtime/optimistic/applyOptimisticEffects.test.ts` (modified)
- `src/features/randomizer/RandomizerModal.test.tsx` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (modified)
- `src-tauri/src/commands/mods/mod_core_cmds.rs` (modified)
- `src-tauri/src/commands/scanner/conflict_cmds.rs` (modified)

## Goal

Make the switch migration fully app-facing, remove dead compatibility leftovers, and keep runtime refresh wiring consistent across mods, corridor, preview, keybindings, and dashboard updates.

## Impact

- Frontend production code no longer depends on `toggle_mod`, `enable_only_this`, or `check_duplicate_enabled`.
- Corridor switch now refreshes the same downstream systems as other enabled-state mutations.
- Duplicate warning flows use one shared payload type.
- Randomizer switch flow now follows the unified workspace switch contract.
- Raw invalidation remains centralized in the query bus and optimistic effect infra only.

## Notes

- `useFolders.ts` and `useObjects.ts` remain as public barrels for non-runtime consumers, but runtime internals now avoid routing cache helpers through them where a narrower module exists.
