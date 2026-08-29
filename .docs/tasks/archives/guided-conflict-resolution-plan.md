# Implementation Plan: Guided Conflict Resolution

## Overview

Ubah modal enabled-mod conflicts menjadi workflow resolusi whole-mod yang aman. Pengguna memilih mod yang dipertahankan atau dinonaktifkan, meninjau dampak global, lalu menjalankan existing bulk toggle dengan partial-failure handling dan refresh konflik.

## Architecture decisions

- Reuse `useBulkToggle`; tidak menambah command/backend baru.
- Simpan keputusan secara global berdasarkan canonical mod path agar satu mod tidak memiliki status berbeda di dua conflict group.
- Pisahkan kalkulasi keputusan/impact menjadi pure TypeScript utilities agar aturan graph konflik dapat diuji tanpa UI.
- Confirmation menjadi view kedua dalam dialog yang sama; hindari nested modal.
- First release hanya whole-mod disable dan open folder. Tidak mengedit INI, priority, section, atau ShaderFixes.

## Task 1 — Decision model

**Description:** Buat pure utilities untuk stable conflict key, memilih winner, menandai mod disable, dan menghitung resolved/unresolved impact.

**Acceptance criteria:**

- Memilih satu winner menandai participant lain sebagai disable.
- Keputusan mod berlaku konsisten pada seluruh conflict group.
- Impact menghasilkan unique disable paths dan jumlah resolved/unresolved yang benar.

**Verification:** Targeted Vitest utility suite.

**Dependencies:** None.

## Task 2 — Guided modal flow

**Description:** Tambahkan keep/disable/open actions, clear choices, impact summary, dan inline review view ke `ConflictModal`.

**Acceptance criteria:**

- Dialog open tidak menjalankan mutation atau preselect winner.
- Review hanya aktif bila minimal satu mod dipilih untuk disable.
- Potential conflict tetap meminta keputusan manual.

**Verification:** Targeted `ConflictModal` component tests.

**Dependencies:** Task 1.

## Task 3 — Mutation and recovery

**Description:** Hubungkan confirmation ke `useBulkToggle`, pertahankan failed paths untuk retry, bersihkan successful decisions, dan tampilkan command-level error tanpa menutup dialog.

**Acceptance criteria:**

- Full success mengosongkan submitted choices dan kembali ke conflict view.
- Partial failure hanya mempertahankan failed paths beserta error.
- Duplicate submission dan dialog close diblokir saat mutation pending.

**Verification:** Full/partial/error component tests and existing bulk-toggle tests.

**Dependencies:** Task 2.

## Task 4 — Copy and verification

**Description:** Tambahkan translation EN/ID/ZH, perbarui history, dan jalankan scoped/full verification yang tersedia.

**Acceptance criteria:**

- Tidak ada hardcoded user-facing copy baru.
- Keyboard-accessible buttons memiliki label dan disabled state yang benar.
- Targeted tests, scoped ESLint, i18n lint, formatting, dan diff check bersih.

**Dependencies:** Tasks 1–3.

## Risks

- Satu mod terlibat di beberapa grup: global path-keyed decisions dan unresolved recomputation mencegah kontradiksi tersembunyi.
- Backend mengembalikan success memakai renamed path: successful originals dihitung dari submitted paths dikurangi failure paths.
- Worktree memiliki unrelated compile drift: hasil full verification dilaporkan terpisah tanpa memperluas scope.
