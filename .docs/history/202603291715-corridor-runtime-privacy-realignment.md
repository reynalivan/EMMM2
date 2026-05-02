# Corridor runtime privacy realignment

## Context

Workspace runtime hasil refactor masih membiarkan `FolderGrid` dan `Preview` menerima mod lintas corridor. Main grid lalu cuma memblur item lawan corridor, padahal requirement mewajibkan backend exclusion sebagai jalur utama.

## Changes

- Runtime `WorkspaceViewModel` explorer sekarang corridor-filtered backend-side sebelum dipetakan ke node runtime.
- Conflict groups di explorer ikut dipruning agar tidak mereferensikan child yang sudah tersembunyi oleh corridor filter.
- Preview runtime tidak lagi mempertahankan `selected_mod_path` lawan corridor; stale selection sekarang jatuh ke `null`.
- Corridor switch frontend sekarang membersihkan selection runtime yang corridor-sensitive, termasuk `selectedModPath` dan explorer path.
- Komentar UI grid diperbarui untuk menegaskan blur hanyalah leak guard, bukan perilaku utama.
- Flow/privacy/collections docs diperbarui agar selaras dengan contract runtime akhir.

## Impacted Files

- `.docs/flow.md` (modified)
- `.docs/requirements/req-30-privacy-safe-mode.md` (modified)
- `.docs/requirements/req-31-collections.md` (modified)
- `src/features/collections/hooks/useCorridorSwitch.ts` (modified)
- `src/features/collections/hooks/useCorridorSwitch.test.tsx` (modified)
- `src/features/folder-grid/FolderCard.tsx` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src-tauri/src/commands/folder_grid/helpers.rs` (modified)
- `src-tauri/src/commands/folder_grid/mod.rs` (modified)
- `src-tauri/src/services/explorer/helpers.rs` (modified)
- `src-tauri/src/services/workspace_service.rs` (modified)

## Goal

Main workspace grid dan preview sekarang hanya bekerja pada corridor aktif, sementara `ObjectList` tetap menampilkan semua object dengan count/status per corridor.

## Impact

- Menghilangkan leak utama di `FolderGrid` dan `Preview` setelah switch corridor.
- Conflict dialog/grid state tidak lagi membawa member yang sudah disembunyikan oleh corridor filtering.
- Tidak ada migration DB atau perubahan public UI flow baru.
- Rust unit-test binary masih mengalami crash loader Windows saat dieksekusi di environment ini, tetapi compile test target dan test frontend yang disentuh lulus.

## Notes

- Filtering runtime explorer sengaja dibuat eksplisit dari `safe_mode` input agar contract `WorkspaceViewModel` tidak bergantung pada masking frontend.
