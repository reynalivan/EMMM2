# Implementation Plan: Active Shader Conflict Notice

## Goal

Pastikan conflict notice hanya merepresentasikan file yang benar-benar aktif menurut runtime GIMI/3DMigoto, selalu segar setelah mutation, dan menampilkan alasan konflik yang dapat ditindaklanjuti.

## Runtime invariants

1. Hanya mod root yang enabled di database yang boleh dipindai oleh active-conflict query.
2. Setiap directory descendant dengan prefix case-insensitive `DISABLED` harus dipangkas dari traversal, sesuai `exclude_recursive = DISABLED*`.
3. Discovery INI dan ShaderFixes bersifat recursive tanpa batas kedalaman buatan, tidak mengikuti symlink, dan mengabaikan `desktop.ini`.
4. Semua mutation yang dapat mengubah file/runtime mod harus menginvalidasi query `conflicts`.
5. Dismissal berlaku pada signature conflict batch saat itu; batch yang berubah harus dapat memunculkan notice baru.
6. UI membedakan resource-hash, shader-hash, dan shader-replacement conflict dan tetap menunjukkan certainty/evidence yang tersedia.

## Phase 1 — Backend characterization and discovery

- Tambahkan test yang membuktikan nested `DISABLED*` INI dan shader replacement tidak dihitung.
- Tambahkan test yang membuktikan INI lebih dalam dari tiga directory tetap ditemukan.
- Buat satu traversal runtime-aware yang dipakai oleh active conflict discovery.
- Hilangkan fixed-depth traversal pada jalur active conflict tanpa mengubah scanner umum yang tidak terkait.

## Phase 2 — Conflict metadata alignment

- Parse key resmi `match_priority` dan pertahankan compatibility untuk data lama bila aman.
- Validasi panjang hash sesuai namespace: TextureOverride 8 hex dan ShaderOverride 16 hex.
- Tambahkan regression tests untuk aturan tersebut.
- Jangan mengklaim draw-call equivalence yang tidak tersedia dari evidence parser; conflict yang bergantung konteks tetap ditandai potential.

## Phase 3 — Freshness and dismissal

- Invalidasi conflict query setelah INI write, import, dan watcher `runtime_file_changed`.
- Gunakan mutation refresh class yang mencakup conflict state untuk import.
- Ganti boolean dismissal dengan conflict signature yang stabil dan deterministik.
- Tambahkan tests untuk tiap invalidation path dan perubahan batch sesudah dismissal.

## Phase 4 — Notice clarity

- Ubah copy menjadi enabled-mod conflicts.
- Tampilkan badge jenis conflict dan evidence yang relevan pada modal.
- Tambahkan/update translation EN, ID, dan ZH tanpa hardcoded production copy.

## Verification

- Targeted Rust tests untuk metadata/hash conflict.
- Targeted Vitest untuk LaunchBar, preview INI mutation, watcher refresh, import refresh, dan modal.
- `cargo fmt --check`, relevant `cargo test`, `pnpm lint`, `pnpm i18n:lint`, `pnpm build`, dan `git diff --check`.
- Catat kegagalan baseline yang tidak terkait secara eksplisit; jangan memperluas scope untuk memperbaikinya.

## Non-goals

- Meniru seluruh interpreter 3DMigoto atau mengevaluasi semua runtime condition/draw-call state.
- Mengubah database authority, reconcile architecture, atau scanner umum di luar active-conflict path.
- Refactor visual LaunchBar yang tidak diperlukan untuk kejelasan notice.
