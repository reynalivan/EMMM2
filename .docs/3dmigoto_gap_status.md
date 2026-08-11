# Status Gap dan Remediation 3DMigoto EMMM2NEW

> Status per 2026-08-11. Dokumen ini hanya melacak gap, penyelesaian, bukti, dan pekerjaan validasi. Panduan teknis utama berada di [`3dmigoto_context_knowledge.md`](./3dmigoto_context_knowledge.md).

## Ringkasan

- Fase implementasi T1–T10: **selesai**.
- Tabel audit aktual: **23 ID**; roadmap awal keliru menyebut 22.
- Gap kode berstatus unresolved: **0**.
- Validasi runtime manual yang masih disarankan: smoke test visual pada package/game nyata.
- Capability yang belum tersedia: EFMI KeyViewer, karena text API resmi belum terverifikasi.
- Kegagalan test di luar scope: DAL ratchet pada `services/collection_service/crud.rs`.

## Status fase

| Fase | Cakupan | Status |
|---|---|---|
| T1 | Runtime path dan `DISABLED*` contract | Selesai |
| T2 | Recursive INI discovery | Selesai |
| T3 | `[Hunting] reload_fixes` discovery/replay | Selesai |
| T4 | Effective-enabled projection | Selesai |
| T5 | Lossless INI reader | Selesai |
| T6 | Stale-safe recoverable writer | Selesai |
| T7 | Portable deterministic KeyViewer | Selesai |
| T8 | Archive classification dan conflict evidence | Selesai |
| T9 | Switch recovery dan runtime-status UX | Selesai |
| T10 | Full verification dan dokumentasi | Selesai |

## Matriks gap

| ID | Masalah awal | Status | Bukti utama |
|---|---|---|---|
| INI-01 | INI hanya ditemukan satu level | Resolved | Recursive deterministic walker dipakai editor dan KeyViewer |
| INI-02 | Shift-JIS ditulis ulang sebagai UTF-8 | Resolved | Encoding/BOM asal dipertahankan; unrepresentable write ditolak |
| INI-03 | Mixed newline dan final newline hilang | Resolved | Terminator disimpan per baris |
| INI-04 | Qualified variable dan repeated key/back hilang | Resolved | Ordered structured indices di atas raw lines |
| INI-05 | Section case/error discovery tidak konsisten | Resolved | Case-insensitive identity dan entry error propagation |
| INI-06 | Mod Include/Key/CommandList/ShaderRegex ditolak | Resolved | Runtime section family classifier + fixtures |
| WRT-01 | Stale editor dapat menimpa external edit | Resolved | BLAKE3 source fingerprint check |
| WRT-02 | Windows replace dapat meninggalkan target hilang | Resolved | Recovery rename dan auto-restore |
| WRT-03 | Fixed temp/concurrent write/satu backup | Resolved | OperationLock, unique temp, tiga backup generations |
| KV-01 | Reload dicari dari `[Key*]` fiktif | Resolved | Parser `[Hunting] reload_fixes` |
| KV-02 | Grammar replay tidak cocok dengan `no_* VK_*` | Resolved | Whitespace/`+`, VK, positive modifier, negative constraint parser |
| KV-03 | Lokasi `d3dx.ini` hanya dekat game exe | Resolved | Parent configured Mods, lalu executable fallback |
| KV-04 | Overlay hardcode legacy GIMI help API | Resolved | `GIMIv8`/`SRMIv1`/`WWMIv1`/`ZZMIv1` renderer |
| KV-05 | Resource path KeyViewer bercampur root/config-relative | Resolved in code | Semua path relatif terhadap `.emmm_data/KeyViewer.ini` |
| KV-06 | Raw DB status mengabaikan disabled ancestor | Resolved | Effective-enabled predicate pada seluruh path component |
| KV-07 | Shader64 diperlakukan sebagai resource32 | Resolved | KeyViewer hanya TextureOverride 8-hex; typed conflict scope untuk shader |
| KV-08 | Duplicate score dan sentinel ambigu | Resolved | Dedupe, threshold konservatif, unique-sentinel requirement |
| KV-09 | Artifact error/cleanup/dead fallback tersembunyi | Resolved | Staging, recoverable swap, propagated degraded warning |
| KV-10 | Conflict report tanpa applicability/provenance | Resolved | Kind, source, namespace, condition, priority, index, stage, certainty |
| SW-01 | Semantik `DISABLED` berbeda frontend/backend/runtime | Resolved | Case-insensitive `DISABLED*` predicate bersama |
| SW-02 | Default F6/F8 bertabrakan dengan package | Resolved | Default `Ctrl+F6`/`Ctrl+F8` dan reserved-key warning |
| SW-03 | Rollback tidak lengkap dapat membuat DB drift | Resolved | Full reconcile + recovery warnings |
| SW-04 | Disk applied disamakan dengan runtime loaded | Resolved | Explicit `ReloadRequired`; reload sukses hanya setelah key replay |

## Bukti verifikasi terakhir

- Rust library: 647 passed, 1 ignored.
- Targeted Rust suites untuk INI, KeyViewer, hotkeys, conflict, archive, dan rollback: passed.
- Frontend: 129 test files; 694 passed, 1 skipped.
- TypeScript, Rust build, Vite production build, formatter, targeted ESLint, dan `git diff --check`: passed.
- Full Rust integration audit masih gagal pada DAL ratchet lama: raw SQL di `src-tauri/src/services/collection_service/crud.rs`. File tersebut tidak disentuh remediation ini.

## Yang masih perlu divalidasi

Ini bukan gap implementasi tersisa, tetapi boundary yang tidak dapat dibuktikan penuh oleh unit test:

1. Smoke test visual KeyViewer pada GIMI, WWMI, SRMI, dan ZZMI nyata: font/API, path resource, flicker 1,5 detik, character swap, dan camera movement.
2. Smoke test reload: UI mutation harus meminta reload; hotkey in-game harus benar-benar mengirim binding hasil `[Hunting]` ketika game fokus.
3. EFMI tetap unsupported sampai namespace/text API resmi ditemukan dan diverifikasi. Jangan fallback ke API package lain.
4. Refresh snapshot repository bila package/launcher upstream berubah; commit sumber yang dipakai tercatat di knowledge base.

## Riwayat implementasi

- [`history/202608100001-3dmigoto-gap-remediation.md`](./history/202608100001-3dmigoto-gap-remediation.md)
- [`history/202608110001-3dmigoto-doc-separation.md`](./history/202608110001-3dmigoto-doc-separation.md)
- [`../tasks/todo.md`](../tasks/todo.md)
