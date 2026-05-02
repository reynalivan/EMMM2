# Repo-Wide Closure and Requirement Realignment

## Context

Shared mod/context hooks still mixed policy, actions, effects, and runtime refresh. Several requirements also still described legacy command paths and invalidation patterns that no longer matched the accepted workspace runtime architecture in `.docs/flow.md`.

## Changes

- Split mod context menu handling into pure policy and imperative action layers.
- Slimmed shared mod actions into a composition root backed by shared dialog/effect helpers.
- Moved preview paste keydown handling into preview effects instead of the surface component.
- Replaced remaining direct runtime descriptor publishing / console logging in folder-grid dialogs with shared runtime result mapping.
- Updated requirement docs to reflect workspace switch, runtime descriptor, import bridge, and corridor-aware object list behavior.
- Expanded architecture audit coverage to guard menu-policy hooks, surface components, folder-grid dialogs, and active requirement docs.

## Impacted Files

- `.docs/requirements/req-08-smart-filters.md` (modified)
- `.docs/requirements/req-13-core-mod-ops.md` (modified)
- `.docs/requirements/req-14-bulk-operations.md` (modified)
- `.docs/requirements/req-15-foldergrid-interactions.md` (modified)
- `.docs/requirements/req-16-preview-panel-layout.md` (modified)
- `.docs/requirements/req-17-metadata-editor.md` (modified)
- `.docs/requirements/req-18-ini-viewer.md` (modified)
- `.docs/requirements/req-19-image-gallery.md` (modified)
- `.docs/requirements/req-20-mod-toggle.md` (modified)
- `.docs/requirements/req-23-mod-import.md` (modified)
- `src/features/mod-runtime/actions/modContextMenuPolicy.ts` (added)
- `src/features/mod-runtime/actions/sharedModDialogs.ts` (added)
- `src/features/mod-runtime/actions/sharedModEffects.ts` (added)
- `src/features/mod-runtime/actions/useModContextMenuActions.ts` (added)
- `src/features/mod-runtime/actions/useSharedModActions.ts` (modified)
- `src/features/mod-runtime/operations/sharedOperations.ts` (modified)
- `src/features/folder-grid/ConflictResolveDialog.tsx` (modified)
- `src/features/folder-grid/FolderCardContextMenu.tsx` (modified)
- `src/features/folder-grid/FolderCardContextMenu.test.tsx` (modified)
- `src/features/folder-grid/FolderListRow.tsx` (modified)
- `src/features/folder-grid/FolderListRow.test.tsx` (modified)
- `src/features/folder-grid/IgnoreManagementModal.tsx` (modified)
- `src/features/folder-grid/MoveToObjectDialog.tsx` (modified)
- `src/features/folder-grid/ObjectConflictModal.tsx` (modified)
- `src/features/preview/PreviewPanel.tsx` (modified)
- `src/features/preview/PreviewPanel.test.tsx` (modified)
- `src/features/preview/components/PreviewPanelContextMenu.tsx` (modified)
- `src/features/preview/hooks/usePreviewEffects.ts` (added)
- `src/features/preview/hooks/usePreviewRuntime.ts` (modified)
- `src/features/workspace-runtime/actions/sharedRuntimeResultMapper.ts` (added)
- `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (modified)
- `src/hooks/useModContextMenuItems.ts` (modified)

## Goal

Repo-wide mod/grid/preview shared hooks now follow the same policy/action/effect boundary, and active requirement docs no longer drift from the canonical workspace runtime architecture.

## Impact

- Context-menu hooks are now policy-only; imperative work lives in action hooks with toast/error handling.
- Preview and folder-grid surfaces rely on shared runtime result mapping instead of local refresh publishing.
- Architecture drift is harder to reintroduce because docs and audit tests now check the final contract directly.
- No backend or DB schema change in this wave.

## Notes

- `.docs/flow.md` remains the canonical architecture source when requirement implementation notes conflict with the accepted runtime design.
