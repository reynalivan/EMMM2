# Disk Reconcile runtime sync hardening

## Context

Disk Reconcile sudah menjadi jalur runtime truth utama, tetapi object disabled/enabled masih bisa drift dari kondisi folder fisik di disk. Trigger FE juga masih perlu diketatkan supaya full reconcile hanya terjadi saat perlu.

## Changes

- Object runtime state di Disk Reconcile sekarang diselaraskan eksplisit dari folder root fisik:
  - `objects.folder_path`
  - `objects.folder_path_key`
  - `objects.status`
- Reconcile object rename/disable tidak lagi mengandalkan rename helper yang ikut mengubah `objects.name`.
- Read-model object/projection sekarang membaca disabled state dari `objects.status`, bukan menebak lagi dari prefix path string.
- Startup boot reconcile dipersempit ke active game saja.
- Coordinator FE sekarang membedakan:
  - first mods entry per game
  - game switched
  - window refocus
  - watcher/event reconcile
  dengan hydration/full-reconcile gate per game.
- Watcher batch besar/ambigu sekarang bisa escalate ke full reconcile.
- Fixed unrelated compile regressions:
  - async rollback resolution di collections recovery
  - missing type import di safe-mode confirm modal

## Impacted Files

- `src-tauri/src/services/disk_reconcile/reconcile.rs` (modified)
- `src-tauri/src/repo/object_repo.rs` (modified)
- `src-tauri/src/services/runtime_projection_service.rs` (modified)
- `src-tauri/src/services/disk_reconcile/orchestrator.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src/features/workspace-runtime/useWorkspaceViewModel.ts` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)

## Goal

Runtime projection ObjectList/FolderGrid sekarang lebih konsisten dengan kondisi nyata di disk, terutama untuk status object `DISABLED ...`, startup active game, refocus, watcher batch, dan entry pertama ke Mods Manager.

## Impact

- External disk changes tetap disk-first tanpa memanggil Deep Match Scanner.
- Full reconcile lebih jarang: sekarang fokus pada active game, game switch, first entry, manual repair, dan watcher ambiguity.
- Breaking change tidak ada pada public API; perubahan ada pada runtime behavior dan internal trigger policy.

## Notes

- Fokus object/navigation tetap hanya mengubah workspace runtime state dan query key, bukan memicu reconcile penuh.
