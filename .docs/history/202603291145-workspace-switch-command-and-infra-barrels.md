# Workspace switch command and infra barrels

## Context

Switch runtime frontend sudah terpusat, tetapi duplicate-resolution masih memakai command lama `enable_only_this`. Di sisi lain, `useFolders.ts` dan `useObjects.ts` masih memuat logic cache/mutation yang terlalu besar untuk sekadar infra surface.

## Changes

- Tambah command backend `execute_workspace_switch` sebagai jalur tunggal switch mod/object.
- Tambah contract switch terstruktur di domain workspace untuk input, target, resolution, status, duplicate payload, dan result.
- Tambah support `force_enable` di backend toggle service agar duplicate force-enable tidak lagi tergantung workaround frontend.
- Migrasikan frontend switch engine ke `commands.executeWorkspaceSwitch(...)` untuk toggle normal, force-enable, dan enable-only-this.
- Hapus pemakaian direct `commands.enableOnlyThis(...)` dari runtime switch flows dan randomizer flow.
- Pecah `useFolders.ts` menjadi barrel infra tipis di atas `folderCache.ts` dan `useFolderCoreMutations.ts`.
- Pecah `useObjects.ts` menjadi barrel infra tipis di atas `objectQueryCache.ts`, `useObjectQueries.ts`, dan `useObjectMutations.ts`.

## Impacted Files

- `src-tauri/src/domain/workspace.rs` (modified)
- `src-tauri/src/commands/app/workspace_cmds.rs` (modified)
- `src-tauri/src/services/mods/core_ops.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (modified)
- `src/types/workspace.ts` (modified)
- `src/lib/bindings.ts` (modified)
- `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.ts` (modified)
- `src/features/folder-grid/ObjectConflictModal.tsx` (modified)
- `src/features/randomizer/RandomizerModal.tsx` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/hooks/folderCache.ts` (added)
- `src/hooks/useFolderCoreMutations.ts` (added)
- `src/hooks/useFolders.ts` (rewritten)
- `src/hooks/objectQueryCache.ts` (added)
- `src/hooks/useObjectQueries.ts` (added)
- `src/hooks/useObjectMutations.ts` (added)
- `src/hooks/useObjects.ts` (rewritten)

## Goal

Switch enabled/disabled sekarang punya command backend tunggal untuk runtime workspace, dan barrel `useFolders` / `useObjects` tinggal menjadi surface infra stabil tanpa orchestration runtime besar.

## Impact

- Duplicate resolution, force enable, dan object/mod toggle sekarang melewati contract backend yang sama.
- Runtime switch frontend lebih simpel karena tidak lagi bercampur antara `toggleMod` dan `enableOnlyThis`.
- `useFolders.ts` dan `useObjects.ts` lebih kecil dan lebih mudah dipelihara; helper cache/mutation dipisah ke modul khusus.
- Tidak ada migration DB.

## Notes

- Command lama `toggle_mod` dan `enable_only_this` masih dipertahankan untuk compatibility, tetapi jalur switch runtime utama sudah pindah ke `execute_workspace_switch`.
- `check_duplicate_enabled` masih hidup sebagai utility non-runtime; duplicate resolution runtime utama tidak lagi bergantung padanya.
