# Implementation Plan: Disk–DB–Watcher–UI Convergence Hardening

## Overview

Menutup seluruh gap yang ditemukan pada audit drift tanpa mengubah prinsip utama: filesystem pada configured Mods directory adalah source of truth, sedangkan DB, runtime projection, collection references, cache, dan UI merupakan projection yang harus dapat dibangun ulang. Implementasi memakai primitive yang sudah ada (`ensure_mutation_preflight`, `OperationLock`, `SuppressionGuard`, disk reconcile, dan event result) dan tidak menambah orchestrator/event bus baru.

## Scope

Termasuk:

- Deep Match scan dan Scan Review commit.
- Perubahan Mods directory melalui Settings dan Source Recovery.
- Lifecycle watcher ketika game/path berubah cepat dan validasi command boundary.
- Restore Trash, collection recovery, dan capture current collection.
- Runtime projection, collection dirty state, frontend store/query cache.
- Browser import staging, archive/extract, metadata/thumbnail/INI mutation.
- Startup setelah aplikasi tidak aktif, external rename/delete/toggle, nested parent/child, bulk, auto-organize/category, dan collection apply.
- Performance watcher/reconcile dan targeted cleanup warning/dead code.

Tidak termasuk:

- Migrasi filesystem menjadi DB-authoritative.
- Event sourcing, distributed transaction, persistent operation framework, atau abstraction mutation generik baru.
- Optimasi spekulatif sebelum benchmark.
- Perubahan `.agents/` atau unrelated dirty worktree milik pengguna.

## Invariant Wajib

1. Path yang dibaca atau dimutasi harus berasal dari konfigurasi game; pasangan `game_id + path` dari command tidak boleh dipercaya langsung.
2. Source unavailable mempertahankan snapshot DB valid terakhir. Folder kosong/different hanya boleh diterapkan setelah klasifikasi dan konfirmasi eksplisit.
3. Konflik stable identity ditemukan oleh read-only preflight sebelum write DB atau disk.
4. Mutation disk satu game diserialisasi dengan `OperationLock`; rollback snapshot dibaca setelah lock diperoleh.
5. Watcher suppression hanya mencakup write fisik. Setelah suppression dilepas, mutation wajib menjalankan reconcile eksplisit.
6. Error reconcile tidak boleh diubah menjadi sukses diam-diam. Partial success harus terstruktur dan dapat dipulihkan.
7. Core DB projection, derived runtime projection, collection dirty state, cache invalidation, dan UI refresh harus akhirnya konvergen walau satu side effect gagal.
8. Startup/offline recovery harus selesai sebelum collection recovery/capture atau mutation lain membaca current state.
9. Watcher overflow atau event loss memicu full reconcile; rapid watcher replacement tidak boleh dihentikan oleh cleanup sesi lama.
10. Tauri command/signature baru atau berubah wajib sinkron dengan invoke handler, permissions, generated bindings, dan registration audit.

## Dependency Graph

```text
T1 invariant characterization
 ├─ T2 scanner command boundary
 │   └─ T3 scanner move journal/failure convergence
 ├─ T4 settings backend path guard
 │   └─ T5 settings/source-recovery UI
 ├─ T6 watcher lifecycle and boundary
 ├─ T7 Trash restore
 └─ T8 collection recovery/capture

T1 ── T9 transactional runtime projection
        └─ T10 durable/retryable side effects and cache

T1 ── T11 browser import staging
T1 ── T12 metadata and frontend optimistic state

T2–T12 ── T13 existing-flow regression matrix
T6 + T9 + T10 + T13 ── T14 performance work
T1–T14 ── T15 cleanup, full verification, documentation
```

## Phase 1 — Characterize dan Tutup Jalur Kritis

### Task 1 — Tambahkan invariant characterization tests

**Description:** Tambahkan failure-injection dan contract tests sebelum implementation untuk membuktikan gap scanner, path setting, watcher lifecycle, Trash, collection capture/recovery, dan derived projection. Reuse fixture reconcile yang sudah ada.

**Acceptance criteria:**

- Test merah membuktikan scanner conflict dapat mencapai commit, path existing dapat berubah lewat generic settings, dan restore dapat sukses walau reconcile gagal.
- Test concurrent/deferred watcher membuktikan cleanup lama dapat menghentikan sesi baru.
- Test side-effect failure membuktikan derived projection/collection dirty dapat tertinggal setelah core commit.

**Verification:** targeted Rust dan Vitest files dapat dijalankan terpisah dan hanya gagal pada invariant yang belum diperbaiki.

**Dependencies:** None.

**Files likely touched:**

- `src-tauri/src/services/scanner/tests/sync_tests.rs`
- `src-tauri/src/commands/app/tests/game_cmds_tests.rs`
- `src-tauri/src/commands/mods/tests/conflict_cmds_tests.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/tests.rs`
- `src/features/file-watcher/hooks.test.ts`

**Estimated scope:** Medium.

### Task 2 — Amankan command boundary Deep Match

**Description:** Pastikan `deepmatch_scanner_cmd` dan `commit_scan_cmd` memvalidasi configured game root, menjalankan full identity-conflict preflight, mengambil per-game operation lock, dan tidak memulai commit jika blocked/source unavailable.

**Acceptance criteria:**

- `Mod A` + `DISABLED Mod A` menghasilkan structured blocked result tanpa write disk/DB dan tanpa SQL 1555.
- `mods_path` yang tidak cocok dengan konfigurasi game ditolak sebelum scan/commit.
- Settings Rescan dan Scan Review memakai boundary yang sama tanpa duplicate orchestration.

**Verification:** targeted scanner command/service tests, registration audit, generated binding check.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/commands/scanner/deepmatch_scanner_cmds.rs`
- `src-tauri/src/services/disk_reconcile/mod.rs`
- `src-tauri/src/services/scanner/tests/sync_tests.rs`
- `src/features/object-list/hooks/useScanReviewFlow.ts`

**Estimated scope:** Medium.

### Task 3 — Buat scanner commit recoverable tanpa framework baru

**Description:** Catat setiap move fisik scanner dalam local move journal. Pada error sebelum DB commit, rollback inverse moves best-effort; pada success atau rollback tidak lengkap, lepaskan suppression lalu jalankan full reconcile sehingga disk terbaru kembali menjadi kebenaran. Propagasikan kegagalan `info.json` yang sekarang diabaikan.

**Acceptance criteria:**

- Injected failure setelah satu/lebih move tidak meninggalkan silent DB–disk drift.
- Rollback yang berhasil mengembalikan disk dan DB ke state awal; rollback yang gagal mengembalikan structured recovery error dan full reconcile terhadap state disk aktual.
- Tidak ada `let _`/`.ok()` untuk write yang menentukan correctness commit.

**Verification:** scanner failure-injection tests untuk first/middle/final move, DB insert failure, metadata failure, dan restart recovery.

**Dependencies:** Task 2.

**Files likely touched:**

- `src-tauri/src/services/scanner/sync/commit/run.rs`
- `src-tauri/src/services/scanner/sync/commit/temp_move.rs`
- `src-tauri/src/services/scanner/sync/commit/execute.rs`
- `src-tauri/src/commands/scanner/deepmatch_scanner_cmds.rs`
- `src-tauri/src/services/scanner/tests/sync_tests.rs`

**Estimated scope:** Medium.

### Checkpoint A — Critical scanner safety

- Scanner conflict tidak dapat mencapai SQL write.
- Scanner failure selalu berakhir pada rollback atau explicit disk-authoritative convergence.
- Targeted Rust tests hijau dan exact diff tidak mengganggu perubahan pengguna lain.

## Phase 2 — Source Path dan Watcher Lifecycle

### Task 4 — Tolak perubahan existing mod path melalui generic settings backend

**Description:** Tambahkan server-side validation pada save settings: game baru boleh memiliki initial path, tetapi perubahan `mod_path` game existing harus melalui inspect/apply Source Recovery. Field settings lain tetap dapat disimpan normal.

**Acceptance criteria:**

- Active maupun inactive existing game tidak dapat mengganti path melalui `save_settings` biasa.
- Menambah game baru dan menyimpan perubahan non-path tetap kompatibel.
- Error menyebut action yang benar dan tidak memodifikasi DB/config parsial.

**Verification:** config/settings service tests dengan existing/new/unchanged path matrix.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/commands/app/settings_cmds.rs`
- `src-tauri/src/services/config/service.rs`
- `src-tauri/src/services/config/tests/service_tests.rs`
- `src-tauri/src/commands/app/tests/game_cmds_tests.rs`

**Estimated scope:** Medium.

### Task 5 — Route Game Settings ke Source Recovery UI

**Description:** Untuk game existing, perubahan directory menjalankan inspect dan confirmation flow yang sudah ada. Empty/different/unavailable ditangani sesuai kontrak; modal menunggu promise dan hanya menutup setelah apply sukses. Setelah apply, active game, watcher session, query cache, dan workspace state menggunakan path baru.

**Acceptance criteria:**

- Matching path auto-sync; empty path meminta konfirmasi “mulai dari nol”; unavailable mempertahankan path/snapshot lama.
- Save/apply failure menjaga modal dan draft tetap terbuka dengan error yang dapat ditindaklanjuti.
- Tidak ada dua implementasi dialog source recovery.

**Verification:** GamesTab/GameFormModal tests, source recovery dialog tests, active/inactive game integration test.

**Dependencies:** Task 4.

**Files likely touched:**

- `src/features/settings/modals/GameFormModal.tsx`
- `src/features/settings/tabs/GamesTab.tsx`
- `src/features/settings/tabs/GamesTab.test.tsx`
- `src/features/folder-grid/components/WorkspaceSourceUnavailableDialog.tsx`
- `src/features/folder-grid/components/WorkspaceSourceUnavailableDialog.test.tsx`

**Estimated scope:** Medium.

### Task 6 — Serialisasi watcher lifecycle dan validasi start boundary

**Description:** Jadikan backend `start_watcher` satu-satunya atomic replacement. Frontend memakai generation guard agar cleanup sesi lama tidak menghentikan sesi baru. Backend memverifikasi `game_id/path` terhadap config; overflow behavior full reconcile dipertahankan.

**Acceptance criteria:**

- Rapid A→B→A game/path switch selalu meninggalkan watcher pada session/generation terakhir.
- Stale cleanup tidak dapat menghentikan watcher terbaru.
- Wrong game/path pairing ditolak; start failure terlihat dan dapat diretry.

**Verification:** deferred-promise hook test, watcher lifecycle Rust test, wrong-path command test, overflow regression test.

**Dependencies:** Task 1.

**Files likely touched:**

- `src/features/file-watcher/hooks.ts`
- `src/features/file-watcher/hooks.test.ts`
- `src-tauri/src/commands/scanner/watcher_cmds.rs`
- `src-tauri/src/services/scanner/watcher/lifecycle.rs`
- `src-tauri/src/services/scanner/tests/watcher_tests.rs`

**Estimated scope:** Medium.

### Checkpoint B — Source dan live synchronization

- Generic settings tidak dapat melewati source recovery.
- Rapid switch, unavailable source, empty source, dan watcher overflow teruji.
- Frontend active path, backend config, watcher root, dan grid menunjukkan source yang sama.

## Phase 3 — Tutup Mutation Bypass Lain

### Task 7 — Jadikan Trash restore mutation yang terjaga

**Description:** Tambahkan configured-root validation, preflight, operation lock, scoped suppression, dan mandatory full reconcile pada restore. Reconcile failure dipropagasikan sebagai partial recovery state, bukan sukses biasa.

**Acceptance criteria:**

- Restore tidak berjalan saat folder conflict/source unavailable.
- Restore vs bulk/toggle terserialisasi.
- Setelah physical restore, command hanya sukses bila DB/grid sudah konvergen; partial failure tetap dapat direconcile ulang.

**Verification:** restore conflict, concurrent mutation, injected reconcile failure, dan normal restore tests.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/commands/mods/trash_cmds.rs`
- `src-tauri/src/services/mods/trash/service.rs`
- `src-tauri/src/commands/mods/tests/conflict_cmds_tests.rs`
- `src/hooks/useFolderMutations.test.tsx`

**Estimated scope:** Medium.

### Task 8 — Sinkronkan collection recovery dan capture current state

**Description:** Jalankan full disk recovery sebelum retry/rollback recovery task dan sebelum create/replace collection yang menangkap current state. Pass existing reconcile lock ke apply; tandai recovery task selesai hanya setelah final reconcile. Rename/delete metadata collection biasa tidak perlu full scan.

**Acceptance criteria:**

- External change selama aplikasi mati diproyeksikan sebelum recovery task/capture membaca current state.
- Collection member path/missing status mengikuti nested rename dan deleted parent terbaru.
- Recovery apply error tidak menandai task complete.

**Verification:** offline external rename/delete fixtures, pending watcher capture test, retry/rollback failure tests.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/commands/collections/cmds.rs`
- `src-tauri/src/services/recovery_service.rs`
- `src-tauri/src/services/collection_service/tests/apply_tests.rs`
- `src-tauri/src/services/collection_service/tests/replace_tests.rs`
- `src-tauri/src/services/collection_service/tests/references_tests.rs`

**Estimated scope:** Medium.

## Phase 4 — Derived Projection, Cache, dan UI Convergence

### Task 9 — Satukan core dan runtime DB projection transaction

**Description:** Gunakan transaction-capable runtime projection helper dalam projection writer sehingga object/mod rows dan affected runtime projection commit bersama. Untuk full rebuild, tambahkan tx variant kecil di repository; jangan membuat repository layer baru.

**Acceptance criteria:**

- Injected runtime projection failure me-roll back core projection dalam reconcile yang sama.
- Scoped reconcile hanya memperbarui affected object IDs; full reconcile membangun ulang projection konsisten.
- Query grid tidak pernah melihat core row baru dengan runtime projection lama setelah command sukses.

**Verification:** transaction rollback tests dan projection parity queries setelah full/scoped reconcile.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/reconcile.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/write.rs`
- `src-tauri/src/repo/runtime_projection_repo.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/tests.rs`

**Estimated scope:** Medium.

### Task 10 — Pertahankan side-effect dirty sampai sukses

**Description:** Pisahkan “core changed” dari “runtime side effects pending”. Requeue/persist existing dirty indication sampai collection snapshot, overlay, dan cache invalidation sukses. Explicit app mutations tidak boleh menelan failure projection/side effects tanpa structured warning.

**Acceptance criteria:**

- Failure pertama lalu retry tanpa perubahan disk tetap mengeksekusi side effect yang tertunda.
- Collection dirty dan overlay tidak dianggap bersih sebelum sukses.
- FE query/store invalidation berasal dari terminal reconcile result dan tidak bergantung pada duplicate watcher toast/event.

**Verification:** failure-then-retry tests untuk collection dirty, overlay/projection, dan frontend event/cache update.

**Dependencies:** Task 9.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/orchestrator/state.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/run.rs`
- `src-tauri/src/services/app/runtime_effects.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/tests.rs`
- `src/features/file-watcher/ExternalChangeHandler.test.tsx`

**Estimated scope:** Medium.

## Phase 5 — Import dan Metadata Reliability

### Task 11 — Perketat browser import state dan staging cleanup

**Description:** Propagasikan write job state yang wajib (`archive_hash`, `staging_path`, `match_result`, terminal status), hapus staging setelah placement sukses, dan tambahkan bounded cleanup terminal job lama menggunakan struktur staging yang sudah ada.

**Acceptance criteria:**

- Job tidak masuk `needs_review` tanpa staging path yang dapat dibaca.
- DB state failure menghentikan pipeline dengan status actionable.
- Success/cancel membersihkan staging; cleanup tidak dapat keluar dari app staging root.

**Verification:** injected repo failure, review continuation, success/cancel cleanup, traversal containment, dan stale-terminal sweep tests.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/services/browser/import_service/pipeline.rs`
- `src-tauri/src/services/browser/import_service/jobs.rs`
- `src-tauri/src/services/browser/import_service/placement.rs`
- `src-tauri/src/repo/browser_repo/import_jobs.rs`
- browser import service tests

**Estimated scope:** Medium.

### Task 12 — Betulkan metadata rollback dan optimistic frontend state

**Description:** Ambil rollback snapshot metadata setelah operation lock. Untuk setting frontend seperti `autoCloseLauncher`, update local state setelah backend sukses atau rollback/refetch pada error.

**Acceptance criteria:**

- Dua update metadata concurrent tidak dapat merestore bytes sebelum mutation pertama.
- Reconcile failure mengembalikan file ke snapshot yang dibaca di dalam lock.
- Backend save failure tidak meninggalkan Zustand state berbeda dari persisted setting.

**Verification:** concurrent metadata failure-injection test dan frontend optimistic rollback test.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/commands/mods/mod_meta_cmds.rs`
- `src-tauri/src/services/mods/tests/info_json_tests.rs`
- `src/stores/appStore/gameSlice.ts`
- `src/stores/useAppStore.test.ts`

**Estimated scope:** Small.

### Checkpoint C — Mutation dan cache convergence

- Trash, collection recovery/capture, scanner/import, metadata, projection, dan cache failure tests hijau.
- Tidak ada success response yang menyembunyikan reconcile failure.
- DB/runtime projection/collection/UI dapat konvergen setelah injected transient failure.

## Phase 6 — Regression Matrix dan Performance

### Task 13 — Kunci seluruh flow yang sudah benar dengan regression matrix

**Description:** Tambahkan/rapikan test matrix agar perubahan pada task sebelumnya tidak merusak flow yang audit nyatakan sudah aman. Reuse fixture dan table-driven tests; jangan menduplikasi setup besar.

**Acceptance criteria:**

- In-app dan external create/rename/delete/toggle diuji untuk flat, nested, parent, child, enabled, dan disabled prefix.
- Single/bulk, auto-organize/category, collection change/apply, archive extract/import, thumbnail/info/INI CRUD, source path, dan startup offline recovery masing-masing memiliki paling sedikit satu convergence assertion disk=DB=projection.
- Existing nested rename dan parent-delete collection-missing regressions tetap hijau.

**Verification:** targeted Rust/TS suites dan E2E fixture menjalankan rename resolution, Trash resolution, offline startup, dan rapid watcher switch.

**Dependencies:** Tasks 2–12.

**Files likely touched:** existing tests colocated per service, `tests/e2e/specs/phase3c-folder-conflicts.e2e.ts`, dan maksimal satu shared fixture helper.

**Estimated scope:** Medium per test slice; implement bertahap per subsystem.

### Task 14 — Hilangkan duplicate full scans berdasarkan benchmark

**Description:** Tambahkan instrumentation/benchmark untuk startup dan watcher burst. Reuse terminal startup reconcile sebagai watcher baseline bila game/path/generation sama; untuk scoped watcher event, pertahankan full lightweight identity census tetapi batasi metadata/INI classification ke affected roots bila hasil benchmark membenarkan.

**Acceptance criteria:**

- Satu activation/startup tidak melakukan full metadata projection berulang tanpa perubahan source.
- Burst pada fixture besar tidak kehilangan event dan selalu menghasilkan terminal result terbaru.
- Optimasi tidak mengurangi conflict detection atau offline recovery correctness; before/after scan count dan latency dicatat.

**Verification:** benchmark fixture 10k folders, watcher burst integration test, startup scan-count assertion, full reconcile regression suite.

**Dependencies:** Tasks 6, 9, 10, dan 13.

**Files likely touched:**

- `src-tauri/src/services/bootstrap.rs`
- `src-tauri/src/services/scanner/watcher/lifecycle.rs`
- `src-tauri/src/services/disk_reconcile/reconcile.rs`
- `src-tauri/src/services/disk_reconcile/disk_snapshot.rs`
- benchmark/integration test module

**Estimated scope:** Medium.

## Phase 7 — Clean Code dan Release Verification

### Task 15 — Targeted cleanup, documentation, dan final gates

**Description:** Setelah correctness stabil, hilangkan duplicate lifecycle responsibility dan warning yang menyentuh area berubah. Pecah `file-watcher/hooks.ts` hanya bila masih melewati batas 350 baris. Hapus unused `_kind`, betulkan exhaustive deps, format touched files, dan dokumentasikan invariant/command registration.

**Acceptance criteria:**

- Tidak ada new dependency, unused production code, swallowed correctness error, atau duplicate mutation guard.
- Touched production files memenuhi lint/format; hook dependency warnings pada area bulk selesai.
- Semua command yang berubah tetap terdaftar, di-allowlist, ada generated binding, dan tercatat di docs.

**Verification:**

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features`
- `cargo test`
- `pnpm test -- --run`
- `pnpm lint`
- `pnpm i18n:lint`
- `pnpm build`
- Targeted WebdriverIO E2E
- `git diff --check` dan read-only final review

**Dependencies:** Tasks 1–14.

**Files likely touched:** hanya file warning yang relevan, `docs/knowledge/tauri-command-registration.md`, dan history/audit record.

**Estimated scope:** Medium.

## Risks and Mitigations

| Risk                                             | Impact                      | Mitigation                                                                                             |
| ------------------------------------------------ | --------------------------- | ------------------------------------------------------------------------------------------------------ |
| Dirty worktree overlap                           | Perubahan pengguna tertimpa | Re-read exact diff sebelum tiap task; patch fungsi kecil; jangan format repo-wide                      |
| Filesystem dan SQLite tidak dapat satu transaksi | Partial move                | Operation lock, local inverse-move journal, best-effort rollback, mandatory reconcile                  |
| Full preflight memperlambat watcher              | UI terlambat                | Correctness tetap prioritas; ukur, lalu pisahkan lightweight identity census dari scoped metadata scan |
| Retry side effects menyebabkan loop              | CPU/log spam                | Retry dibatasi oleh existing coordinator/backoff dan dirty hanya dihapus saat sukses                   |
| Generic settings guard memutus onboarding        | Game baru gagal dibuat      | Bedakan new game, unchanged path, dan existing-path mutation dalam contract tests                      |
| Cleanup staging menghapus path salah             | Data loss                   | Canonical containment terhadap app staging root sebelum delete                                         |
| Refactor terlalu luas                            | Bug baru                    | Cleanup dilakukan paling akhir dan hanya pada touched code/warning yang relevan                        |

## Definition of Done

- Disk tetap source of truth hanya setelah configured source tervalidasi.
- Seluruh mutation disk memakai preflight, per-game lock, scoped suppression, dan terminal reconcile.
- SourceUnavailable/Blocked tidak mengubah snapshot valid terakhir.
- Tidak ada jalur scanner/restore/recovery/settings yang dapat melewati invariant.
- External changes saat aplikasi aktif maupun tidak aktif menghasilkan DB, runtime projection, collection reference, cache, dan UI yang sama.
- Failure injection menunjukkan retry/rollback dapat mencapai convergence.
- Full test/lint/build/E2E gates dicatat apa adanya.
- Final code review tidak menemukan correctness/security regression berprioritas tinggi.

## Execution Order

Kerjakan secara sequential sesuai dependency graph karena hampir semua task menyentuh reconcile contracts. Test-only regression slices pada Task 13 boleh dikerjakan terpisah setelah contract tiap subsystem stabil, tetapi production writes pada scanner, watcher, dan reconcile tidak boleh diparalelkan.
