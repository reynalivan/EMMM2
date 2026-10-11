# Optimize normal sync dan traversal KeyViewer

## Context

Implementasi plan berbasis benchmark yang disetujui pengguna, dengan mitigasi
risiko tanpa menambah state atau service.

## Changes

- Normal projection membagi spare child budget delapan; root scheduling, strict
  collector, urutan/identity, recursion serial dan error contract tetap dipakai.
- KeyViewer melewati direktori DISABLED sebelum descent dengan local-name check.
  Scan tetap di bawah publication guards karena secondary-root coverage belum
  terbukti. Journal/rollback dan ownership filter tetap dipakai.
- Tambahkan risk tests, benchmark produksi versus serial reference, dan native
  full/scoped sync/duplicate ownership spec.

## Impacted Files

- Backend: `src-tauri/src/modules/reconciliation/application/disk_reconcile/{disk_snapshot.rs,census_tests.rs}`;
  `src-tauri/src/modules/system/application/app/{post_apply.rs,runtime_preflight_tests.rs}`.
- Native: `tests/e2e/specs/normal-sync-keyviewer.e2e.ts` (added).
- Docs: `src-tauri/AGENT.md`;
  `docs/plans/startup-performance/normal-sync-keyviewer-{plan,execution}-20261010.md`;
  this history file.
- Generated evidence: `.audit-output/normal-sync-keyviewer-implementation-20261010/`
  source/build hashes, RED log, validation metadata, runner, benchmark logs/JSON
  and private executable snapshots.

## Goal and Impact

Satu root besar memakai concurrency yang tersedia; KeyViewer tidak menelusuri
subtree yang dinonaktifkan. Tidak ada dependency/schema/cache/worker baru.
Produksi synthetic median full 1.230 → 323 ms, scoped 1.506 → 348 ms.
Final KeyViewer mayoritas DISABLED 382 → 80 ms; fixture aktif 384 → 390 ms.
Potensi biaya aktif 1,5% diterima sebagai tradeoff untuk gain disabled sekitar
79%; dua run awal beririsan compiler dan disimpan terpisah. Tidak diklaim
sebagai gain total startup atau library real GB.

85 related Rust tests dan strict all-target Clippy lulus. Native build baru lulus
2/2 spec dan tiga restart; dua owned fixtures dibersihkan setelah shutdown.
Source terkait tetap cocok dengan snapshot. GIMI asli tidak diakses; aplikasi
terpasang tidak diganti. Full-suite terbaru module lain tetap milik chat terkait.

[Hasil dan batas verifikasi](../plans/startup-performance/normal-sync-keyviewer-execution-20261010.md).
