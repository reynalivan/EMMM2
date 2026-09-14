# Storage Optimizer Dedup: Execution Checklist

**Status:** Implemented safety-critical scope; semantic enrichment and benchmarks remain.

## Contract dan fixtures

- [x] SOD-01.1 Bedakan logical unit, orchestrated subvariant, toggle variant, exact copy, shared asset, related variant, dan runtime conflict di Req 32.
- [x] SOD-01.2 Dokumentasikan semantics merger GIMI dan strong/weak orchestration evidence.
- [ ] SOD-01.3 Tambahkan merged, nested, disabled-child, false-merged-name, exact-copy, dan head/tail-collision fixtures.
- [x] SOD-01.4 Reproduksi child-row false duplicate dengan regression test merah.

## Ownership

- [ ] SOD-02.1 Gunakan satu authoritative disk inventory untuk progress dan pipeline.
- [ ] SOD-02.2 Tambahkan `LogicalModUnit` dan owner-root mapping.
- [x] SOD-02.3 Collapse descendants dari terminal ModPackRoot/FlatModRoot/VariantContainer.
- [x] SOD-02.4 Hapus immediate-parent-only filter.
- [ ] SOD-03.1 Detect orchestration dari INI semantics, bukan filename saja.
- [x] SOD-03.2 Pertahankan ownership saat source INI child sudah `DISABLED` atau tidak ada.
- [x] SOD-03.3 Pastikan internal `.emmm-*`, hidden stages, dan symlink bukan candidate.

## Exact hashing dan relations

- [x] SOD-04.1 Samakan ignore policy.
- [x] SOD-04.2 Jadikan partial BLAKE3 hanya prefilter.
- [x] SOD-04.3 Full-hash seluruh surviving match.
- [x] SOD-04.4 Bentuk content multiset dan path-aware manifests.
- [ ] SOD-04.5 Rekam bounded unreadable-file warnings.
- [ ] SOD-05.1 Tambahkan typed relation classes.
- [x] SOD-05.2 Group exact copies tanpa transitive similarity union.
- [x] SOD-05.3 Hapus transitive max-score certainty.
- [x] SOD-05.4 Jadikan name/structure signal explanation-only untuk izin destruktif.

## 3DMigoto semantic matching

- [x] SOD-06.1 Reuse encoding decoder untuk signature INI.
- [x] SOD-06.2 Hapus 200-line cap dan comment-as-header signal.
- [ ] SOD-06.3 Extract typed kind/hash/namespace/index/priority/condition/order.
- [ ] SOD-07.1 Extract cycle values, branches, command lists, slots, and resource bindings.
- [ ] SOD-07.2 Include `checktextureoverride` dependencies.
- [ ] SOD-07.3 Classify same-target/different-resource sebagai related/conflict.
- [ ] SOD-07.4 Pastikan semantic score tidak memberi destructive permission.

## Lifecycle dan persistence

- [x] SOD-08.1 Tambahkan `Failed` terminal event.
- [ ] SOD-08.2 Emit progress aktual dan terminal event tepat sekali.
- [ ] SOD-08.3 Scope running/cancel/report state per game/job.
- [x] SOD-09.1 Persist jobs/groups/members atomically memakai schema existing.
- [x] SOD-09.2 Query latest completed report per game.
- [x] SOD-09.3 Jangan ganti report sukses dengan job failed/cancelled.
- [x] SOD-09.4 Verifikasi report bertahan setelah restart.

## Resolution safety dan UX

- [x] SOD-10.1 Reject Trash/hardlink untuk relation non-exact.
- [x] SOD-10.2 Revalidate full hashes dan path-aware manifest sebelum mutation.
- [ ] SOD-10.3 Validate containment, same volume, operation lock, watcher suppression, rollback, dan reconcile.
- [x] SOD-10.4 Uji file berubah setelah scan dan hardlink rollback.
- [x] SOD-11.1 Refresh report hanya pada `Finished`.
- [ ] SOD-11.2 Pisahkan exact duplicates, shared assets, dan related mods di UI.
- [x] SOD-11.3 Sembunyikan destructive controls pada setiap hasil non-exact.
- [ ] SOD-11.4 Tampilkan logical-unit, verified-byte, skipped-file, dan reclaim metrics.
- [ ] SOD-12.1 Tambahkan path review dan reason untuk disabled actions.
- [ ] SOD-12.2 Lengkapi EN/ID/ZH translations.

## Verification

- [x] Focused Rust dedup/inventory/resolver tests pass (40 dedup + 6 classifier).
- [x] Focused frontend hook/report/resolution tests pass.
- [ ] `cargo fmt --check` pass. Blocked by formatting in unrelated collections/reconcile/watcher files; scoped changed files pass.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` pass. Library target passes; all-targets is blocked by an unrelated reconcile test lint.
- [x] `cargo test` pass atau unrelated blockers dicatat dengan bukti.
- [x] `pnpm test -- --run` pass atau unrelated blockers dicatat dengan bukti.
- [x] `pnpm lint` dan `pnpm i18n:lint` pass (lint: 0 error, existing max-lines warnings).
- [x] `pnpm build` dan generated bindings check pass.
- [ ] 1k-file performance dan 10k-root capacity benchmark direkam.
- [ ] Cancellation mencapai terminal state dalam target.
- [ ] Manual smoke memakai copy library, bukan live GIMI Mods root.
- [x] `git diff --check` pass.
- [x] History log dan remaining limitations ditulis.
