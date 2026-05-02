# Surface Closure for ObjectList, FolderGrid, and PreviewPanel

## Context

Wave refactor sebelumnya sudah merapikan runtime `mods`, tetapi tiga surface utama masih menyimpan orchestration lokal: `ObjectList` masih memegang event bridge, `PreviewPanel` masih memegang import/asset side effects, dan `FolderGrid` masih menahan reveal/switch/dialog flow dalam satu hook besar.

## Changes

- `window` custom event untuk auto-organize/archive import diganti dengan `workspaceIntentBus` frontend-only.
- `ObjectList` bootstrap background sync dan intent subscription dipindah ke hook effect resmi.
- `usePreviewData` dijadikan raw query/mutation only; runtime invalidation/effect dipindah ke runtime layer.
- `PreviewPanel` asset/import/location actions dipindah ke hook aksi terpisah; komponen tidak lagi dispatch custom event atau memanggil command langsung.
- `FolderGrid` dipecah ke runtime, actions, dan selection hooks; ancestor-enable dialog dipindah ke workspace dialog state.
- object action gating dipusatkan ke `workspaceActionPolicy`, bukan branch enable/disable langsung di leaf component.

## Impacted Files

- `src/features/workspace-runtime/workspaceIntentBus.ts` (added)
- `src/features/workspace-runtime/state/workspaceState.ts` (modified)
- `src/features/workspace-runtime/state/workspaceDialogs.ts` (modified)
- `src/features/workspace-runtime/state/workspaceReducer.ts` (modified)
- `src/features/workspace-runtime/actions/workspaceActionPolicy.ts` (added)
- `src/features/object-list/hooks/useObjectListEffects.ts` (added)
- `src/features/object-list/ObjectList.tsx` (modified)
- `src/features/object-list/ObjectListContent.tsx` (modified)
- `src/features/object-list/ObjectContextMenu.tsx` (modified)
- `src/features/object-list/ObjectContextMenuTarget.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridRuntime.ts` (added)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (added)
- `src/features/folder-grid/hooks/useFolderGridSelection.ts` (added)
- `src/features/folder-grid/hooks/useFolderGridImport.ts` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src/features/folder-grid/EnableParentDialog.tsx` (modified)
- `src/features/folder-grid/FolderListRow.tsx` (modified)
- `src/features/preview/hooks/usePreviewData.ts` (modified)
- `src/features/preview/hooks/usePreviewRuntime.ts` (modified)
- `src/features/preview/hooks/usePreviewActions.ts` (added)
- `src/features/preview/PreviewPanel.tsx` (modified)

## Goal

Surface utama sekarang lebih dekat ke bentuk final `render + dispatch only`, dengan effect bridge, mutation refresh, dan ancestor-enable flow dipusatkan ke runtime/action hooks.

## Impact

- `ObjectList`, `FolderGrid`, dan `PreviewPanel` tidak lagi saling terhubung lewat `window` custom event.
- preview mutation flow tetap refresh runtime/query, tetapi wiring-nya tidak lagi hidup di raw query hooks.
- ancestor-disabled flow di `FolderGrid` sekarang ikut workspace dialog machine.
- breaking change internal: contract `ContextMenuTarget` sekarang punya policy terhitung, dan state dialog runtime bertambah `folderEnableParent`.

## Notes

- Batch ini fokus ke closure boundary frontend/runtime; tidak ada perubahan backend/DB.
