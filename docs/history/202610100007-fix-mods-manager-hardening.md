# Mods Manager hardening

Completion update: MM-5/MM-6 partial status and their native/recovery blockers
are superseded by [native completion](202610100011-complete-mods-preview-and-folder-recovery.md).
The results below remain the original historical record.

## Context

User approved MM-1–MM-6, including grouped conflicts and reversible preview hiding. Changes remain uncommitted on `main`; unrelated in-flight edits were preserved. Bug Hunter/Fixer scope and focused correctness review guided the implementation.

## Changes

- MM-1: raw relative metadata writes → validated physical disk targets, with original result labels and matching rollback paths.
- MM-2: unconditional ObjectList success → structured success/failure/warning feedback, including refresh failures and stale-game suppression.
- MM-3: DISABLED-equivalent INI containment → canonical physical containment.
- MM-4: shared conflict thumbnails → physical cache ownership; one grouped explorer card opens the existing focused conflict overlay. Normal toggles reuse cached image bytes.
- MM-5: preview deletion → persisted hide/show/restore without deleting assets. Managed fallback images remain restorable after preview import; canonical aliases share visibility, replacement custom thumbnails become visible, and invalidated cold generations cannot republish hidden thumbnails.
- MM-6: unbound staging → original root/parent/stage proofs, identity-bound journal preparation under the shared lease, no-overwrite promotion and owned-empty-directory-only rollback.

## Validation

- Nine targeted regression failures were reproduced before their production corrections; all are covered by passing final Rust suites. The fallback-after-import regression could not demonstrate RED because concurrent audit code temporarily failed compilation.
- Frontend: 58 files / 430 tests passed; typecheck, production build and scoped ESLint/Prettier passed.
- Rust: library 256 passed / 1 ignored; images 10 passed; finder 12 passed; coordinator-filtered tests 13 passed; command permission registry 1 passed. These are 292 distinct passing Rust tests.
- Strict Clippy, owned-file rustfmt and whitespace checks passed. New IPC bindings were generated, not hand-edited.
- Native Windows WDIO final completed run: 2 passed / 3 failed. Conflict grouping/overlay and gallery navigation/cancel passed. Junction creation passed an earlier run but failed the final run. No flaky case was excluded to obtain a clean verdict.
- Native failures recorded watcher coverage errors and mutation rejection: `Disk watcher authority needs repair before this operation; core indexing is already complete`. Unfavorite did not settle; bulk Pin was therefore not reached. Hide/restore and junction creation were rejected before their requested write.
- One final retry was blocked by another driver owning port 4444; its process was not terminated. Owned test fixtures were removed only after verified shutdown.
- Focused independent review found three edge cases, corrected them, and reported no actionable findings on re-review. Runtime verification remains incomplete.

## Manual review / pending verification

1. Coordinate the concurrent watcher changes and resolve identity-coverage loss on fresh native QA fixtures. Do not bypass the mutation guard. Rerun Favorite/Unfavorite, bulk Pin/Remove Pin, junction creation and gallery hide/show/restore, including fullscreen/right-click controls.
2. Verify actual external INI-editor launch and real clipboard/native image import. Containment, PNG normalization and hidden-thumbnail replacement are tested in Rust; native OS integration was not exercised.
3. Verify process-crash recovery for folder creation. Ownership, replacement, raced destinations and durable expected identity have unit coverage; a kill/restart recovery scenario was not run.

MM-1–MM-4 are verified at their scoped boundaries. MM-5/MM-6 are implemented but retained as partial pending native integration verification. Machine report: `C:/Users/yusri/AppData/Local/Temp/emmm2-mods-audit-20261010/mods-manager-fix-report.json` (schema validated). Report counters combine 722 distinct passing unit/component tests with 2 final native passes; its 3 new verification failures are not proven fixer-introduced regressions. Baseline counters describe targeted RED checks, not a full-repository before/after comparison.

## Impacted Files

Backend and generated boundary (modified unless marked added):

```text
src-tauri/AGENT.md
src-tauri/permissions/app-commands.toml
src-tauri/src/lib.rs
src-tauri/src/modules/library/adapters/tauri/mod_bulk_cmds.rs
src-tauri/src/modules/library/adapters/tauri/mod_core_cmds.rs
src-tauri/src/modules/library/adapters/tauri/mod_thumbnail_cmds.rs
src-tauri/src/modules/library/adapters/tauri/preview_cmds.rs
src-tauri/src/modules/library/adapters/tauri/thumbnail_cmds.rs
src-tauri/src/modules/library/adapters/tauri/tests/mod_core_cmds_tests.rs
src-tauri/src/modules/library/adapters/tauri/tests/mod_thumbnail_cmds_tests.rs
src-tauri/src/modules/library/application/mods/bulk/attributes.rs
src-tauri/src/modules/library/application/mods/metadata.rs
src-tauri/src/modules/library/application/mods/preview_image.rs
src-tauri/src/modules/library/application/mods/preview_ops.rs
src-tauri/src/modules/library/application/mods/tests/bulk_attributes_tests.rs
src-tauri/src/modules/library/application/mods/tests/metadata_conflict_tests.rs
src-tauri/src/modules/library/application/mods/tests/preview_image_tests.rs
src-tauri/src/modules/library/application/mods/tests/preview_ops_tests.rs
src-tauri/src/modules/mutation/coordinator.rs
src-tauri/src/modules/workspace/application/scanner/core/thumbnail.rs
src-tauri/src/platform/images/mod.rs
src-tauri/src/platform/images/preview_visibility.rs (added)
src-tauri/src/platform/images/thumbnail_cache.rs
src-tauri/src/platform/images/tests/thumbnail_cache_tests.rs
src/shared/api/tauri/bindings.gen.ts (generated)
```

Frontend and localization (modified):

```text
src/entities/mod/api/useThumbnail.ts
src/entities/mod/model/queryKeys.ts
src/entities/mod/model/queryKeys.test.ts
src/features/workspace-runtime/state/workspaceDialogs.ts
src/features/workspace-runtime/state/workspaceState.ts
src/widgets/object-sidebar/components/ObjectListConflictBanner.tsx
src/widgets/object-sidebar/hooks/useObjectBulkActions.ts
src/widgets/object-sidebar/hooks/useObjectBulkActions.test.ts
src/widgets/mod-explorer/FolderGrid.tsx
src/widgets/mod-explorer/components/FolderCard.tsx
src/widgets/mod-explorer/components/FolderCard.test.tsx
src/widgets/mod-explorer/components/FolderCardThumbnail.tsx
src/widgets/mod-explorer/components/FolderGridBanners.tsx
src/widgets/mod-explorer/components/FolderGridContent.tsx
src/widgets/mod-explorer/components/FolderListRow.tsx
src/widgets/mod-explorer/hooks/useFolderGrid.ts
src/widgets/mod-explorer/hooks/useFolderGrid.test.ts
src/widgets/mod-explorer/hooks/useFolderGridRuntime.ts
src/widgets/mod-explorer/hooks/useFolderGridRuntime.test.ts
src/widgets/mod-explorer/hooks/useFolderGridSelection.ts
src/widgets/mod-explorer/modals/FolderConflictManager.tsx
src/widgets/mod-explorer/modals/FolderConflictManager.test.tsx
src/widgets/mod-preview/PreviewPanel.tsx
src/widgets/mod-preview/PreviewPanel.test.tsx
src/widgets/mod-preview/components/GallerySection.tsx
src/widgets/mod-preview/components/GallerySection.test.tsx
src/widgets/mod-preview/components/PreviewConfirmDialogs.tsx
src/widgets/mod-preview/hooks/previewPanelState.test-fixtures.ts
src/widgets/mod-preview/hooks/usePreviewActions.ts
src/widgets/mod-preview/hooks/usePreviewData.ts
src/widgets/mod-preview/hooks/usePreviewData.test.ts
src/widgets/mod-preview/hooks/usePreviewPanelState.ts
src/widgets/mod-preview/hooks/usePreviewPanelState.editing.test.ts
src/widgets/mod-preview/hooks/usePreviewRuntime.ts
src/shared/i18n/locales/en/preview.json
src/shared/i18n/locales/en/grid.json
src/shared/i18n/locales/en/folder_grid.json
src/shared/i18n/locales/id/preview.json
src/shared/i18n/locales/id/grid.json
src/shared/i18n/locales/id/folder_grid.json
src/shared/i18n/locales/zh/preview.json
src/shared/i18n/locales/zh/grid.json
src/shared/i18n/locales/zh/folder_grid.json
```

Validation and records:

```text
tests/e2e/specs/client-workspace-smoke.e2e.ts (modified)
docs/plans/mods-manager-hardening/implementation_plan.md (added)
docs/history/202610100007-fix-mods-manager-hardening.md (added)
```

## Goal

Metadata and previews track physical ownership; conflicts are explicit; visibility is reversible; folder creation cannot overwrite or remove replacement paths.

## Impact

No dependency or DB migration. Existing preview remove/clear IPC now hides images, preserving mod bytes. Visibility travels with a renamed mod folder in a bounded sidecar. Native end-to-end release readiness is not established until the manual-review items above settle.
