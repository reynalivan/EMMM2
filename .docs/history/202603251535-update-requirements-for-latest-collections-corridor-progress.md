# Update Requirements for Latest Collections and Corridor Progress

## Context

Beberapa perubahan runtime terbaru di collections, switch preview, unsaved naming, dan preview tree sudah berjalan di code tetapi requirement docs belum ikut diperbarui.

## Changes

- Memperbarui `req-31-collections` agar sesuai dengan perilaku terbaru:
  - source of truth nama unsaved
  - self-apply guard di topbar
  - delete active collection langsung kembali ke unsaved aktif
  - preview tree terminal semantics untuk flat/modpack/variant
  - inactive-container section terpisah
  - snapshot metadata persistence untuk preview tree
- Memperbarui `req-30-privacy-safe-mode` agar restore corridor memakai resolusi `active -> unsaved -> system fallback`, serta menyamakan label unsaved di switch dialog.
- Memperbarui `req-11-folder-listing` untuk menegaskan warning corrupt 0 KB INI dan reuse metadata klasifikasi oleh collection preview.

## Impacted Files

- `.docs/history/202603251535-update-requirements-for-latest-collections-corridor-progress.md` (added)
- `.docs/requirements/req-11-folder-listing.md` (modified)
- `.docs/requirements/req-30-privacy-safe-mode.md` (modified)
- `.docs/requirements/req-31-collections.md` (modified)

## Goal

Dokumentasi requirement kembali selaras dengan perilaku sistem terbaru di collections, safe-mode corridor switch, dan preview tree semantics.

## Impact

- Tidak ada perubahan runtime atau schema.
- Mengurangi gap antara implementasi dan requirement untuk task berikutnya.
