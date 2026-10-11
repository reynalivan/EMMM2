# Fix Settings, shell ownership and warnings

## Context

Verified async UI/diagnostic defects and unused wrappers required correction.
The user also requested all observed warnings be fixed without disabling checks.

## Changes

- Fence diagnostic producers, persistence and IPC error reports to original consent.
- Limit frontend telemetry to explicit diagnostic payloads without automatic tracking.
- Bind compact actions, game editor callbacks and resize cleanup to their lifetimes.
- Remove verified unused helpers/shims; keep storage sizes and recovery contracts.
- Rename internal enums while preserving wire values, labels and runtime cache keys.
- Repair test async settling/mock DOM props, formatting and LF diagnostics; remove
  the native launcher's shell-spawn deprecation.

## Impacted files

All source/test/document files changed by this pass are listed below. Most of the
81 enum changes are qualified Rust reference substitutions; sixteen frontend test
files contain warning repairs. Ignored evidence/format archives stay in
`.audit-output/audit-settings-shell-fixes-20261011/` and the earlier audit directory.

<details>
<summary>180 affected files</summary>

- `.gitignore`
- `.prettierignore`
- `docs/history/202610120001-fix-settings-shell-and-warnings.md`
- `docs/plans/audit-dashboard-preview-20261011/coverage.md`
- `docs/plans/audit-dashboard-preview-20261011/report.md`
- `docs/plans/audit-settings-shell-20261011/coverage.md`
- `docs/plans/audit-settings-shell-20261011/report.md`
- `docs/plans/audit-settings-shell-fixes-20261011/implementation_plan.md`
- `docs/plans/audit-settings-shell-fixes-20261011/validation.md`
- `docs/plans/stable-mod-switching/native-webview-results.json`
- `docs/plans/storage-first-switching/implementation_plan.md`
- `docs/requirements/req-42-ingame-hotkeys.md`
- `docs/requirements/req-43-dynamic-keyviewer.md`
- `docs/test-cases/tc-42-ingame-hotkeys.md`
- `docs/test-cases/tc-43-dynamic-keyviewer.md`
- `index.html`
- `src-tauri/AGENT.md`
- `src-tauri/src/lib.rs`
- `src-tauri/src/modules/automation/adapters/tauri/hotkey_notice.rs`
- `src-tauri/src/modules/automation/api.rs`
- `src-tauri/src/modules/automation/application/hotkeys/manager.rs`
- `src-tauri/src/modules/automation/application/hotkeys/reload.rs`
- `src-tauri/src/modules/automation/application/hotkeys/tests/manager_tests.rs`
- `src-tauri/src/modules/automation/application/keyviewer/generator/ini.rs`
- `src-tauri/src/modules/automation/application/keyviewer/tests/generator/ini_tests.rs`
- `src-tauri/src/modules/automation/application/keyviewer/tests/generator/mod.rs`
- `src-tauri/src/modules/automation/application/keyviewer/tests/generator/reload_key_tests.rs`
- `src-tauri/src/modules/browser/application/browser/download_handler.rs`
- `src-tauri/src/modules/catalog/adapters/sqlite/object/tests.rs`
- `src-tauri/src/modules/catalog/adapters/sqlite/object/update.rs`
- `src-tauri/src/modules/catalog/adapters/tauri/tests/object_cmds_tests.rs`
- `src-tauri/src/modules/catalog/application/objects/tests/classification_tests.rs`
- `src-tauri/src/modules/catalog/application/objects/tests/mutate_tests.rs`
- `src-tauri/src/modules/catalog/application/objects/tests/query_tests.rs`
- `src-tauri/src/modules/catalog/application/objects/tests/reconcile_tests.rs`
- `src-tauri/src/modules/collections/adapters/sqlite/references.rs`
- `src-tauri/src/modules/collections/adapters/tauri/apply_commands.rs`
- `src-tauri/src/modules/collections/adapters/tauri/tauri.rs`
- `src-tauri/src/modules/collections/application/apply/apply_pipeline_tests.rs`
- `src-tauri/src/modules/collections/application/collection/tests/apply_tests.rs`
- `src-tauri/src/modules/collections/application/collection/tests/mod.rs`
- `src-tauri/src/modules/collections/application/collection/tests/preview_tests.rs`
- `src-tauri/src/modules/collections/application/collection/tests/update_tests.rs`
- `src-tauri/src/modules/collections/application/runtime/tests.rs`
- `src-tauri/src/modules/dashboard/adapters/sqlite/dashboard/tests.rs`
- `src-tauri/src/modules/dashboard/adapters/tauri/tests/dashboard_cmds_tests.rs`
- `src-tauri/src/modules/duplicates/adapters/sqlite/dedup/tests.rs`
- `src-tauri/src/modules/duplicates/adapters/tauri/tauri.rs`
- `src-tauri/src/modules/duplicates/api.rs`
- `src-tauri/src/modules/duplicates/application/dedup/hashing.rs`
- `src-tauri/src/modules/duplicates/application/dedup/signals.rs`
- `src-tauri/src/modules/duplicates/application/dedup/tests/dedup_resolver_tests.rs`
- `src-tauri/src/modules/duplicates/application/dedup/tests/dedup_scanner_tests.rs`
- `src-tauri/src/modules/games/adapters/sqlite/game/tests.rs`
- `src-tauri/src/modules/games/adapters/tauri/game_cmds.rs`
- `src-tauri/src/modules/games/adapters/tauri/tests/game_cmds_tests.rs`
- `src-tauri/src/modules/games/api.rs`
- `src-tauri/src/modules/games/application/game/schema_loader.rs`
- `src-tauri/src/modules/games/application/game/tests/validator_tests.rs`
- `src-tauri/src/modules/games/domain/models_test.rs`
- `src-tauri/src/modules/games/domain/models.rs`
- `src-tauri/src/modules/ingestion/adapters/sqlite/import_batch/tests.rs`
- `src-tauri/src/modules/ingestion/adapters/tauri/tauri.rs`
- `src-tauri/src/modules/ingestion/application/import_batch/placement_tests.rs`
- `src-tauri/src/modules/ingestion/application/import_batch/relocation.rs`
- `src-tauri/src/modules/ingestion/application/import_batch/tests.rs`
- `src-tauri/src/modules/library/adapters/sqlite/mods/batch.rs`
- `src-tauri/src/modules/library/adapters/sqlite/mods/listing.rs`
- `src-tauri/src/modules/library/adapters/sqlite/mods/mutate.rs`
- `src-tauri/src/modules/library/adapters/sqlite/mods/tests/listing_tests.rs`
- `src-tauri/src/modules/library/adapters/sqlite/mods/tests/mod_repo_test.rs`
- `src-tauri/src/modules/library/adapters/sqlite/mods/types.rs`
- `src-tauri/src/modules/library/adapters/tauri/mod_bulk_cmds.rs`
- `src-tauri/src/modules/library/adapters/tauri/mod_meta_cmds.rs`
- `src-tauri/src/modules/library/adapters/tauri/tests/mod_meta_cmds_tests.rs`
- `src-tauri/src/modules/library/adapters/tauri/tests/mod_thumbnail_cmds_tests.rs`
- `src-tauri/src/modules/library/adapters/tauri/tests/thumbnail_cmds_tests.rs`
- `src-tauri/src/modules/library/application/mod_health/service.rs`
- `src-tauri/src/modules/library/application/mod_health/tests.rs`
- `src-tauri/src/modules/library/application/mods/archive/types.rs`
- `src-tauri/src/modules/library/application/mods/tests/bulk_attributes_tests.rs`
- `src-tauri/src/modules/library/application/mods/tests/metadata_conflict_tests.rs`
- `src-tauri/src/modules/library/application/mods/tests/object_switch_tests.rs`
- `src-tauri/src/modules/library/application/mods/tests/organizer_move_tests.rs`
- `src-tauri/src/modules/matching/application/deep_matcher/analysis/indexes.rs`
- `src-tauri/src/modules/mutation/application/workspace_mutation/import_commit_tests.rs`
- `src-tauri/src/modules/mutation/application/workspace_mutation/tests.rs`
- `src-tauri/src/modules/reconciliation/adapters/tauri/disk_reconcile_cmds.rs`
- `src-tauri/src/modules/reconciliation/adapters/tauri/runtime_sync.rs`
- `src-tauri/src/modules/reconciliation/adapters/tauri/toggle_projection.rs`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/census_tests.rs`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/onboarding_session.rs`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/tests.rs`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/projection_writer/state.rs`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/projection_writer/tests.rs`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/reconcile_tests.rs`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/rename_confirmation.rs`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/source_recovery.rs`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/tests/rename_healer_tests.rs`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/types.rs`
- `src-tauri/src/modules/settings/adapters/tauri/settings_cmds.rs`
- `src-tauri/src/modules/settings/application/config/service.rs`
- `src-tauri/src/modules/settings/application/config/tests/service_tests.rs`
- `src-tauri/src/modules/system/adapters/sqlite/settings/tests.rs`
- `src-tauri/src/modules/system/adapters/sqlite/utils/unicode_keys.rs`
- `src-tauri/src/modules/system/adapters/tauri/app_cmds.rs`
- `src-tauri/src/modules/system/adapters/tauri/tests/app_cmds_tests.rs`
- `src-tauri/src/modules/system/application/app/bootstrap.rs`
- `src-tauri/src/modules/system/application/app/post_apply.rs`
- `src-tauri/src/modules/system/application/app/runtime_preflight_tests.rs`
- `src-tauri/src/modules/system/application/app/tests/app_service_tests.rs`
- `src-tauri/src/modules/system/application/app/tests/dashboard_tests.rs`
- `src-tauri/src/modules/system/application/telemetry_exporter.rs`
- `src-tauri/src/modules/system/application/telemetry/consent_tests.rs`
- `src-tauri/src/modules/system/application/telemetry/consent.rs`
- `src-tauri/src/modules/system/application/telemetry/mod.rs`
- `src-tauri/src/modules/workspace/adapters/mod.rs`
- `src-tauri/src/modules/workspace/adapters/sqlite/conflict/mod.rs`
- `src-tauri/src/modules/workspace/adapters/sqlite/task/testing.rs`
- `src-tauri/src/modules/workspace/adapters/tauri/watcher_runtime.rs`
- `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs`
- `src-tauri/src/modules/workspace/application/recovery/tests.rs`
- `src-tauri/src/modules/workspace/application/scanner/tests/master_db_cache_tests.rs`
- `src-tauri/src/modules/workspace/application/scanner/tests/native_file_moves.rs`
- `src-tauri/src/modules/workspace/application/scanner/watcher/lifecycle.rs`
- `src-tauri/src/modules/workspace/application/workspace/switch.rs`
- `src-tauri/src/modules/workspace/application/workspace/tests/mod.rs`
- `src-tauri/src/modules/workspace/application/workspace/tests/prepared_switch_tests.rs`
- `src-tauri/src/modules/workspace/application/workspace/tests/roots.rs`
- `src-tauri/src/modules/workspace/application/workspace/tests/switch_tests.rs`
- `src-tauri/src/modules/workspace/domain/classifier.rs`
- `src-tauri/src/modules/workspace/domain/tests/classifier_tests.rs`
- `src-tauri/src/modules/workspace/domain/workspace/impact.rs`
- `src-tauri/src/platform/fs/tests/guard_tests.rs`
- `src-tauri/tests/bulk_ops.rs`
- `src-tauri/tests/config_service.rs`
- `src/app/LaunchActions.integration.test.tsx`
- `src/app/store/useAppStore.test.ts`
- `src/features/import-batches/ImportBatchWizardHost.test.tsx`
- `src/features/match-wizard/components/ObjectClassificationWizardHost.test.tsx`
- `src/features/mod-runtime/actions/modViewerExternalReviewState.mounted.test.tsx`
- `src/features/mod-runtime/hooks/useBulkModMutations.test.tsx`
- `src/features/randomizer/RandomizerModal.test.tsx`
- `src/features/scanner/components/DuplicateReport.test.tsx`
- `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.test.tsx`
- `src/pages/browser/components/BrowserToolbar.test.tsx`
- `src/pages/dashboard/components/DashboardQuickActions.test.tsx`
- `src/pages/dashboard/hooks/useActiveGame.test.ts`
- `src/pages/onboarding/WelcomeScreen.test.tsx`
- `src/pages/settings/components/tabs/AITab.test.tsx`
- `src/pages/settings/components/tabs/GamesTab.session.test.tsx`
- `src/pages/settings/components/tabs/GamesTab.tsx`
- `src/pages/settings/components/tabs/GeneralTab.test.tsx`
- `src/pages/settings/hooks/useSettings.test.ts`
- `src/pages/settings/modals/GameFormModal.test.tsx`
- `src/pages/settings/modals/GameFormModal.tsx`
- `src/shared/api/tauri/bindings.gen.ts`
- `src/shared/api/tauri/bindings.telemetry.test.ts`
- `src/shared/api/tauri/bindings.ts`
- `src/shared/lib/telemetry.test.ts`
- `src/shared/lib/telemetry.ts`
- `src/shared/ui/components/layout/LaunchMenuTargetContext.ts`
- `src/widgets/app-shell/ResizableWorkspace.test.tsx`
- `src/widgets/app-shell/ResizableWorkspace.tsx`
- `src/widgets/launch-bar/LaunchBar.test.tsx`
- `src/widgets/launch-bar/LaunchBar.tsx`
- `src/widgets/mod-explorer/FolderGrid.test.tsx`
- `src/widgets/mod-explorer/hooks/useFolderGrid.test.ts`
- `src/widgets/mod-explorer/hooks/useFolderGridViewModel.test.ts`
- `src/widgets/mod-explorer/hooks/useFolderGridViewModel.ts`
- `src/widgets/mod-explorer/hooks/useFolderMutations.ts` (removed)
- `src/widgets/mod-preview/components/IniEditorSection.test.tsx`
- `src/widgets/object-sidebar/hooks/useObjectBulkActions.test.ts`
- `src/widgets/object-sidebar/hooks/useObjectListVirtualizer.test.ts`
- `src/widgets/top-bar/GlobalActions.tsx`
- `src/widgets/top-bar/index.test.tsx`
- `src/widgets/top-bar/index.ts`
- `src/widgets/top-bar/TopBarActionsPortal.tsx` (removed)
- `tests/e2e/specs/settings-shell-lifecycle.e2e.ts`
- `wdio.conf.ts`

</details>

## Goal and impact

Stale async work cannot redirect editor completion or diagnostics across sessions
or consent changes. Compact actions and global resize styles retain correct
lifecycles. Numeric/string wire contracts and existing database sizes remain intact.
No migration, dependency, installed-app or real-library change by this pass.

## Validation

Final full frontend validation passed 1,517 tests in 243 files with one existing
skip and zero warning diagnostics. Default Rust library validation passed 1,855
tests with zero failures and 40 ignored. Two stale Rust test checks were corrected
without changing production behavior. Focused serialization/registry and
actual-hook cleanup checks also passed. Release production/all-feature Clippy,
types, lint and independent review passed. Fresh private release native capture
passed all three cases; owned processes stopped and fixtures were removed.
All harness inputs and production logic match; five test-only deltas and one
whitespace-only HTML format delta have verified hashes.
Prettier checks all authored project files and leaves generated/evidence files
in their native format; JSON data and HTML non-whitespace content are unchanged.
Exact outcomes and limits are recorded in
`docs/plans/audit-settings-shell-fixes-20261011/validation.md`.

No real collector send, external picker or game launch is claimed. No commit/push
performed by this pass; stable source is handed to the separately authorized
release chat after final documentation and preservation checks.
