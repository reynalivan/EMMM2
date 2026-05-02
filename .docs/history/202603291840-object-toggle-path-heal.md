# Object Toggle Path Heal

## Context

Object enable/disable dari ObjectList masih bisa gagal dengan `os error 2` ketika `objects.folder_path` di DB stale setelah refactor dan migrasi runtime baru.

## Changes

- Menambahkan resolver path object yang lebih defensif di command workspace switch.
- Backend sekarang mencoba:
  - path object dari DB,
  - fallback candidate dari nama object,
  - scan root Mods dengan normalized name.
- Jika menemukan folder fisik yang benar, backend langsung heal `objects.folder_path`, child `mods.folder_path`, dan referensi collection sebelum toggle dilanjutkan.

## Impacted Files

- `src-tauri/src/commands/app/workspace_cmds.rs` (modified)

## Goal

Toggle enable/disable object dari ObjectList tetap bekerja walau DB path object sempat stale, tanpa perlu workaround di FE.

## Impact

- Mengurangi kegagalan `Path does not exist or invalid` pada object switch.
- Healing path hanya jalan saat target object path di DB tidak cocok dengan disk.
- Tidak mengubah boundary Disk Reconcile vs workspace switch; ini hanya hardening untuk path resolution.

## Notes

- Jika folder fisik object memang benar-benar tidak ada di disk, command tetap gagal eksplisit.
