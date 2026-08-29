# Checklist Disk–DB–Watcher–UI Convergence

## Phase 1 — Critical scanner safety

- [x] T1 Tambahkan characterization/failure-injection tests untuk seluruh invariant yang gagal
- [x] T2 Validasi configured root, preflight, dan lock pada Deep Match commands
- [x] T3 Tambahkan local move journal, rollback, error propagation, dan final reconcile scanner
- [x] Checkpoint A: scanner conflict/failure tests hijau tanpa SQL 1555 atau silent drift

## Phase 2 — Source path dan watcher

- [x] T4 Tolak perubahan existing `mod_path` melalui generic backend settings
- [x] T5 Route Game Settings ke inspect/apply Source Recovery dan await modal save
- [x] T6 Serialisasi watcher lifecycle, stale cleanup guard, dan validate start boundary
- [x] Checkpoint B: active path, config, watcher, DB, dan grid memakai source yang sama

## Phase 3 — Mutation bypass

- [x] T7 Tambahkan preflight/lock/reconcile dan partial failure contract pada Trash; in-app restore ditolak eksplisit karena restore Windows Recycle Bin tidak tersedia
- [x] T8 Reconcile sebelum collection recovery/capture dan setelah recovery apply

## Phase 4 — Projection dan cache

- [x] T9 Commit core rows dan affected runtime projection dalam transaksi yang sama
- [x] T10 Pertahankan collection/runtime side-effect dirty sampai sukses dan refresh FE dari terminal result

## Phase 5 — Import dan metadata

- [x] T11 Propagasikan essential browser import writes dan cleanup staging secara contained
- [x] T12 Baca metadata rollback snapshot di dalam lock dan rollback optimistic frontend settings
- [x] Checkpoint C: injected transient failures tetap berakhir pada convergence

## Phase 6 — Regression dan performance

- [x] T13 Lengkapi regression matrix untuk online/offline, in-app/external, parent/child, bulk, organize/category, collections, import/archive, thumbnail/INI/info
- [x] T13 E2E: conflict rename, Trash resolution, startup offline changes, dan rapid watcher switch
- [x] T14 Ukur scan count/latency dengan fixture 10k folders
- [x] T14 Hilangkan duplicate startup/watcher full scans tanpa mengurangi conflict detection

## Phase 7 — Cleanup dan verification

- [x] T15 Selesaikan relevant exhaustive-deps/unused/size warnings tanpa broad refactor
- [x] T15 Verifikasi invoke handler, allowlist, generated bindings, dan command docs
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets --all-features`
- [x] `cargo test`
- [x] `pnpm test -- --run`
- [x] `pnpm lint`
- [x] `pnpm i18n:lint`
- [x] `pnpm build`
- [x] Targeted WebdriverIO E2E
- [x] `git diff --check`
- [x] Read-only final correctness/security/regression review
- [x] Catat hasil, limitation, dan benchmark before/after
