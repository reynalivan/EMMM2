# Disk Reconcile E2E Closure

## Context

Disk Reconcile sudah disk-only di backend, tetapi end-to-end masih punya gap pada invalidation FE, internal preview/file writes, toast external change, dan sisa legacy code reconcile yang sudah tidak dipakai.

## Changes

- Menambahkan `InternalMutation`, `path_updates`, dan `change_summary` ke hasil Disk Reconcile untuk healing path FE, invalidation terarah, dan toast perubahan eksternal.
- Menyatukan preview/file writes internal (`write_mod_ini`, `update_mod_info`, save/remove/clear preview image, paste/update thumbnail) ke jalur `Disk Reconcile` dengan watcher suppression agar tidak konflik dengan file watcher.
- Memperluas invalidation FE Disk Reconcile ke objects, folder grid, thumbnails, collections/corridor, dashboard stats, conflicts, dan details/preview saat path aktif terdampak.
- Menambahkan rewrite path FE untuk selection, explorer path, dan grid selection saat rename/move terdeteksi.
- Menambahkan toast batch untuk perubahan eksternal object/mod folder, dan mengecualikan `InternalMutation`, onboarding, game switch, serta thumbnail-only.
- Mengganti fallback sync lama di FolderGrid ke `reconcileDiskState`.
- Menghapus dead code reconcile: watcher cooldown FE, `overlay_needed`, wrapper `gcLostObjects`, dan mock test legacy yang tidak dipakai.
- Memperbarui `flow.md` agar membedakan Disk Reconcile vs Deep Match Scanner, `InternalMutation`, dashboard invalidation, dan toast behavior.

## Impacted Files

- `.docs/flow.md` (modified)
- `src-tauri/src/services/disk_reconcile/reconcile.rs` (modified)
- `src-tauri/src/services/disk_reconcile/orchestrator.rs` (modified)
- `src-tauri/src/services/disk_reconcile/types.rs` (modified)
- `src-tauri/src/commands/mods/mod_meta_cmds.rs` (modified)
- `src-tauri/src/commands/mods/mod_thumbnail_cmds.rs` (modified)
- `src-tauri/src/commands/mods/preview_cmds.rs` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src/features/file-watcher/pathUtils.ts` (modified)
- `src/features/preview/hooks/usePreviewData.ts` (modified)
- `src/features/dashboard/hooks/useDashboardStats.ts` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/hooks/useObjects.ts` (modified)
- `src/lib/bindings.ts` (modified)
- `src/lib/services/objectService.ts` (modified)
- `src/stores/useAppStore.ts` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/object-list/useObjHandlersArchive.ts` (modified)
- `src/features/scanner/ScannerFeature.tsx` (modified)
- `src/features/object-list/ObjectList.test.tsx` (modified)

## Goal

Runtime refresh sekarang konsisten: disk tetap source of truth, DB tetap projection cepat, dan perubahan object/mod/runtime-file tersinkron ke ObjectList, FolderGrid, collections, keyviewer, thumbnail, dan dashboard tanpa memanggil Deep Match Scanner.

## Impact

- External add/remove/rename/move/modify sekarang bisa memicu toast, invalidate cache yang benar, dan heal selection/path FE.
- Internal preview/info/ini writes tidak lagi menunggu watcher dan tidak double-trigger side effect.
- Thumbnail-only refresh tetap ringan dan tidak membuat collection dirty.
- Breaking change: hasil public `DiskReconcileResult` bertambah field `path_updates` dan `change_summary`; FE bindings ikut diperbarui.

## Notes

- Rename/move healing paling akurat tetap datang dari watcher batch karena rename hints eksplisit. Focus/mods-entry reconcile tetap memulihkan truth dari disk, tetapi rename yang watcher miss masih bisa terdegradasi menjadi add/remove di beberapa edge case.
