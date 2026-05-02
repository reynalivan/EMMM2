# Phase 4 legacy dialog store removal

## Context

Runtime `mods` sudah memakai machine dialog state, tetapi collection file-in-use dan dialog component masih menahan fallback ke legacy dialog fields di app store.

## Changes

- Collection apply/undo file-in-use flow dipindah dari `useAppStore.getState().openFileInUseDialog(...)` ke `openWorkspaceFileInUseDialog(...)`.
- `FileInUseDialog` tidak lagi membaca fallback legacy store; sekarang hanya render dari runtime dialog machine.
- Legacy dialog fields dan methods di `useAppStore` untuk conflict, duplicate conflict, dan file-in-use dihapus.
- Audit test diperketat untuk memastikan app store tidak lagi menyimpan dialog legacy.
- Hook test collection ditambah untuk memastikan error `FileInUse` membuka runtime dialog yang benar.

## Impacted Files

- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/components/dialogs/FileInUseDialog.tsx` (modified)
- `src/stores/useAppStore.ts` (modified)
- `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (modified)
- `src/features/collections/hooks/useCollections.test.ts` (modified)

## Goal

Legacy dialog compatibility terakhir yang masih mempengaruhi runtime `mods` hilang, dan file-in-use flow sekarang konsisten lewat runtime machine.

## Impact

- Jalur utama `mods` tidak lagi bergantung pada store dialog lama.
- Collection apply/undo tetap menampilkan retry dialog yang sama, tetapi source of truth-nya sekarang runtime machine.
- Tidak ada breaking change yang diharapkan untuk panel utama `mods`.
