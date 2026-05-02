# Shared Runtime Actions, Event Bus, and Workspace Object Rows

## Context
Workspace runtime masih terpecah: grid dan preview punya action flow sendiri, refresh policy masih tersebar, dan ObjectList masih hidup di `ObjectSummary` lama walau grid/preview sudah mulai pindah ke workspace runtime.

## Changes
- Shared mod action engine diperluas untuk menangani rename, delete, move, duplicate warning, safe/unsafe flow, active-context flow, dan sync-with-DB dari satu hook.
- FolderGrid dan PreviewPanel sekarang memakai engine action yang sama; logic lokal tinggal khusus UI seperti reveal-in-explorer.
- Refresh runtime dinaikkan ke event-scoped bus lewat `publishRuntimeEvents(...)`, lalu dipakai di jalur mutation utama dan dialog conflict/auto-setup.
- `WorkspaceViewModel.objects` diperkaya menjadi `WorkspaceObjectRow` dengan metadata runtime final untuk ObjectList (`display_name`, `is_effectively_active`, `inactive_reason`, warning state).
- ObjectList virtualizer/row/context target dipindah ke row model runtime workspace, bukan lagi raw `ObjectSummary` untuk render utama.
- Mod context menu dirapikan ke policy-descriptor declarative agar visibility/action branching tidak tersebar.

## Impacted Files
- `src/features/runtime-sync/queryRefresh.ts` (modified)
- `src/features/mod-runtime/actions/useSharedModActions.ts` (added/reworked)
- `src/features/mod-runtime/operations/sharedOperations.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (reworked)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src/features/folder-grid/MoveToObjectDialog.tsx` (modified)
- `src/features/folder-grid/ConflictResolveDialog.tsx` (modified)
- `src/features/folder-grid/IgnoreManagementModal.tsx` (modified)
- `src/features/folder-grid/ObjectConflictModal.tsx` (modified)
- `src/features/preview/hooks/usePreviewPanelActions.ts` (reworked)
- `src/features/preview/components/PreviewPanelModals.tsx` (modified)
- `src/features/preview/PreviewPanel.tsx` (modified)
- `src/features/object-list/AutoSetupModal.tsx` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)
- `src/features/object-list/useObjectListVirtualizer.ts` (modified)
- `src/features/object-list/ObjectContextMenuTarget.ts` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/object-list/useObjHandlersBulk.ts` (modified)
- `src/hooks/useObjects.ts` (modified)
- `src/hooks/useModContextMenuItems.ts` (modified)
- `src/types/workspace.ts` (modified)
- `src-tauri/src/domain/workspace.rs` (modified)
- `src-tauri/src/services/workspace_service.rs` (modified)
- `src/features/object-list/ObjectRowItem.test.tsx` (modified)
- `src/features/object-list/ObjectListContent.test.tsx` (modified)
- `src/features/object-list/useObjectListVirtualizer.test.ts` (modified)

## Goal
Runtime `mods` workspace sekarang lebih backend-first dan lebih seragam: actions utama lewat satu engine, refresh utama lewat event bus, dan ObjectList sudah ikut model row runtime dari workspace.

## Impact
- Grid, Preview, dan ObjectList lebih sinkron untuk rename/delete/move/safe/sync flows.
- Refresh query lebih audit-able karena mutation utama publish event scope daripada invalidate key literal acak.
- ObjectList render lebih declarative dan siap untuk langkah berikutnya: state machine selection/dialog dan pengurangan query overlap lebih jauh.
- Tidak ada migration DB; perubahan backend hanya read-model enrichment.

## Notes
- Heavy preview details (full INI/image payload) masih tetap query terpisah; yang dipindah di fase ini adalah semantic runtime dan object row contract.
- Runtime Rust tests di environment ini masih hanya tervalidasi sampai compile.
