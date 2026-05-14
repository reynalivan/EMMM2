# Phase 7 Dead-Code Inventory

Date: 2026-05-14

Scope: evidence-first cleanup candidates after scanner/import and registry/modularity work. Deletion requires caller search, command registry audit, permission registry audit, and targeted tests to agree.

## Current Guardrails

| Guardrail                                           | Evidence                                                                                                                                                                                 | Decision                                                            |
| --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Legacy IPC aliases stay removed                     | Audit blocks removed command names such as `resolve_folder_collision`, `bulk_delete_mods_by_ids`, `get_log_lines`, `get_file_watcher_state`, `pin_object_cmd`, and `repair_orphan_mods`. | Keep audit coverage.                                                |
| Workspace refresh stays descriptor based            | Active source `invalidateQueries` is limited to `runtime-sync`, tests, mocks, and audit checks.                                                                                          | Keep runtime architecture audit as the deletion gate.               |
| Watcher/refocus/bootstrap do not call Deep Match    | Passive watcher/runtime/disk-reconcile paths have no Deep Match call sites; command registry entries are explicit scanner commands only.                                                 | Keep scanner/import architecture audit.                             |
| Production/Specta command registry is single-source | `src-tauri/src/lib.rs` uses one `emmm_collect_commands!()` registry for runtime and export.                                                                                              | Keep command registry audit requiring one `collect_commands!` list. |
| Frontend command wrappers need callers              | `commandRegistry.audit.test.ts` requires each `commands.*` wrapper to have a non-test caller or explicit policy entry.                                                                   | Delete no-caller wrappers by default.                               |

## Safe Cleanup Completed

| Candidate                                                             | Evidence                                                                                       | Decision                                                                             | Tests                                                                              |
| --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| `ArchiveModal.tsx` mixed shell/state/options/list/footer/collision UI | File was 598 lines and owned multiple responsibilities.                                        | Split into shell, state hook, utilities, list/row/options/footer/confirm components. | Scanner architecture audit, ESLint targeted.                                       |
| `commit_scan_results` passive missing-row purge                       | Scan commit deleted DB mod rows based on `Path::exists`, duplicating Disk Reconcile ownership. | Removed passive purge; `deleted_mods` remains `0` for scan commit compatibility.     | `cargo test services::scanner::sync --lib`.                                        |
| Collision `existing_mod_id` TODO                                      | Collision payload had `existing_mod_id: None` even when target folder was mapped.              | Added transactional mod lookup by path key.                                          | Collision mapped/physical-only tests.                                              |
| Duplicate Rust command registry lists                                 | Production and Specta had two long command lists in `src-tauri/src/lib.rs`.                    | Replaced with one shared macro used by both.                                         | `pnpm test -- src/lib/commandRegistry.audit.test.ts`.                              |
| Inline collection/browser service tests                               | Service files carried test blocks inside production module files.                              | Moved tests to `collection_service/tests.rs` and `browser/import_service/tests.rs`.  | `cargo test collection_service --lib`, `cargo test browser::import_service --lib`. |
| `deleteObject` typed escape hatch                                     | `objectService.deleteObject` used `as any` because binding omitted `force`.                    | Added `force` to wrapper params and removed `as any`.                                | TypeScript gate.                                                                   |

## Aggressive Removal Batch

| Removed surface                                            | Evidence                                                                    | Canonical replacement                                                |
| ---------------------------------------------------------- | --------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `list_mod_folders` / `commands.listModFolders`             | No non-test frontend caller; workspace UI consumes `WorkspaceViewModel`.    | `get_workspace_view_model` and internal `list_mod_folders_for_game`. |
| `pre_delete_check` / `commands.preDeleteCheck`             | No active caller; delete flow confirms directly before trash move.          | `delete_mod` with guarded backend path validation.                   |
| `toggle_favorite` / `commands.toggleFavorite`              | No active caller; single favorite actions already use bulk path.            | `bulk_toggle_favorite`.                                              |
| `get_thumbnail` / `commands.getThumbnail`                  | No active caller; folder cards use mod thumbnail pipeline.                  | `get_mod_thumbnail` and `ThumbnailCache::resolve`.                   |
| `check_shader_conflicts` / `commands.checkShaderConflicts` | No active caller; folder-wide conflict scan is the explicit scanner path.   | `detect_conflicts_cmd` and `detect_conflicts_in_folder_cmd`.         |
| `clear_pending_tasks` / `commands.clearPendingTasks`       | No active caller; recovery handling is explicit per task.                   | `app_startup_check` and `resolve_recovery_task`.                     |
| `browser_close_tab` / `commands.browserCloseTab`           | No active caller; frontend closes the Webview directly.                     | BrowserPage `Webview.close()`.                                       |
| `remove_game` / `commands.removeGame`                      | No active caller; settings page removes games through settings persistence. | `save_settings` with updated game list.                              |

## Next Candidates

| Candidate                                               | Owner Area           | Evidence                                                                              | Risk                                                                      | Recommended Phase                                                                      |
| ------------------------------------------------------- | -------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `object_repo.rs` large mixed repo                       | Backend objects      | Query, mutation, projection, canonical match, and fixture helpers remain in one file. | Medium-high: broad caller surface and schema-sensitive queries.           | Split after object repo baseline tests are green.                                      |
| `mod_repo.rs` large mixed repo                          | Backend mods         | Query, mutation, path-key, and sync projection helpers are mixed.                     | Medium: path-key drift can affect Disk Reconcile/runtime.                 | Split into path queries, mutations, sync projection helpers.                           |
| `browser/import_service.rs` production responsibilities | Browser import       | Queue, staging/hash, smart match, events, and pipeline are mixed.                     | Medium: smart import can call Deep Match only under explicit import flow. | Split with scanner/import architecture audit running after each move.                  |
| `collection_service.rs` production facade               | Collections/corridor | Dirty state, preview tree, recovery, and apply pipeline share one facade file.        | Medium-high: corridor dirty-state and apply pipeline are high-impact.     | Extract dirty-state, preview tree, recovery/path healing modules behind stable facade. |
| Test typed escape hatches                               | Frontend tests       | `@typescript-eslint/no-explicit-any` and `as any` clusters remain in feature tests.   | Low-medium: test-only, but can hide contract drift.                       | Clean per feature when touching tests.                                                 |
| Internal `too_many_arguments` allows                    | Rust services/repos  | Command boundary allows are intentional; internal service/repo allows remain.         | Medium: parameter structs improve readability but can churn callers.      | Replace only when modifying that function's owner module.                              |

## Deletion Policy

1. `rg` must show no active caller outside tests/docs, or the candidate must be replaced by a canonical path.
2. Command deletion requires production registry, Specta export, permission TOML, frontend binding audit, and wrapper usage audit to pass.
3. Filesystem-truth changes must preserve FileWatcher trigger-only, Disk Reconcile source of truth, runtime-sync refresh bus, and WorkspaceViewModel UI read-model.
4. Schema/domain compatibility fields remain if they are still data-state, even when the old command surface is gone.
