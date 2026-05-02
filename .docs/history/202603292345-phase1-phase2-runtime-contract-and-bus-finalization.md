# Finalize Mods Runtime Contract and Descriptor Bus

## Context

Fase 1 dan 2 untuk workspace `mods` masih tertahan oleh dua hal: object list masih memakai kontrak runtime yang terpisah dari explorer/preview, dan refresh runtime utama masih bercampur antara descriptor bus, event manual, dan wrapper compatibility lama.

## Changes

- Menyatukan object row ke family kontrak runtime yang sama dengan explorer/preview.
  - Sebelum: object rows memakai shape semantic terpisah.
  - Sesudah: object rows memakai `WorkspaceNodeKind::Object` dan metadata runtime yang setara (`display_mode`, `type_chip`, `warning_state`, `inactive_reason`, `capabilities`).
- Mengganti publish refresh manual di jalur utama `mods` menjadi descriptor-driven bus.
  - Sebelum: beberapa modal, watcher, shared operations, dan handlers masih memanggil `publishRuntimeEvents(...)` langsung.
  - Sesudah: consumer utama `mods` membangun descriptor lalu publish lewat `publishRuntimeDescriptor(...)`.
- Menurunkan `refreshObjectListQueries()` menjadi compatibility-only.
  - Sebelum: helper ini masih dipakai di beberapa jalur runtime.
  - Sesudah: tidak ada pemakaian aktif di consumer utama `mods`; yang tersisa hanya definisi compatibility.
- Menghapus helper legacy selection runtime yang tidak lagi dipakai.
  - `runtimeSelection.ts` dihapus setelah machine runtime menjadi source of truth selection/navigation.
- Menyelaraskan test object/runtime mocks ke kontrak universal baru.

## Impacted Files

### Runtime contract

- `src/types/workspace.ts` (modified)
- `src-tauri/src/domain/workspace.rs` (modified)
- `src-tauri/src/services/workspace_service.rs` (modified)

### Descriptor-driven runtime refresh

- `src/hooks/useFolderMutations.ts` (modified)
- `src/features/mod-runtime/actions/useSharedModActions.ts` (modified)
- `src/features/mod-runtime/operations/sharedOperations.ts` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (modified)
- `src/features/folder-grid/MoveToObjectDialog.tsx` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src/features/folder-grid/ConflictResolveDialog.tsx` (modified)
- `src/features/folder-grid/ObjectConflictModal.tsx` (modified)
- `src/features/folder-grid/IgnoreManagementModal.tsx` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/workspace-runtime/actions/sharedObjectActionOps.ts` (modified)
- `src/features/object-list/AutoSetupModal.tsx` (modified)

### Legacy cleanup

- `src/features/workspace-runtime/runtimeSelection.ts` (removed)

### Tests

- `src/features/object-list/ObjectListContent.test.tsx` (modified)
- `src/features/object-list/ObjectRowItem.test.tsx` (modified)
- `src/features/object-list/useObjectListHandlers.test.ts` (modified)

## Goal

Workspace `mods` sekarang memakai kontrak runtime yang lebih seragam antar object/grid/preview, dan refresh utama berjalan lewat descriptor bus yang sama tanpa drift event manual di consumer utama.

## Impact

- Object list, folder grid, dan preview membaca semantic runtime yang lebih konsisten.
- Jalur refresh `mods` lebih mudah diaudit karena descriptor menjadi sumber tunggal refresh scope.
- Tidak ada perubahan schema database.
- Wrapper compatibility `refreshObjectListQueries()` masih ada untuk flow non-`mods`, tetapi bukan lagi jalur aktif runtime utama.

## Notes

- Scope batch ini hanya runtime `mods`; invalidation lama di settings, collections, atau flow non-`mods` tidak dibersihkan di sini.
