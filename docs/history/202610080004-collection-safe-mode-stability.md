# Collection and Safe Mode stability

## Context

The collection audit found dormant nonmember activation, unrecoverable Safe Mode
config writes, lost manual selection, inaccurate preview diffs, and unstructured
folder-lock errors. Final review also found stale saved safety classifications
and restore intent surviving an explicit change of baseline.

## Changes

- Include dormant children of activated Objects in exclusive-swap planning.
- Persist Safe Mode target/rollback intent; complete tasks after config sync.
- Preserve live edits, revalidate safety, follow semantic moves, and refresh
  restore selection when users save or replace the active baseline.
- Read current member safety while retaining missing-member snapshot metadata.
- Synchronize apply previews; show missing members and Object changes explicitly.
- Preserve FileInUse/PathBusy retry information and stale-path repair behavior.
- Repair typed preview test fixtures, isolate demo listeners, and add an in-memory
  collection preview/Skip Missing fixture flow.
- Split affected modules by responsibility to satisfy the 350-line limit.

## Goal / Impact

Collection switches preserve selection and expose recoverable failure states.
An additive migration adds Safe Mode snapshots and task intents; it deletes no
user data. No new application dependencies, IPC payload fields, or worktree.

## Validation

- Frontend: 1,222 passed, 1 skipped; production build and changed-file ESLint passed.
- Rust: 1,440 passed, 15 ignored with four test threads; collection regressions
  95/95 passed. `cargo check --lib --locked --offline` passed without warnings.
- One default-parallel 1,000-rename watcher test failed; it passed standalone and
  in the full four-thread rerun. Its trusted-echo window is 10 seconds. Production
  watcher logic was not modified; high-load/native timing remains a smoke target.
- Formatting, diff whitespace, and affected-file size checks passed.
- Browser reached collection list/preview and the apply error state before missing
  demo handlers were repaired. Further localhost access was rejected by browser
  security policy. No completed browser/native smoke claim is made.

## Remaining Smoke Coverage

On this patch: native F5/KeyViewer, real application crash/restart, late collisions
and third-party file locks, Save As/Update/Last Changes GUI flows, and large libraries.
Live GameBanana fetch and performance benchmarks remain ignored by the Rust suite.
Release/update, actual game launch, live downloads, and OS recycle-bin/dedup flows
were not smoke-tested in this session. Earlier client Workspace/Settings/Browser
evidence remains documented in `202610080002-client-smoke-regression-fixes.md`.

## Impacted Files

Modified or added; grouped braces enumerate filenames. This history is added.

- `src-tauri/migrations/{20261008000000_safe_mode_transition_intent.sql}`
- `src-tauri/src/modules/automation/application/hotkeys/{safe_mode.rs}`
- `src-tauri/src/modules/collections/adapters/sqlite/{mod.rs,members.rs,safe_mode.rs}`
- `src-tauri/src/modules/collections/adapters/tauri/{tauri.rs,apply_commands.rs,capture_commands.rs,command_tests.rs,preview_commands.rs,recovery_commands.rs,runtime_commands.rs,save_changes.rs}`
- `src-tauri/src/modules/collections/application/apply/{apply_pipeline.rs,apply_pipeline_tests.rs}`
- `src-tauri/src/modules/collections/application/apply/steps/{batch_rename.rs,mod.rs,resolve_current_state.rs,resolve_target.rs,rollback.rs,targets.rs}`
- `src-tauri/src/modules/collections/application/collection/{crud.rs,current_state.rs,live_state.rs,mod.rs,path_transition.rs,preview.rs,references.rs,safe_target.rs,references_transitions.rs,safe_mode.rs,safe_mode_references.rs}`
- `src-tauri/src/modules/collections/application/collection/tests/{mod.rs,parent_activation_tests.rs,safe_mode_tests.rs,safe_mode_state_tests.rs}`
- `src-tauri/src/modules/mutation/application/workspace_mutation/{engine.rs,engine_validation.rs}`
- `src-tauri/src/modules/workspace/adapters/sqlite/task/{mod.rs,testing.rs}`
- `src-tauri/src/modules/workspace/application/projected_state/{members.rs,mod.rs}`
- `src-tauri/src/modules/workspace/application/recovery/{mod.rs}`
- `src/demo/{commands.ts,collectionCommands.test.ts,collectionCommands.ts,collectionData.ts,collectionPreviewData.ts,commandResult.ts,dupScanData.ts,modInboxCommands.ts,settingsCommands.ts}`
- `src/features/workspace-runtime/hooks/{useBackgroundIndexingStatus.test.tsx,useBackgroundIndexingStatus.ts}`
- `src/pages/collections/{applyPreviewDiff.test.ts,applyPreviewDiff.ts,collectionPreviewSemantics.ts}`
- `src/pages/collections/components/{ApplyCollectionModal.tsx,CollectionPreviewPanel.test.tsx,CollectionPreviewPanel.tsx,CollectionTreeView.tsx,ApplyCollectionStatePanel.tsx,ApplyPreviewSummary.tsx,CollectionTreeNodeVisuals.tsx,RecursiveCollectionTree.tsx,VirtualCollectionTree.tsx,collectionTreeStructure.ts}`
- `src/shared/i18n/locales/{en,id,zh}/collections.json`
- `src/shared/lib/{appError.test.ts,appError.ts}`
- `src/widgets/mod-preview/hooks/{usePreviewPanelState.test.ts,previewPanelState.test-fixtures.ts,usePreviewPanelState.editing.test.ts}`
- `docs/plans/collection-stability/implementation_plan.md`
