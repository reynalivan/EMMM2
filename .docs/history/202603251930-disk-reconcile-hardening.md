# Disk Reconcile Hardening

## Context

Disk Reconcile sudah terpasang di trigger utama, tetapi backend masih setengah bergantung ke jalur lama:
- watcher batch masih menyiapkan mutasi DB sendiri
- reconcile inti masih memakai engine Deep Match Scanner (`scan_preview` / `commit_scan_results`)

## Changes

- Watcher batch di-hardening menjadi helper disk-only.
  - Sebelum: membawa jalur mutasi DB langsung.
  - Sesudah: hanya kumpulkan `changed_paths` dan rename hints untuk Disk Reconcile.
- Reconcile inti diganti ke projection engine disk-first.
  - Sebelum: memakai `MasterDb::from_json("[]")`, `scan_preview`, dan `commit_scan_results`.
  - Sesudah: scan folder root/object-mod langsung dari disk lalu reconcile ke `objects` dan `mods`.
- Rename/move healing tetap dipertahankan di gateway yang sama.
  - Watcher rename hints sekarang dipakai di dalam Disk Reconcile untuk heal object/mod path dan collection path.
- Helper runtime dipisah dari scanner sync helper.
  - Stable mod id, status disabled, corridor classification, dan `info.json` metadata sekarang hidup di helper Disk Reconcile sendiri.

## Impacted Files

- `src-tauri/src/services/disk_reconcile/helpers.rs` (added)
- `src-tauri/src/services/disk_reconcile/mod.rs` (modified)
- `src-tauri/src/services/disk_reconcile/orchestrator.rs` (modified)
- `src-tauri/src/services/disk_reconcile/reconcile.rs` (modified)
- `src-tauri/src/services/disk_reconcile/watcher_batch.rs` (modified)

## Goal

Disk Reconcile sekarang menjadi jalur backend tunggal yang:
- membaca kenyataan di disk
- memperbarui projection runtime DB
- menjaga object tetap `Other` sampai Deep Match Scanner dijalankan eksplisit

## Impact

- Watcher tidak lagi menjadi dual-writer DB sebelum reconcile utama.
- Disk Reconcile tidak lagi bergantung ke preview/commit pipeline milik Deep Match Scanner.
- Rename/move/status sync untuk runtime path tetap terjaga lewat satu gateway yang sama.
- Tidak ada breaking change ke trigger FE atau public command.

## Notes

- Reconcile runtime saat ini sengaja depth-1/depth-2 sesuai domain object root + mod child folder.
- `info.json` dipakai untuk runtime metadata `actual_name` dan `is_safe` tanpa menarik canonical matching ke domain ini.
