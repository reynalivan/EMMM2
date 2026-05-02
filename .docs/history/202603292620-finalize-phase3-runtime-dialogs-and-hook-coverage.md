# Finalize Phase 3 Runtime Dialogs and Hook Coverage

## Context

Fase 3 masih belum penuh karena dialog conflict/duplicate/file-in-use di jalur `mods` masih membuka store legacy langsung, shared action hooks belum punya operation coverage di level hook, dan wrapper refresh/query lama masih tersisa di collections/corridor/dedup.

## Changes

- Memindahkan dialog runtime `mods` ke workspace machine.
  - Menambah varian dialog runtime untuk `conflict`, `duplicateConflict`, dan `fileInUse`.
  - Menambah helper dispatch dialog runtime agar consumer `mods` tidak lagi memanggil `useAppStore.getState().open...` langsung.
  - `ConflictResolveDialog`, `FolderGridModals`, `FileInUseDialog`, `ObjectList`, dan `FolderGridBanners` sekarang membaca atau membuka dialog lewat runtime machine.
- Menutup stale dialog state di shared action hooks.
  - `useSharedModActions` dan `useSharedObjectActions` sekarang mereduksi dialog dari runtime state terbaru, bukan closure state lama.
  - Flow sync/delete/escalation dialog jadi konsisten saat action berurutan memutakhirkan dialog yang sama.
- Menghapus wrapper compatibility refresh/query yang sudah mati.
  - `useCollections`, `useCorridorSwitch`, dan `useDedup` pindah dari `refreshObjectListQueries(...)` ke descriptor bus runtime.
  - `useObjects(...)`, `refreshObjectListQueries(...)`, dan `useModFolders(...)` dihapus karena tidak punya consumer produksi aktif lagi.
- Menambah operation coverage langsung untuk shared action hooks.
  - `useSharedModActions.test.ts`
  - `useSharedObjectActions.test.ts`
- Memperketat audit arsitektur.
  - Audit sekarang menjaga agar consumer utama `mods` tidak lagi membuka dialog runtime lewat store legacy.

## Impacted Files

### Runtime state and dialog bridge

- `src/features/workspace-runtime/state/workspaceState.ts` (modified)
- `src/features/workspace-runtime/state/workspaceReducer.ts` (modified)
- `src/features/workspace-runtime/state/workspaceDialogs.ts` (added)

### Dialog consumers

- `src/components/dialogs/FileInUseDialog.tsx` (modified)
- `src/features/folder-grid/ConflictResolveDialog.tsx` (modified)
- `src/features/folder-grid/FolderGridModals.tsx` (modified)
- `src/features/folder-grid/FolderGridBanners.tsx` (modified)
- `src/features/object-list/ObjectList.tsx` (modified)

### Shared actions and mutations

- `src/features/mod-runtime/actions/useSharedModActions.ts` (modified)
- `src/features/workspace-runtime/actions/useSharedObjectActions.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/hooks/useCorridorSwitch.ts` (modified)
- `src/features/scanner/hooks/useDedup.ts` (modified)
- `src/hooks/useObjects.ts` (modified)

### Tests and audit

- `src/features/mod-runtime/actions/useSharedModActions.test.ts` (added)
- `src/features/workspace-runtime/actions/useSharedObjectActions.test.ts` (added)
- `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (modified)
- `src/features/folder-grid/ConflictResolveDialog.test.tsx` (modified)

## Goal

Jalur utama `mods` sekarang memakai machine dialog dan descriptor/runtime bus secara konsisten, shared action hooks punya coverage langsung di level hook, dan compatibility wrapper lama yang tidak lagi dipakai sudah dicabut.

## Impact

- Dialog error/runtime di `mods` tidak lagi bergantung pada store legacy sebagai source of truth.
- Sync/update dialog di shared action hooks lebih stabil karena tidak lagi mereduksi dari state closure yang stale.
- Collections, corridor switch, dan dedup tidak lagi bergantung pada wrapper refresh object lama.
- Surface non-`mods` yang masih memakai `openFileInUseDialog` legacy tetap aman karena `FileInUseDialog` sekarang mendukung runtime dialog dan legacy dialog.

## Notes

- Scope cleanup legacy tetap fokus ke runtime `mods`; query independen non-`mods` seperti picker object di browser review tidak diubah.
