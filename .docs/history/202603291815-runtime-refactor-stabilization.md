# Runtime Refactor Stabilization

## Context

Refactor besar terakhir meninggalkan beberapa gap wiring pada runtime baru: projection object tidak selalu ter-refresh, manual repair belum memakai hasil Disk Reconcile penuh, refresh preview/thumbnail masih dobel, dan status object masih tercampur dengan konsep "no active mods".

## Changes

- Menutup refresh projection untuk mutasi DB-only yang mempengaruhi `object_runtime_projection`.
  - `toggle_mod_safe` sekarang refresh projection object terdampak.
  - `move_mod_to_object_service` sekarang refresh projection object lama dan baru.
  - `repair_orphan_mods` pindah ke helper runtime/object netral dan refresh projection setelah commit.
- Manual repair FE sekarang memakai hasil `reconcileDiskState(...)` langsung via `applyDiskReconcileResult(...)`.
- Internal preview/thumbnail flow tidak lagi publish refresh global kedua jika backend command sudah emit `disk_reconcile:result`.
- Menghapus fetch corridor ganda dari mutation hook runtime yang sudah punya descriptor refresh sendiri.
- Mengunci status object: object hanya dianggap disabled jika folder object fisiknya disabled.
- Menghapus sisa reason legacy `no_active_mods` dari runtime model.

## Impacted Files

- `src-tauri/src/services/runtime_projection_service.rs` (modified)
- `src-tauri/src/services/mods/metadata.rs` (modified)
- `src-tauri/src/services/mods/organizer_ext.rs` (modified)
- `src-tauri/src/services/workspace_service.rs` (modified)
- `src-tauri/src/domain/workspace.rs` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src/features/preview/hooks/usePreviewRuntime.ts` (modified)
- `src/features/preview/hooks/usePreviewActions.ts` (modified)
- `src/hooks/useFolderCoreMutations.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/types/workspace.ts` (modified)
- `src/features/workspace-runtime/workspaceSemantics.ts` (modified)
- `src/features/object-list/ObjectRowItem.test.tsx` (modified)
- `src/locales/en/common.json` (modified)
- `src/locales/id/common.json` (modified)
- `src/locales/zh/common.json` (modified)
- `src/features/collections/utils/refetchCorridorState.ts` (removed)

## Goal

Runtime baru sekarang kembali stabil: Disk Reconcile menangani disk truth, projection refresh menangani DB-only mutations, dan ObjectList/workspace tidak lagi drift atau menampilkan object disabled secara salah.

## Impact

- Preview/info/INI/thumbnail internal writes sekarang refresh satu jalur saja.
- Mutation runtime tidak lagi memicu refetch corridor ganda.
- Object dengan `enabled_count == 0` tetapi folder object fisiknya normal tetap dianggap enabled.
- Breaking change internal: reason `no_active_mods` dan helper `refetchCorridorState` sudah dihapus dari product source.

## Notes

- Jalur rename fisik yang sah tetap hanya apply/switch/recovery/explicit filesystem CRUD.
- `publishRuntimeDescriptor(...)` masih dipakai untuk mutation non-Disk-Reconcile; itu disengaja.
