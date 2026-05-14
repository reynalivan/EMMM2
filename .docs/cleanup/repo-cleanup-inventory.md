# Repo Cleanup Inventory

Date: 2026-05-13

## Baseline Status

- Frontend dashboard/scanner/object-list stale i18n tests are repaired.
- `pnpm test -- src/features/file-watcher src/features/runtime-sync src/features/workspace-runtime src/features/dashboard src/features/scanner src/features/object-list` passes: 39 files, 223 tests, 2 skipped.
- `cargo test --lib` does not pass yet: 514 passed, 34 failed.
- Workspace/FileWatcher/Disk Reconcile guardrails still look aligned:
  - Raw `queryClient.invalidateQueries` is limited to `runtime-sync` and tests.
  - Raw `commands.setWatcherSuppression` is limited to `file-watcher/watcherSuppression.ts` and audit tests.
  - Watcher/Disk Reconcile source paths do not call Deep Match Scanner.

## Backend Test Failure Categories

### Likely Real Behavior Regressions

These failures touch runtime semantics or defensive behavior and should be fixed before broad dead-code deletion.

- `commands::mods::mod_core_cmds::*`
  - Prefix standardization still preserves malformed disabled prefixes.
  - Rename collision/case-insensitive duplicate checks do not return the expected collision error.
  - Suggested phase: fix mod core path normalization and collision contract first.
- `commands::mods::preview_cmds::details_command_remove_and_clear_preview_images`
  - Preview image removal rejects the test image as escaping the mod folder.
  - Suggested phase: verify canonical path validation and update fixture only if the new policy is correct.
- `commands::objects::object_cmds::test_get_objects_with_disabled_prefix`
  - Disabled prefix state does not map to expected object disabled state.
  - Suggested phase: confirm current disabled-state source of truth before changing the assertion.
- `commands::folder_grid::tests::test_list_mod_folders_invalid_subpath`
  - Test expects invalid subpath to return `Ok`, while current path validation returns an error.
  - Suggested phase: decide whether invalid user subpath should be empty/safe success or typed validation error.

### Stale Fixture Or Schema Drift

These look like test fixture setup no longer matches current schema or migrations.

- `commands::objects::object_cmds::test_delete_object_fk_constraints`
  - Fixture insert misses required `objects.folder_path`.
- `services::pin_service::*`
  - Fixture DB lacks `pin_config`.
- `services::scanner::dedup::resolver::*`
  - Resolver fixtures hit required job/schema fields during duplicate resolution.
- `services::collection_service::*` and `services::corridor_service::*`
  - Multiple failures are fixture uniqueness/FK/corridor-state drift.
- `services::app::tests::maintenance_service_tests::test_run_maintenance`
  - Expected cleanup count differs from current maintenance behavior.

### Stale Contract Assertions

These are probably tests expecting an old representation, not production breakage.

- `commands::tests::error_handling_tests::test_command_error_serialization`
- `types::tests::test_command_error_serialization`
  - Tests expect stringified errors; current serialization is enum-shaped JSON.
- `services::hotkeys::tests::hotkey_tests::list_bindings_returns_correct_count`
  - Expected count is 4, current count is 6.
- `services::keyviewer::tests::generator_tests::*`
  - Tests expect legacy text fragments such as `[KeyToggleBody]` and `Safe: OFF`.

### Runtime Or Platform Harness Issues

These may be real, but first pass should isolate test runtime assumptions.

- `commands::mods::mod_thumbnail_cmds::paste_thumbnail_rejects_oversize`
  - `can call blocking only when running on the multi-threaded runtime`.
- `services::images::thumbnail_cache::test_cache_hits_for_toggled_disabled_state`
  - Thumbnail cache singleton points at a temp path that may already be dropped on Windows.
- `services::scanner::dedup::resolver::test_nc_9_2_01_file_locked_graceful_skip`
  - Needs platform-specific lock behavior review.

## Dead-Code And Cleanup Inventory

### Large Mixed-Responsibility Files

Split only after the baseline is trustworthy. Priority is based on size plus behavior surface.

| File                                                                  | Lines | Suggested split                                                                                      |
| --------------------------------------------------------------------- | ----: | ---------------------------------------------------------------------------------------------------- |
| `src-tauri/src/services/collection_service.rs`                        |  1592 | Defer until collection/corridor tests are green. Split preview, apply, clone, delete, restore flows. |
| `src-tauri/src/repo/object_repo.rs`                                   |  1505 | Split read queries, writes, canonical match, test fixtures.                                          |
| `src-tauri/src/repo/mod_repo.rs`                                      |  1014 | Split identity writes, folder queries, metadata/thumbnail fields.                                    |
| `src-tauri/src/services/browser/import_service.rs`                    |   974 | Split staged download, archive extract, scanner handoff, commit.                                     |
| `src/features/scanner/components/ArchiveModal.tsx`                    |   598 | Safe frontend split candidate: archive row, password/collision controls, action footer.              |
| `src/features/object-list/ObjectList.tsx`                             |   589 | Safe split after object-list tests: toolbar/drop overlay/list shell.                                 |
| `src/features/folder-grid/FolderCard.tsx`                             |   485 | Safe split after folder-grid tests: status badges, actions, thumbnail/body.                          |
| `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.ts` |   490 | Split only after runtime contract tests stay green.                                                  |

### Suppression Cleanup

High-value next suppressions:

- `src/hooks/useDebounceCallback.ts`
  - Replace `any` callback args with a generic tuple type.
- `src/lib/services/objectService.ts`
  - Replace response escape hatch with a typed command result.
- `src/lib/services/dedupService.test.ts`
  - Replace broad test `any` with typed invoke mock helpers.
- `src/features/settings/tabs/GamesTab.test.tsx` and `MaintenanceTab.test.tsx`
  - Convert broad `any` mocks to typed command/service stubs.
- `src/features/preview/hooks/usePreviewPanelState.test.ts`
  - Replace broad `any` with focused preview state fixtures.

Keep for now:

- React Compiler `incompatible-library` suppressions around TanStack virtualizers and incompatible hooks.
- Tauri command boundary `too_many_arguments` allows. These preserve IPC shape.

Refactor when touched:

- Internal Rust service `too_many_arguments` allows in `services/mods/core_ops.rs`, `services/mods/bulk.rs`, `services/mods/organizer_ext.rs`, `services/mods/archive/extract.rs`, and `services/collection_preview_tree.rs`.
- Prefer params structs there, but do not change public command payloads in this phase.

### Legacy And Compatibility Surfaces

Keep unless a follow-up proves no live caller:

- `database/models.rs` legacy array payload handling.
- `game/schema_loader.rs` legacy game type normalization.
- `settings_repo.rs` deprecated table safeguard.
- `commands/objects/master_db_cmds.rs` array JSON compatibility.

Completed:

- `services/images/thumbnail_cache.rs` original-path compatibility API.
  - `ThumbnailCache::get_thumbnail` and the legacy `get_thumbnail` IPC were removed after frontend command usage audit proved no active caller.
- `commands/mods/mod_core_cmds.rs` backward-compat re-export for tests.
  - Candidate to remove after tests import concrete service functions.
- `services/scanner/sync/commit.rs` collision metadata.
  - `existing_mod_id` is now populated when a target path maps to an existing DB mod row.

## Recommended Next Phases

1. Fix backend baseline categories in this order:
   - mod core normalization/collision tests,
   - fixture/schema drift for object/pin/dedup,
   - collection/corridor fixture drift,
   - platform harness for thumbnail cache and blocking runtime.
2. Add focused tests for every behavior fix, then rerun `cargo test --lib`.
3. Remove broad TS test `any` in scanner/settings/preview tests.
4. Split `ArchiveModal.tsx`, `ObjectList.tsx`, and `FolderCard.tsx` as low-risk frontend modularity work.
5. Refactor large Rust repo/service files only after their current tests are green.
