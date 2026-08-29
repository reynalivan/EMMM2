# Storage Optimizer Dedup: Implementation Plan

**Status:** Safety-critical implementation complete; follow-up enrichment remains in the checklist.
**Design:** [`.docs/tasks/archives/2026-08-28-storage-optimizer-dedup-design.md`](./2026-08-28-storage-optimizer-dedup-design.md)

## Outcome

Scanner bekerja pada logical mod units, mengabaikan subvariant internal, memisahkan exact-content duplicate dari semantic 3DMigoto relations, dan hanya memberi aksi storage setelah full-content verification.

## Guardrails

- Disk inventory adalah source of truth; DB hanya enrichment/persistence.
- Semua task production mengikuti red-green-refactor dan menjaga file di bawah batas repo.
- Tidak ada delete permanen; resolver harus rollback-safe dan diikuti reconcile.
- Tidak ada destructive action untuk `OrchestratedSubvariant`, `ToggleVariant`, `RelatedVariant`, atau `RuntimeConflict`.
- Existing unrelated changes di worktree tidak disentuh.

## Phase 0 — Contract dan characterization

### SOD-01: Selaraskan requirement dengan semantic merged mods

**Depends on:** none
**Files:**

- `.docs/requirements/req-32-dedup-scanner.md`
- `.docs/3dmigoto_context_knowledge.md`
- `src-tauri/src/services/scanner/dedup/tests/dedup_scanner_tests.rs`
- `src-tauri/src/services/scanner/dedup/tests/fixtures.rs` (new)

**Work:**

- Definisikan logical unit, orchestrated subvariant, toggle variant, exact copy, shared assets, dan runtime-related.
- Tambahkan fixture minimal dari pola merger: cycle variable, ordered branches, resource references, disabled child INIs, nested child, dan false-positive file bernama `merged.ini` tanpa orchestration.
- Tulis regression test yang membuktikan bug saat DB berisi root dan child rows.

**Done when:** test merah mereproduksi child/subvariant false duplicate dan requirement tidak lagi menyamakan similarity dengan exact identity.

## Phase 1 — Ownership dan authoritative inventory

### SOD-02: Bangun logical unit inventory

**Depends on:** SOD-01
**Files:**

- `src-tauri/src/services/scanner/core/walker.rs`
- `src-tauri/src/services/scanner/dedup/ownership.rs` (new)
- `src-tauri/src/services/scanner/dedup/mod.rs`
- `src-tauri/src/services/scanner/dedup/scanner.rs`
- `src-tauri/src/services/scanner/dedup/tests/ownership_tests.rs` (new)

**Work:**

- Jadikan hasil walker terminal sebagai satu-satunya daftar kandidat.
- Tambahkan `LogicalModUnit` dengan owner root, root kind, owned descendants, dan orchestration evidence.
- Map row DB ke unit berdasarkan resolved path; jangan membuat kandidat dari row DB.
- Hapus `apply_modpack_filter` immediate-parent karena ownership boundary menggantikannya.

**Done when:** root/child/nested child selalu collapse ke satu unit dan `total_folders` sama pada progress serta pipeline.

### SOD-03: Perkuat orchestration evidence tanpa bergantung nama file

**Depends on:** SOD-02
**Files:**

- `src-tauri/src/common/classifier.rs`
- `src-tauri/src/services/scanner/dedup/ownership.rs`
- `src-tauri/src/services/ini/document.rs`
- `src-tauri/src/common/tests/classifier_tests.rs`
- `src-tauri/src/services/scanner/dedup/tests/ownership_tests.rs`

**Work:**

- Kenali root orchestrator dari valid section + multi-child resource refs + cycle/branch evidence.
- Perlakukan filename `merged.ini` sebagai hint, bukan proof.
- Pertahankan terminal semantics walaupun child INIs sudah `DISABLED` atau child hanya berisi asset.

**Done when:** fixture merger nyata dikenali, sedangkan README/config yang kebetulan bernama merged tidak meng-claim child ownership.

## Phase 2 — Exact content safety

### SOD-04: Pisahkan prefilter hash dari full verification

**Depends on:** SOD-02
**Files:**

- `src-tauri/src/services/scanner/dedup/hashing.rs`
- `src-tauri/src/services/scanner/dedup/snapshot.rs`
- `src-tauri/src/services/scanner/dedup/scanner.rs`
- `src-tauri/src/services/scanner/dedup/tests/dedup_phase1_tests.rs`
- `src-tauri/src/services/scanner/dedup/tests/dedup_scanner_tests.rs`

**Work:**

- Terapkan ignore policy konsisten dan catat unreadable-file warning secara bounded.
- Gunakan size + partial BLAKE3 hanya untuk candidate reduction.
- Full-hash setiap surviving candidate sebelum exact classification.
- Bentuk content multiset manifest dan path-aware manifest; jangan key hash hanya dari relative path.

**Done when:** head/tail collision fixture ditolak dan exact class selalu membawa full-verification evidence.

### SOD-05: Ganti scoring/grouping dengan typed relation

**Depends on:** SOD-04
**Files:**

- `src-tauri/src/services/scanner/dedup/signals.rs`
- `src-tauri/src/services/scanner/dedup/grouping.rs`
- `src-tauri/src/types/dup_scan.rs`
- `src-tauri/src/services/scanner/dedup/tests/dedup_scanner_tests.rs`
- `src-tauri/src/services/scanner/dedup/tests/dedup_phase1_tests.rs`

**Work:**

- Tambahkan relation class `ExactCopy`, `SharedAssets`, `RelatedVariant`, dan `RuntimeConflict`.
- Group exact results memakai canonical manifest ID, bukan transitive similarity.
- Simpan pair/edge evidence untuk non-exact relations.
- Nama dan struktur menjadi explanation/ranking saja, bukan safety proof.

**Done when:** A~~B~~C tidak membuat exact group kecuali ketiganya mempunyai manifest ID identik.

## Phase 3 — 3DMigoto semantic signatures

### SOD-06: Reuse runtime-aware INI traversal dan decoder

**Depends on:** SOD-03
**Files:**

- `src-tauri/src/services/scanner/dedup/snapshot.rs`
- `src-tauri/src/services/scanner/conflict/hash_scan.rs`
- `src-tauri/src/services/ini/document.rs`
- `src-tauri/src/services/scanner/dedup/runtime_signature.rs` (new)
- `src-tauri/src/services/scanner/dedup/tests/runtime_signature_tests.rs` (new)

**Work:**

- Hapus parser dedup `take(200)` dan gunakan decoder yang mendukung encoding/ordered sections.
- Extract typed targets: kind, validated hash, namespace, `match_first_index`, `match_priority`, conditions, dan section order.
- Prune folder/file yang tidak loadable sesuai GIMI traversal.

**Done when:** signal setelah baris 200 terbaca dan comments tidak menjadi section evidence.

### SOD-07: Model command/resource graph secukupnya untuk variant relation

**Depends on:** SOD-06
**Files:**

- `src-tauri/src/services/scanner/dedup/runtime_signature.rs`
- `src-tauri/src/services/scanner/dedup/signals.rs`
- `src-tauri/src/services/scanner/dedup/ownership.rs`
- `src-tauri/src/services/scanner/dedup/tests/runtime_signature_tests.rs`
- `src-tauri/src/services/scanner/conflict/tests/conflict_tests.rs`

**Work:**

- Catat cycle variable/value, ordered branch, command-list call, resource slot, `checktextureoverride`, dan resource content identity.
- Klasifikasikan target sama + graph/resource berbeda sebagai related/conflict, bukan exact.
- Reuse typed facts yang relevan dari conflict scanner agar dua fitur tidak drift.

**Done when:** recolor/subvariant fixture tidak pernah menerima exact/destructive relation walaupun object hash sama.

## Phase 4 — Job lifecycle dan persistence

### SOD-08: Buat lifecycle scan terminal dan game-scoped

**Depends on:** SOD-05
**Files:**

- `src-tauri/src/commands/duplicates/dup_scan_cmds.rs`
- `src-tauri/src/types/dup_scan.rs`
- `src-tauri/src/services/scanner/dedup/scanner.rs`
- `src-tauri/src/commands/duplicates/tests/dup_scan_cmds_tests.rs`
- `src/lib/bindings.gen.ts` (generated)

**Work:**

- Tambahkan `Failed` event dan processed-count nyata.
- Scope running/cancel/report state berdasarkan game/job; cegah stale cross-game report.
- Pastikan terminal event selalu emitted tepat sekali.

**Done when:** success, failure, dan cancellation masing-masing mempunyai state serta test yang deterministik.

### SOD-09: Persist report atomically memakai schema existing

**Depends on:** SOD-08
**Files:**

- `src-tauri/src/repo/dedup_repo.rs`
- `src-tauri/src/services/scanner/dedup/scanner.rs`
- `src-tauri/src/commands/duplicates/dup_scan_cmds.rs`
- `src-tauri/src/repo/tests/dedup_repo_test.rs`
- `src-tauri/src/commands/duplicates/tests/dup_scan_cmds_tests.rs`

**Work:**

- Tulis `dedup_jobs/groups/group_members` dalam transaction hanya untuk hasil terminal yang valid.
- Query latest completed report per game; failed/cancelled job tidak mengganti report sukses terakhir.
- Simpan relation/evidence dalam JSON existing; evaluasi migration hanya jika profiling membuktikan query tidak cukup.

**Done when:** report bertahan setelah restart dan resolver selalu menemukan persisted group row.

## Phase 5 — Safe resolver dan UX

### SOD-10: Revalidate sebelum Trash/hardlink

**Depends on:** SOD-05, SOD-09
**Files:**

- `src-tauri/src/services/scanner/dedup/resolver.rs`
- `src-tauri/src/commands/duplicates/dup_resolve_cmds.rs`
- `src-tauri/src/services/scanner/dedup/tests/dedup_resolver_tests.rs`
- `src-tauri/src/commands/duplicates/tests/dup_resolve_cmds_tests.rs`

**Work:**

- Load persisted relation; reject destructive action pada class non-exact.
- Re-stat dan full-hash target segera sebelum mutation.
- Validasi same-volume hardlink, containment, mod identity/version, rollback, dan trailing reconcile.
- Skip portable metadata sesuai policy requirement.

**Done when:** post-scan file mutation menyebabkan safe rejection dan rollback test tidak kehilangan original.

### SOD-11: Pisahkan duplicate, shared assets, dan related mods di UI

**Depends on:** SOD-08, SOD-09
**Files:**

- `src/features/scanner/StorageOptimizerPage.tsx`
- `src/features/scanner/hooks/useDedup.ts`
- `src/features/scanner/components/DuplicateReport.tsx`
- `src/features/scanner/components/DuplicateTable.tsx`
- `src/features/scanner/components/DuplicateReport.test.tsx`

**Work:**

- Refresh report pada `Finished`, bukan saat start command return.
- Tambahkan states failed/cancelled/stale dan metrics logical units/files/verified bytes/skipped warnings.
- Tampilkan destructive controls hanya pada exact relation.
- Tampilkan `RelatedVariant` sebagai explanation/link ke conflict review, bukan duplicate row.

**Done when:** UI tidak menawarkan Keep/Delete/Hardlink untuk merged subvariant atau related variant.

### SOD-12: Resolution modal, i18n, dan generated contract

**Depends on:** SOD-10, SOD-11
**Files:**

- `src/features/scanner/components/ResolutionModal.tsx`
- `src/features/scanner/utils/resolutionRequests.ts`
- `src/locales/en/scanner.json`
- `src/locales/id/scanner.json`
- `src/locales/zh/scanner.json`

**Work:**

- Jelaskan exact verification, recoverable Trash, hardlink limitation, dan reasons untuk disabled actions.
- Review exact paths dan reclaim estimate sebelum mutation.
- Tambahkan translation keys EN/ID/ZH tanpa hard-coded user copy.

**Done when:** review modal hanya membangun request yang legal untuk relation class terpilih.

## Phase 6 — Verification dan rollout

### SOD-13: Regression, performance, dan documentation closeout

**Depends on:** SOD-01..SOD-12
**Files:**

- `src-tauri/src/services/scanner/dedup/tests/dedup_scanner_tests.rs`
- `src-tauri/src/services/scanner/dedup/tests/dedup_resolver_tests.rs`
- `src/features/scanner/hooks/useDedup.test.ts`
- `src/features/scanner/components/DuplicateReport.test.tsx`
- `.docs/history/<timestamp>-storage-optimizer-dedup.md` (new after implementation)

**Work:**

- Jalankan focused Rust/Frontend tests, full fmt/clippy/test/lint/i18n/build, bindings check, dan diff check.
- Benchmark 1k-file requirement serta 10k logical-root capacity; ukur cancellation latency.
- Manual smoke dengan copy library, bukan live library, untuk merged mod, exact copy, modified resource, locked file, and rollback.

**Done when:** seluruh acceptance criteria design terbukti dan limitation/manual follow-up dicatat jujur.

## Recommended execution order

`SOD-01 -> SOD-02 -> SOD-03 -> SOD-04 -> SOD-05 -> SOD-06 -> SOD-07 -> SOD-08 -> SOD-09 -> SOD-10 -> SOD-11 -> SOD-12 -> SOD-13`

Ownership (`SOD-02`) dan full-verification (`SOD-04`) adalah release blockers. Semantic enrichment boleh di-rollout bertahap, tetapi destructive UI tidak boleh aktif sebelum `SOD-10` selesai.
