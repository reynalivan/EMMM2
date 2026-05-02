# Workspace Switch Object Toggle Fix

## Context

Object enable/disable masih lewat service toggle mod, sehingga path stale, rename root object, dan bulk toggle menghasilkan error runtime seperti `os error 2/3/5` atau toast `[object Object]`.

## Changes

- Object target di `Workspace Switch` dipisah ke service backend khusus object root.
- Object switch sekarang heal path fisik lebih dulu, lalu rename root object, update child mod paths/status, update collection reference, dan refresh projection.
- Rename fallback hanya copy/move untuk cross-device nyata; `AccessDenied` same-volume sekarang diperlakukan sebagai `FileInUse` atau `PathBusy`.
- Mod toggle dan bulk mod toggle sekarang ikut refresh `object_runtime_projection`.
- FE bulk/failure toast sekarang format error typed dengan benar, tidak lagi menampilkan object mentah.
- Object filter status backend sekarang pakai `is_object_disabled`, bukan heuristik `enabled_count == 0`.

## Impacted Files

- `src-tauri/src/services/mods/object_switch.rs` (added)
- `src-tauri/src/services/mods/mod.rs` (modified)
- `src-tauri/src/services/fs_utils/file_utils.rs` (modified)
- `src-tauri/src/domain/errors.rs` (modified)
- `src-tauri/src/services/mods/core_ops.rs` (modified)
- `src-tauri/src/commands/app/workspace_cmds.rs` (modified)
- `src-tauri/src/repo/object_repo.rs` (modified)
- `src-tauri/src/services/runtime_projection_service.rs` (modified)
- `src-tauri/src/services/mods/bulk.rs` (modified)
- `src/lib/appError.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/features/object-list/useObjHandlersBulk.ts` (modified)
- `src/features/mod-runtime/operations/sharedOperations.ts` (modified)
- `src/types/scanner.ts` (modified)
- `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.ts` (modified)

## Goal

Explicit enable/disable runtime sekarang sinkron dengan arsitektur terbaru: object switch punya owner sendiri, projection ikut refresh, dan FE menerima error/runtime result yang konsisten.

## Impact

- ObjectList, FolderGrid, dan bulk toggle sekarang memakai contract error yang lebih manusiawi.
- Object dengan `enabled_count == 0` tidak lagi dianggap disabled hanya karena count nol.
- Tidak ada fallback move pada rename same-volume yang sedang busy; flow sekarang fail fast dengan error typed.
- Tidak ada breaking change public command, tetapi behavior switch object dan bulk failure toast berubah.

## Notes

- Disk Reconcile tetap tidak dipakai untuk explicit toggle runtime.
- Nama object tetap dipertahankan bersih saat runtime disabled prefix hanya mengubah `folder_path`.
