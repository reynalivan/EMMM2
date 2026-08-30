# Refactor: Flatten Adapters Layer (Phase H - Part 2)

## Context
Menghilangkan struktur hierarki ganda `adapters/inbound` dan `adapters/outbound` agar lebih selaras dengan prinsip *Ports & Adapters* murni (menamai adapter berdasarkan teknologinya).

## Changes
- `adapters/inbound` diubah menjadi `adapters/tauri`.
- `adapters/outbound/sqlite` diubah menjadi `adapters/sqlite`.
- Meng-update ratusan rujukan `crate::modules::*::adapters::inbound::*` menjadi `adapters::tauri::*` di seluruh *source code* dan tests.
- Meng-update file `mod.rs` di setiap folder `adapters`.

## Impacted Files
- `src-tauri/src/modules/*/adapters/*` (restrukturisasi folder)
- `src-tauri/src/lib.rs` (tauri wiring)
- `src-tauri/tests/` (integration tests imports)

## Goal
Hierarki internal setiap modul kini lebih dangkal (flat) dan penamaannya eksplisit menunjukkan batas infrastruktur (`tauri` untuk IPC frontend, `sqlite` untuk database).

## Impact
- `cargo check` berhasil tanpa *error*.
- Tidak ada perubahan logika.
