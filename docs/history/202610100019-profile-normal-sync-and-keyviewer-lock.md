# Profile normal sync dan lock KeyViewer

## Context

Pengguna meminta bukti impact klasifikasi satu root besar dan traversal KeyViewer
sebelum menyusun plan optimasi.

## Changes

- Tambahkan dua ignored benchmark dan helper khusus `cfg(test)` pada fixture
  sintetis, tanpa perubahan perilaku produksi.
- Susun plan child scheduling terbatas dan prepared discovery KeyViewer dengan
  gate coverage, revalidation, dan recovery.

## Impacted Files

- `src-tauri/src/modules/reconciliation/application/disk_reconcile/census_tests.rs`
  (profiling helper/benchmark ditambahkan pada test file yang sudah ada).
- `src-tauri/src/modules/system/application/app/runtime_preflight_tests.rs`
  (profiling helper/benchmark ditambahkan pada test file yang sudah ada).
- `docs/plans/startup-performance/normal-sync-keyviewer-plan-20261010.md` (added).
- `docs/history/202610100019-profile-normal-sync-and-keyviewer-lock.md` (added).

## Goal

Prioritas optimasi didukung pengukuran dan batas keselamatan yang dapat diuji.
Implementasi produksi belum dilakukan.

## Impact

- Klasifikasi full: median 503,378 → 244,171 ms; scoped: 487,106 → 246,437 ms.
  Seluruh field/order/identity projection identik; kontrol banyak root hanya noise.
- Traversal aktual di bawah guard menahan revision/activation waiter sekitar
  257 ms pada 20.504 entry tanpa duplikat. Simulasi di luar lock menguji stale
  rejection, belum publikasi sukses.
- Pruning DISABLED naif ditolak: fixture aktif melambat sekitar 25%.
- Dua benchmark lulus; scoped rustfmt, diff check, dan strict Clippy lib/tests
  lulus. Pengukuran debug/cache hangat bukan total startup atau native UI.
- GIMI asli tidak diakses. Tidak ada perubahan data/config aplikasi atau native
  E2E pada sesi audit ini.
