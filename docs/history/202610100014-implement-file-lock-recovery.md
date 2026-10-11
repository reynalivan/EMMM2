# Implement consistent file-lock recovery

## Context and changes

Approved plan: two automatic retries (250 ms, then 2 seconds), persistent manual recovery and safe process actions. Failed manual Retry now stays in the existing overlay. Windows discovery registers files rather than directories; force termination requires explicit confirmation and verified process identity/policy.

Structural actions share bounded retry and ownership checks. Verified transaction rollback preserves every original identity, reverses ancestor spellings and includes selected move no-ops. Partial results replay only failed items; fully rolled-back operations replay their complete frozen selection. Conflict workflows pause at the uncommitted step and Cancel stops later steps. Context renewal rejects replacement/new targets and revoked capabilities.

## Impacted files

Backend (`src-tauri/src/`, modified unless new):

- `lib.rs`, `shared/errors.rs`; platform `fs/{atomic_file,locking,mod,recycle_bin}.rs`, `fs/retry.rs` (new), `fs/locking/windows.rs` (new).
- System `api.rs`, `application/mod.rs`, `domain/mod.rs`, `adapters/tauri/mod.rs`, `application/app/readonly_audit.rs`, `application/telemetry/mod.rs`; `application/file_lock.rs`, `domain/file_lock.rs`, `adapters/tauri/file_lock_cmds.rs` (new).
- Mutation `application/mod.rs`, `application/file_lock_scope.rs` (new), `application/workspace_mutation/{engine,object_status,import_commit}.rs`.
- Library `adapters/tauri/{mod_core_cmds,mod_bulk_cmds,mod_meta_cmds,trash_cmds,conflict_cmds}.rs`; `application/ini/write.rs`; `application/mods/{info_json,organizer_move}.rs`, `mods/core_ops/{toggle,rename,folder_conflict_resolution}.rs`, `mods/object_switch/toggle.rs`, `mods/bulk/{toggle,delete}.rs`, `mods/trash/service.rs`; associated bulk-toggle/organizer tests.
- Collections `adapters/tauri/apply_commands.rs`, `application/collection/apply.rs`, `application/apply/apply_pipeline.rs`, `application/apply/steps/{mod,batch_rename,rollback}.rs`.
- Catalog `application/objects/mutate.rs`, `adapters/tauri/object_cmds.rs`; Workspace `application/workspace/switch.rs`, `adapters/tauri/workspace_cmds.rs`.

Frontend (`src/`, modified unless new):

- `features/file-watcher/dialogs/FileInUseDialog.tsx` and new component tests.
- Workspace runtime `actions/{fileInUseRetry,workspaceSwitchOps,useObjectMutations,useSharedObjectActions}.ts`, `state/workspaceState.ts`; associated new/updated recovery, object and workspace tests.
- Mod runtime `hooks/{useFolderCoreMutations,useBulkModMutations}.ts`, `operations/sharedOperations.ts` and associated tests; `pages/collections/hooks/useCollections.ts`, `entities/game-object/api/objectService.ts`.
- `widgets/mod-explorer/modals/FolderConflictManager.tsx`, its normal/trash tests, `hooks/useFolderGridBulk.ts` and tests.
- `shared/lib/appError.ts` and tests, `shared/api/tauri/{bindings,bindings.gen}.ts`; EN/ID/ZH `common.json`.

Other: `src-tauri/Cargo.toml`, `permissions/app-commands.toml`, `src-tauri/AGENT.md`; new `tests/e2e/specs/mods-manager-file-locks.e2e.ts` and `support/hold-file-lock.rs`; plan/validation documents and this history. Unrelated dirty changes were preserved.

## Validation and impact

Final Rust **1,679 passed / zero failed / 22 ignored**; additional file-lock filter17 pass. Frontend targeted tests, typecheck/build, scoped lint/architecture/format and strict all-target Clippy pass. Native Windows **4/4 pass**, session `82f686e7527e70c8e6cb2a631bbdbe89`; only owned fixture processes controlled and cleaned. Review findings fixed and re-reviewed.

Earlier linker and watcher failures plus the broader E2E typecheck limitation remain in [validation](../plans/mods-manager-file-locks/validation.md). No new dependency package, schema migration, elevation, driver, install, commit or push. Directory/kernel holders can remain unknown; native UI coverage is scoped rather than every planning-matrix row.
