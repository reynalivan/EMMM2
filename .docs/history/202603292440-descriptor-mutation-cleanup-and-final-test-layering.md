# Clean Up Descriptor Mutations and Finalize Test Layers

## Context

Sesudah fase 1 dan 2 selesai, masih ada sisa refresh imperative di beberapa hook `mods`, dan suite test belum menunjukkan struktur akhir yang jelas antara contract, operation, dan render coverage.

## Changes

- Memindahkan sisa refresh imperative di hook `mods` ke descriptor bus.
  - `useFolders` tidak lagi memakai `refreshRuntimeQueries(...)` langsung untuk corridor/conflict/trash refresh.
  - `useFolderMutations` untuk empty trash sekarang publish descriptor refresh.
  - `useFolderGridImport` untuk manual refresh grid sekarang publish descriptor refresh.
- Menipiskan surface compatibility kecil di ObjectList runtime.
  - `useObjectListLogic()` tidak lagi mengekspor setter `setSelectedObjectFolderPath`; selection tetap lewat runtime machine.
- Mengekstrak builder contract untuk workspace view model.
  - Menambahkan `buildWorkspaceViewModelFilter(...)`.
  - Menambahkan `buildWorkspaceViewModelInput(...)`.
  - Hook `useWorkspaceViewModel()` sekarang memakai builder ini, jadi contract data bisa diuji tanpa tergantung timing React Query.
- Menambahkan layering test akhir untuk runtime `mods`:
  - **contract**: builder filter/input dan query key `WorkspaceViewModel`
  - **operation**: batching refresh bus
  - **render/audit**: render node UI dan audit arsitektur jalur `mods`

## Impacted Files

### Descriptor cleanup

- `src/hooks/useFolders.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridImport.ts` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)

### Contract/runtime helpers

- `src/features/workspace-runtime/useWorkspaceViewModel.ts` (modified)

### Tests

- `src/features/runtime-sync/queryRefresh.test.ts` (existing operation coverage used)
- `src/features/workspace-runtime/useWorkspaceViewModel.contract.test.ts` (added)
- `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (modified)

## Goal

Runtime `mods` sekarang makin konsisten descriptor-first, dan test suite mulai terkunci ke struktur akhir `contract / operation / render` yang sesuai arsitektur baru.

## Impact

- Jalur refresh mutation `mods` lebih sedikit imperative branch.
- Contract data `WorkspaceViewModel` lebih mudah diuji dan dirawat.
- Audit arsitektur lebih ketat terhadap regress ke helper refresh lama.

## Notes

- Scope batch ini tetap khusus `mods`; compatibility path non-`mods` seperti collections/settings tidak dibersihkan di sini.
