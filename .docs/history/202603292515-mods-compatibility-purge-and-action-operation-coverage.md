# Purge Mods Compatibility Paths and Add Action Operation Coverage

## Context

Sesudah descriptor bus dan contract/runtime test sudah masuk, masih ada tiga gap di jalur utama `mods`: fallback compatibility query di move dialog, coverage operasi shared action yang belum ada, dan test lama yang masih menggambarkan arsitektur ObjectList sebelum `WorkspaceViewModel`.

## Changes

- Menghapus fallback compatibility di move dialog.
  - `MoveToObjectDialog` tidak lagi query object list sendiri lewat `commands.getObjects`.
  - Dialog sekarang selalu menerima `objects` dari consumer runtime `mods` dan hanya bertindak sebagai adapter UI.
- Menyelesaikan migration refresh imperative lama di hook `mods`.
  - `useFolders`, `useFolderMutations`, dan `useFolderGridImport` sekarang memakai descriptor-driven refresh untuk corridor/conflict/trash/manual refresh.
- Menambahkan operation coverage untuk shared action layer:
  - operasi mod runtime di `sharedOperations`
  - operasi object runtime di `sharedObjectActionOps`
- Menambahkan audit yang memastikan consumer utama `mods` tidak lagi bergantung pada:
  - `useObjects(...)`
  - `useModFolders(...)`
  - `commands.getObjects(...)`
- Menghapus test ObjectList lama yang sudah stale dan berbasis arsitektur query/sidebar sebelumnya.

## Impacted Files

### Runtime cleanup

- `src/features/folder-grid/MoveToObjectDialog.tsx` (modified)
- `src/features/folder-grid/hooks/useFolderGridImport.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)

### Operation tests

- `src/features/mod-runtime/operations/sharedOperations.test.ts` (added)
- `src/features/workspace-runtime/actions/sharedObjectActionOps.test.ts` (added)

### Contract/audit tests

- `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (modified)
- `src/features/workspace-runtime/useWorkspaceViewModel.contract.test.ts` (existing contract layer used)
- `src/features/folder-grid/MoveToObjectDialog.test.tsx` (modified)

### Legacy removal

- `src/features/object-list/ObjectList.test.tsx` (removed)

## Goal

Panel utama `mods` sekarang makin bersih dari compatibility query lama, operation layer shared actions punya coverage langsung, dan audit arsitektur menjaga supaya consumer utama tetap memakai runtime contract + descriptor bus.

## Impact

- Move dialog tidak lagi punya fallback data source sendiri.
- Shared mod/object action operations sekarang lebih aman direfactor karena sudah punya test operasi.
- Audit test lebih ketat terhadap dependensi helper/query lama di consumer `mods`.

## Notes

- Scope purge ini tetap khusus panel utama `mods`; helper/query lama yang masih dipakai surface non-`mods` tidak disentuh di batch ini.
