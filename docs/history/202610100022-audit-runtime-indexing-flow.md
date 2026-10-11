# Audit runtime indexing dan settlement

## Context

Pengguna meminta rekomendasi indexing yang lebih cepat/stabil dan simplified
architecture tanpa overengineering. Scope sesi ini audit/plan, bukan patch produksi.

## Changes

- Dokumentasikan race source-confirmed: direct publication dapat supersede queued
  revision tanpa queue generation baru, lalu worker mengakui skipped work sebagai
  success. Runtime reproduction masih menjadi gate implementasi.
- Prioritaskan batch harvest invalidation, metadata preparation, verified byte/
  digest reuse, dan penyatuan admission lewat existing owner.
- Tambahkan std-only scratch probe untuk repeated-retain versus batch invalidation;
  model memeriksa exact final-cache parity dan unrelated game tetap utuh.

## Impacted Files

- `docs/plans/startup-performance/runtime-indexing-audit-20261010.md` (added).
- `docs/history/202610100022-audit-runtime-indexing-flow.md` (added).
- `.audit-output/runtime-indexing-audit-20261010/` (generated source, executable,
  metadata, benchmark log/build digest dan projection evidence summary).

## Goal and Impact

Plan terurut dari koreksi settlement ke pengurangan pekerjaan berulang.
Model 2.048-mod root: 4.096→1 retains, median126,125→0,384ms; one-mod128→108us.
Angka tersebut bukan gain produksi/end-to-end. Existing projection snapshot
metadata~72–98ms versus read-model~2–3ms menjadi dasar prioritas.

Model parity dan warnings-as-errors compile/rustfmt lulus; source review
independen mengonfirmasi reachability race, belum native regression. Tidak ada
perubahan kode produksi, dependency, schema, config aplikasi atau akses GIMI asli.

[Audit, risiko dan acceptance plan](../plans/startup-performance/runtime-indexing-audit-20261010.md).
