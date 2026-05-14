# Phase 1 Requirement Gap Matrix

> Status: audit-only checkpoint for cleanup planning. This file maps active requirement docs to current implementation surfaces and records cleanup gaps for later phases.

## Audit Summary

- Requirement set audited: `req-01` through `req-44`; `req-24` is absent from active requirements.
- Runtime architecture baseline remains: FileWatcher is trigger-only, Disk Reconcile owns passive filesystem projection, `WorkspaceViewModel` is the workspace UI read-model, and `runtime-sync` is the frontend refresh bus.
- Phase 1 did not refactor production code. It only records gaps and corrects requirement text that could steer later work toward stale APIs.
- Severity scale: `High` blocks reliable cleanup or can hide drift; `Medium` is maintainability or coverage risk; `Low` is mostly hygiene/noise.

## Foundation Epics

| Epic | Runtime owner / source of truth | Active implementation entrypoints | Existing coverage | Gap / drift found | Severity | Next phase |
| --- | --- | --- | --- | --- | --- | --- |
| 01 App Bootstrap | Tauri setup, config service, startup Disk Reconcile | `src-tauri/src/lib.rs`, `src/App.tsx`, `app_cmds.rs` | `src/App.test.tsx`, backend config tests | `src-tauri/src/lib.rs` duplicates production and Specta command lists; command registry drift is already visible. | High | Phase 2 registry audit, Phase 6 lib split |
| 02 Game Management | `ConfigService` and game commands | `game_cmds.rs`, `GamesTab.tsx`, `GameSelector.tsx` | `GamesTab.test.tsx`, game schema loader tests | TS `GameType` enum and backend/string command surfaces still require casts in some UI paths. | Medium | Phase 3 typed test/helpers, Phase 6 config cleanup |
| 03 Onboarding | Onboarding state machine and game save commands | `WelcomeScreen.tsx`, `ManualSetupForm.tsx`, `AutoDetectResult.tsx` | Onboarding component tests | Baseline was repaired, but motion test mocks still leak unsupported DOM props as warning noise. | Low | Phase 3 harness cleanup |
| 04 Settings | Settings repo/config service | `useSettings.ts`, settings tabs, `settings_cmds.rs` | Settings tab tests and hook tests | Several settings tabs still use local `console.error` paths and test `any` escape hatches. | Medium | Phase 3 typed mocks, Phase 7 error normalization |
| 34 App Updater | Updater hook and metadata update commands | `useAppUpdater.ts`, `UpdateTab.tsx`, `update_cmds.rs` | `UpdateTab.test.tsx`, `useAppUpdater.test.ts` | Tests pass but emit `act(...)` warnings; update/runtime refresh behavior should stay descriptor-based. | Low | Phase 3 warning cleanup |
| 36 Toast & Error Handling | `appError`, toast store, error boundary | `appError.ts`, `useToastStore.ts`, `ErrorBoundary.tsx` | Toast/error boundary tests | Error formatting exists, but many feature surfaces still log raw errors locally before showing toast. | Medium | Phase 7 typed error cleanup |

## Workspace, Object, Folder, Preview Epics

| Epic | Runtime owner / source of truth | Active implementation entrypoints | Existing coverage | Gap / drift found | Severity | Next phase |
| --- | --- | --- | --- | --- | --- | --- |
| 05 Workspace Layout | Client layout shell only | `MainLayout.tsx`, `ResizableWorkspace`, `LaunchBar` | MainLayout/top-bar tests | Requirement is aligned; follow-up is mostly large-file/layout hygiene. | Low | Phase 4 UI modularity |
| 06 ObjectList Navigation | Workspace runtime state and local layout persistence | `ObjectList.tsx`, object list virtualizer hooks | ObjectList and virtualizer tests | `ObjectList.tsx` remains over 350 lines and mixes shell, state, modal, and handlers. | Medium | Phase 4 split |
| 07 Object List | Disk Reconcile projection via `WorkspaceViewModel` | `useWorkspaceViewModel.ts`, `ObjectList.tsx` | Workspace contract/audit tests, ObjectList tests | Architecture is aligned; test fixtures still use several `unknown as` casts. | Medium | Phase 3 typed fixtures |
| 08 Smart Filters | Object filter input mapped into `WorkspaceViewModel` | `useObjectListLogic.ts`, search worker, object filter state | Search/filter tests | No major drift; keep worker/filter code from becoming a second read-model. | Low | Phase 2 audit guardrail |
| 09 Object Schema | Game schema loader and MasterDB cache | `schema_loader.rs`, `master_db_cmds.rs`, `types/object.ts` | Schema loader and object service tests | Some frontend object/game schema types duplicate generated binding concepts. | Medium | Phase 7 type consolidation review |
| 10 Object CRUD | Object commands and repo, guarded disk creation | `object_cmds.rs`, `services/objects`, `object_repo.rs` | Object command/service tests | `object_repo.rs` is very large; `objectService.deleteObject` still uses `as any`. | High | Phase 3 typed escape cleanup, Phase 6 repo split |
| 11 Folder Listing | Shared classifier used by listing and Disk Reconcile | `services/explorer/listing.rs`, `list_mod_folders` | FolderGrid/listing tests, workspace tests | Backend command remains valid for explorer input, but workspace consumers must continue using `WorkspaceViewModel`. | Medium | Phase 2 audit expansion |
| 12 Folder Grid UI | Backend-filtered `WorkspaceViewModel.explorer` | `FolderGrid.tsx`, `FolderCard.tsx`, folder-grid hooks | FolderGrid/FolderCard tests | `FolderGrid.tsx` and `FolderCard.tsx` exceed target size and mix rendering with interaction state. | Medium | Phase 4 split |
| 13 Core Mod Ops | Workspace switch plus dedicated rename/delete commands | `execute_workspace_switch`, `core_ops.rs`, mod core commands | mod core backend tests, runtime action tests | `core_ops.rs` is large; command list duplication increases risk when adding/removing commands. | Medium | Phase 2 registry audit, Phase 6 service split |
| 14 Bulk Operations | Backend batch commands and runtime descriptors | `mod_bulk_cmds.rs`, `useObjHandlersBulk.ts` | Bulk/runtime tests | Frontend bulk handler is large; partial failure UX should stay centralized. | Medium | Phase 4 handler split |
| 15 FolderGrid Interactions | Policy hooks plus dedicated action hooks | context menu policy/actions, MoveToObject dialog | Context/menu/dialog tests | Policy split is mostly healthy; keep dialogs free of direct runtime descriptor publishing. | Low | Phase 2 guardrail |
| 16 Preview Layout | `WorkspaceViewModel.preview` and preview runtime hooks | `PreviewPanel.tsx`, `usePreviewPanelState.ts` | Preview and workspace reducer tests | Preview tests still rely on broad `any`; INI editor emits act warnings. | Medium | Phase 3 warning/typing cleanup, Phase 4 split |
| 17 Metadata Editor | Preview commands with Disk Reconcile refresh | metadata draft/action hooks, `update_mod_info` | Preview metadata tests | Local error logging and draft parsing can be centralized behind typed outcomes. | Medium | Phase 7 error cleanup |
| 18 INI Viewer | Preview commands and Disk Reconcile internal mutation | `IniEditorSection`, preview cmds | INI editor tests | Tests pass but emit `act(...)` warnings. | Low | Phase 3 warning cleanup |
| 19 Image Gallery | Preview image commands and thumbnail refresh | `GallerySection`, preview image commands | Gallery tests | Test run warns about empty `img src`; this may indicate an invalid UI state in fallback image handling. | Medium | Phase 3 real warning fix |
| 20 Mod Toggle | `execute_workspace_switch` and runtime mutation engine | workspace switch command/pipeline | runtime reducer/action tests, backend switch tests | Docs are aligned; keep frontend from calling old toggle commands. | Low | Phase 2 guardrail |
| 21 Mod Rename | Rename command plus disabled prefix normalizer | `rename_mod_folder`, scanner normalizer, core ops | mod core/normalizer tests | Recently stabilized; remaining risk is service size. | Low | Phase 6 service split |
| 22 Trash Safety | Trash service and commands | `trash.rs`, trash commands, TrashManager modal | Trash manager tests, backend trash tests | Restore/delete refresh must remain Disk Reconcile or descriptor-driven; no immediate doc drift. | Medium | Phase 2 refresh guardrail |
| 23 Mod Import | Import bridge plus archive/deepmatch flow | `import_mods_from_paths`, archive handlers, scan review modal | Import/drop/archive tests | `useObjHandlersArchive.ts` is large and combines archive analysis, extraction, skip, and auto-organize flows. | High | Phase 5 archive/import split |
| 28 File Watcher | Trigger-only watcher plus Disk Reconcile | `ExternalChangeHandler`, file-watcher hooks, watcher lifecycle | watcher hooks/audit tests | Architecture is well guarded; keep source unavailable and suppression paths covered. | Low | Phase 2 keep guardrails |
| 39 Folder Collision | Toggle conflict command/dialog plus archive collision resolver | `ConflictResolveDialog`, `resolve_conflict`, `collision_resolver` | Conflict dialog/backend tests | `resolve_folder_collision` exists in Specta test and permission but is not registered in production invoke handler or frontend binding. | High | Phase 2 command registry fix |
| 40 Metadata Actions | DB-only pin/favorite and guarded move | metadata commands, move mod operation | metadata/action tests | Mostly aligned; move refresh must stay runtime-sync descriptor based. | Low | Phase 2 guardrail |
| 41 Thumbnail System | Thumbnail cache plus preview image commands | thumbnail cache, `useThumbnail`, thumbnail commands | thumbnail tests | Legacy thumbnail APIs remain intentionally for compatibility; audit before removal. | Low | Phase 7 dead-code inventory |

## Scanner, Import, Dedup Epics

| Epic | Runtime owner / source of truth | Active implementation entrypoints | Existing coverage | Gap / drift found | Severity | Next phase |
| --- | --- | --- | --- | --- | --- | --- |
| 25 Scan Engine | Explicit scanner state, not Disk Reconcile | scan control cmds, scanner walker, `scanService` | scan service tests, scanner feature tests | Requirement text now clarifies `scanService.cancelScan()` -> `cancel_scan_cmd`; command is idempotent and returns success. | Medium | Phase 2 command registry audit |
| 26 Deep Matcher | Explicit scan/import/user sync only | deep matcher modules, scanner commands | deep matcher golden/unit tests | Large matcher modules are acceptable but need dead-code inventory before any pruning. | Medium | Phase 7 evidence sweep |
| 27 Sync Database | Explicit scan commit transaction | `scanner/sync/commit.rs`, commit command | sync tests | `commit.rs` still has an `existing_mod_id` TODO; decide whether to implement lookup or document invariant. | High | Phase 5 sync cleanup |
| 32 Dedup Scanner | Dedup scanner/resolver services | dedup commands, dedup service/hooks | dedup scanner/resolver tests | Resolver tests are large; core behavior currently covered after fixture drift cleanup. | Medium | Phase 7 test fixture cleanup |
| 37 Archive Extraction | Archive analysis/extraction pipeline | archive commands, `ArchiveModal.tsx`, archive handlers | archive service/component tests | `ArchiveModal.tsx` is over 350 lines; collision resolution UI and extraction options are intertwined. | High | Phase 5 archive split |
| 38 Auto Organizer | Deep Match preview/commit, not direct filesystem truth | bulk/drop auto-organize handlers, deepmatch preview for objects | scan service/object-list tests | Recent object-ID preview path is aligned; keep list-object-path IPC removed. | Low | Phase 2 guardrail |
| 44 Discover Hub / Smart Import | Browser download/import services | `BrowserPage.tsx`, browser commands, `import_service.rs` | browser component/service tests | Browser page and import service are large; command/queue lifecycle needs phase-specific audit before cleanup. | High | Phase 5/6 browser import split |

## Privacy, Collections, Runtime Epics

| Epic | Runtime owner / source of truth | Active implementation entrypoints | Existing coverage | Gap / drift found | Severity | Next phase |
| --- | --- | --- | --- | --- | --- | --- |
| 30 Privacy / Safe Mode | Corridor switch pipeline and backend-authoritative state | `switch_corridor`, `switch_pipeline`, `corridor_service` | PIN/safe-mode/corridor tests | Requirement pseudo-code was updated to reference active switch pipeline instead of stale direct `toggle_mods` flow. | Medium | Phase 2 docs audit expansion |
| 31 Collections | Collection service, corridor runtime, recovery tasks | `collection_service.rs`, collection commands, pipeline steps | collection/corridor tests | `collection_service.rs` is the largest service and mixes apply, dirty state, preview, and recovery concerns. | High | Phase 6 collection split |
| 35 Randomizer / Launcher | Random proposals plus workspace switch apply path | `suggest_random_mods`, `RandomizerModal`, launch command | randomizer tests | Randomizer calls switch command per proposal; later cleanup should ensure partial failures are surfaced consistently. | Medium | Phase 7 outcome consistency |
| 42 In-game Hotkeys | Hotkey manager and settings commands | hotkey manager/commands, settings tabs | hotkey tests | Hotkey manager is large; keep game-focus gating and cooldown as single owner. | Medium | Phase 6 hotkey split |
| 43 Dynamic KeyViewer | Keyviewer generator and active mod set refresh | keyviewer generator/harvester/resource pack | keyviewer tests | Generator is large; `.emmm_data` exclusion from scanner should be kept under audit. | Medium | Phase 2 scanner exclusion guardrail, Phase 6 split |

## Dashboard, Conflict, Metadata Epics

| Epic | Runtime owner / source of truth | Active implementation entrypoints | Existing coverage | Gap / drift found | Severity | Next phase |
| --- | --- | --- | --- | --- | --- | --- |
| 29 Conflict Detection | Conflict scanner plus workspace switch duplicate policy | conflict scanner/services, ObjectConflictModal, ignore commands | conflict and modal tests | Folder collision and shader conflict concepts are close but distinct; registry drift around `resolve_folder_collision` increases confusion. | Medium | Phase 2 command registry fix |
| 33 Dashboard | Dashboard query service and runtime-sync scopes | dashboard commands, hooks, dashboard components | Dashboard tests | Dashboard code is healthier after prior split; keep stats refresh descriptor-based. | Low | Phase 2 guardrail |

## Phase 2 Candidates

- Add a command registry consistency audit that compares production invoke handler, Specta export list, permissions, and frontend bindings.
- Resolve `resolve_folder_collision`, `bulk_delete_mods_by_ids`, and `get_log_lines` drift from command/permission inventory.
- Expand runtime docs audit to include all active runtime-adjacent docs, not only workspace docs.
- Add scanner exclusion guardrail for `.emmm_data` and any generated KeyViewer runtime files.

## Phase 3+ Cleanup Candidates

- Replace broad test `any`/`unknown as` casts with typed mock factories.
- Fix non-failing test warnings that point to invalid UI/test states.
- Split oversized frontend files before backend service splits so UI changes remain easier to review.
- Split backend services only after command registry and docs audit are locked.
