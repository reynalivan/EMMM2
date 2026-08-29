# Folder conflict and anti-drift reconciliation

## Context

Enable/disable and watcher reconciliation could project two physical folders to the same stable ID, causing SQLite error 1555, duplicate watcher toasts, partial collection cleanup, and missed nested/offline rename healing.

## Goal

Make disk state authoritative across startup, watcher, single/bulk mutations, source relocation, metadata/file CRUD, collections, and conflict resolution without partial DB projections or duplicate error reporting.

## Changes

- Added coherent full-snapshot conflict preflight with `BlockedByFolderConflicts`; blocked passes preserve the last valid DB snapshot.
- Added bounded conflict details, secure Explorer/Trash actions, atomic two-phase batch rename with journal recovery, and a responsive EN/ID/ZH conflict manager.
- Added persistent filesystem identity and `NeedsRenameConfirmation`; unique offline renames auto-heal, while missing/ambiguous identity requires an explicit batch decision (`Rename` or separate delete/add).
- Made filesystem identity win over stale path keys, with two-phase DB staging for destination replacement and mod/object swaps so metadata, child ownership, and collection bindings follow the physical folders without stable-ID collisions.
- Serialized mutations and reconcile, added watcher backpressure/gap recovery and 3-second error-toast dedupe, and made startup/watcher restart perform a full recovery pass.
- Gated the first workspace read until startup recovery reaches a terminal result, and hardened conflict journals with root containment, source filesystem identities, quarantine of malicious data, swap-safe rollback, and journal retention on unverifiable rollback.
- Made durable collection members game-scoped and wildcard-safe, retain missing state after parent/child removal, rewrite arbitrary nested paths, and rebind when folders return.
- Added collision-safe collection set merging for offline A-to-B replacement, so two formerly distinct references converge to one live binding without a primary-key failure or a false missing-path report.
- Made collection rewrites swap-safe with a two-phase batch for object keys and member paths; collection-bearing mod swaps and whole-parent swaps now preserve both memberships and runtime bindings.
- Made recovery a per-activation single-flight gate with generation-safe terminal results. Startup settings hydration and game switching wait for the latest full disk scan, and failed recovery is surfaced instead of silently serving a stale projection.
- Migrated mutation callers to preflight + filesystem-first + trailing reconcile so single, bulk, parent, child, import, move, toggle, delete, hotkey, and watcher paths converge through one projection writer.
- Added `SourceUnavailable` directory inspection/apply with Matching/NewLibrary/Empty/Different classification, stale fingerprints, explicit empty/different confirmation, config rollback, and watcher-session replacement.
- Made category and auto-recognize metadata transactional, preserved manual mod categories during reconcile, and moved all DB writes behind the repository boundary.
- Hardened `info.json`, thumbnail, preview, INI, object-thumbnail, and KeyViewer writes with shared atomic replacement, measured rollback, game-scoped path validation, and scoped watcher suppression.
- Invalidated both in-memory and disk thumbnail caches for external image events; an external delete can no longer be undone accidentally by an atomic metadata writer recreating the missing folder.
- Added an automated command-registry/permission gate and documented the registration checklist; the audit also found and allowed four previously omitted commands.
- Added revision-based CAS and single-transaction settings persistence so stale UI/config writers cannot restore an old active game, mods directory, game list, or security settings after recovery.
- Made config startup fail closed after a non-authoritative DB read, persisted active-game clearing and explicit game-removal deltas, and serialized database reset with settings writers using a monotonic reset generation and rollback-tested destructive transaction.
- Validated every Deep Match write against the configured game root, serialized it with other mutations, journaled temporary moves, and forced a terminal full reconcile with rollback on prepare/DB/metadata failures.
- Committed the disk-derived core rows and affected runtime projection in one SQLite transaction; startup also retries collection/overlay effects so an earlier transient failure cannot disappear across an app restart.
- Hardened browser imports so duplicate reviews are staged, extracted, and matched before becoming actionable; essential writes propagate, cleanup is contained and bounded, and an unavailable app-data root fails explicitly.

## Impact

- Conflicts and unavailable sources now block safely while preserving the last valid projection and guiding users through recovery.
- External/offline changes converge on activation or watcher recovery; collection references follow confirmed rename identity and retain explicit missing state after deletion.
- Settings and reset writes are serialized, revision-checked, transactional, and fail closed; no database migration is required for the settings revision key.
- Full disk scans add bounded startup/activation work, while normal successful toggle/bulk reconciliation remains quiet and scoped.

## Impacted Files

Backend configuration and migrations:

- `src-tauri/{Cargo.toml,Cargo.lock,permissions/app-commands.toml}`
- `src-tauri/migrations/{20260825000001_preserve_collection_members.sql,20260825000002_filesystem_identity.sql}`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/app/{app_cmds.rs,game_cmds.rs,settings_cmds.rs}`
- `src-tauri/src/repo/{game_repo.rs,settings_repo.rs}`
- `src-tauri/src/services/config/{models.rs,persistence.rs,pin_ops.rs,service.rs,tests/service_tests.rs}`

Backend commands/domain/repositories:

- `src-tauri/src/commands/app/workspace_cmds.rs`
- `src-tauri/src/commands/collections/cmds.rs`
- `src-tauri/src/commands/duplicates/dup_resolve_cmds.rs`
- `src-tauri/src/commands/folder_grid/mod.rs`
- `src-tauri/src/commands/mods/{conflict_cmds.rs,mod_bulk_cmds.rs,mod_core_cmds.rs,mod_import_cmds.rs,mod_meta_cmds.rs,mod_thumbnail_cmds.rs,preview_cmds.rs,trash_cmds.rs}`
- `src-tauri/src/commands/mods/tests/conflict_cmds_tests.rs`
- `src-tauri/src/commands/objects/{object_cmds.rs,tests/object_cmds_tests.rs}`
- `src-tauri/src/commands/scanner/{archive_cmds.rs,disk_reconcile_cmds.rs}`
- `src-tauri/src/domain/collection.rs`
- `src-tauri/src/repo/collection_repo/{members.rs,references.rs,state.rs}`
- `src-tauri/src/repo/mod_repo/{listing.rs,sync.rs,types.rs,update.rs}`
- `src-tauri/src/repo/object_repo/{lookup.rs,types.rs,update.rs}`
- `src-tauri/src/repo/runtime_projection_repo.rs`

Backend services and tests:

- `src-tauri/src/services/{bootstrap.rs,workspace_switch_service.rs}`
- `src-tauri/src/services/browser/import_service/placement.rs`
- `src-tauri/src/services/collection_service/{references.rs,tests/mod.rs,tests/references_tests.rs}`
- `src-tauri/src/services/disk_reconcile/{disk_snapshot.rs,emit.rs,identity_conflicts.rs,mod.rs,reconcile.rs,reconcile_tests.rs,rename_confirmation.rs,rename_healer.rs,types.rs,watcher_batch.rs}`
- `src-tauri/src/services/disk_reconcile/source_recovery.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/{entry.rs,request.rs,run.rs,state.rs,tests.rs}`
- `src-tauri/src/services/disk_reconcile/projection_writer/{index.rs,mods.rs,objects.rs,prune.rs,tests.rs,write.rs}`
- `src-tauri/src/services/disk_reconcile/tests/rename_healer_tests.rs`
- `src-tauri/src/services/fs_utils/operation_lock.rs`
- `src-tauri/src/services/fs_utils/{atomic_file.rs,mod.rs}`
- `src-tauri/src/services/images/{thumbnail_cache.rs,tests/thumbnail_cache_tests.rs}`
- `src-tauri/src/services/keyviewer/generator/atomic.rs`
- `src-tauri/src/services/hotkeys/cycle_preset.rs`
- `src-tauri/src/services/mods/bulk/{delete.rs,toggle.rs}`
- `src-tauri/src/services/mods/core_ops/{conflict_resolution.rs,folder_conflict_resolution.rs,mod.rs,rename.rs,toggle.rs}`
- `src-tauri/src/services/mods/{object_switch/resolve.rs,organizer_move.rs,trash/service.rs}`
- `src-tauri/src/services/mods/{info_json.rs,metadata.rs,preview_image.rs}`
- `src-tauri/src/services/mods/bulk/attributes.rs`
- `src-tauri/src/services/objects/{mutate.rs,tests/mutate_tests.rs}`
- `src-tauri/src/services/scanner/{tests/watcher_tests.rs,watcher/event_filter.rs,watcher/events.rs,watcher/lifecycle.rs,watcher/mod.rs,watcher/suppressor.rs}`
- `src-tauri/tests/{arch_audit.rs,epic4_integration.rs}`

Frontend and generated contract:

- `src/features/collections/components/DeleteCollectionModal.tsx`
- `src/features/file-watcher/{ExternalChangeHandler.test.tsx,hooks.test.ts,hooks.ts,reconcileSelection.ts}`
- `src/features/folder-grid/components/FolderGridBanners.tsx`
- `src/features/folder-grid/components/{WorkspaceSourceUnavailableBanner.tsx,WorkspaceSourceUnavailableDialog.tsx,WorkspaceSourceUnavailableDialog.test.tsx}`
- `src/features/folder-grid/hooks/useFolderGridViewModel.ts`
- `src/features/folder-grid/modals/{ConflictResolveDialog.tsx,FolderConflictCandidateCard.tsx,FolderConflictManager.test.tsx,FolderConflictManager.tsx,RenameConfirmationManager.test.tsx,RenameConfirmationManager.tsx,folderConflictValidation.test.ts,folderConflictValidation.ts}`
- `src/features/workspace-runtime/actions/{workspaceSwitchOps.test.ts,workspaceSwitchOps.ts}`
- `src/features/workspace-runtime/actions/{sharedObjectActionOps.test.ts,sharedObjectActionOps.ts}`
- `src/features/workspace-runtime/state/{workspaceDialogs.ts,workspaceState.ts}`
- `src/lib/{bindings.gen.ts,bindings.ts}`
- `src/locales/{en,id,zh}/folder_grid.json`
- `src/stores/appStore/gameSlice.ts`
- `src/types/scanner.ts`

Acceptance artifacts:

- `tasks/{folder-conflict-drift-audit-plan.md,folder-conflict-drift-audit-todo.md}`
- `tasks/{source-recovery-mutation-drift-plan.md,source-recovery-mutation-drift-todo.md}`
- `.docs/knowledge/tauri-command-registration.md`
- `test/specs/phase3c-folder-conflicts.e2e.ts`

## Verification

- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed with no findings.
- `cargo test`: 759 passed, 2 ignored across 9 suites.
- `pnpm test -- --run`: 717 passed, 1 skipped across 134 files.
- `pnpm lint`: passed with 0 errors; 5 non-blocking `max-lines` warnings remain in test files only.
- `pnpm i18n:lint`: passed.
- `pnpm build`: passed.
- Command registry/permission gate and generated Specta bindings export: passed.
- CodeGraph full index rebuilt with the current engine: 1,001 files, 11,538 nodes, and 33,600 edges.
- Targeted WebdriverIO E2E: 2 passed (conflict rename/Trash, allowed conflict-details command, and nested offline rename/startup recovery).
- Manual 10,000-folder disk-snapshot benchmark: 214.0295 ms for classification on the current machine; the benchmark remains ignored in normal test runs.
- `git diff --check`: passed.
- Final read-only correctness/security/regression audit found no remaining production-code blocker; no TODO/FIXME or permissive staging-root fallback was left in the changed paths.

## Manual follow-up

- Windows Explorer window behavior and restoring an item from the system Recycle Bin remain manual smoke checks; automated tests verify command authorization, containment, and Trash invocation.
- Existing `.agents/` user changes were not touched.
