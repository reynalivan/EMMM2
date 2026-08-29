# Implementation Plan: Responsive Mods Workspace with Async Disk Reconcile

## Outcome

Pembukaan workspace Mods tidak lagi menunggu full disk reconcile sebelum menampilkan interface dan daftar folder. Full reconcile tetap berjalan asynchronous, filesystem tetap menjadi source of truth, database tetap merupakan last-valid transactional projection, dan seluruh mutation tetap ditolak sampai initial recovery untuk game aktif mencapai hasil terminal.

Progress bar harus berasal dari unit kerja backend yang benar-benar selesai. UI tidak menghitung persentase atau ETA dari timer sintetis. Saat total belum diketahui, progress tampil indeterminate; saat total diketahui, label menyebut unit yang diukur (misalnya object root yang selesai dipindai), dan ETA hanya tampil setelah sampel cukup.

Tidak ditambahkan persistent filesystem journal, polling daemon, event bus baru, cache disk baru, saga engine, atau background service terpisah. Implementasi memakai `DiskReconcileState`, per-game reconcile queue, `OperationLock`, React Query, dan event Tauri yang sudah ada.

## Invariant

1. Disk tetap sumber kebenaran. Snapshot DB yang tampil selama sync hanya last-valid read model dan tidak pernah ditulis kembali ke disk.
2. Full startup/offline reconcile menulis DB dan runtime projection dalam transaction yang sudah ada; UI tidak menerima partial DB state.
3. Browse, scroll, filter, dan navigasi aplikasi tetap responsif selama sync. Rename, toggle, delete, bulk, import, organize, collection apply, metadata write, dan action disk lain tetap disabled di UI dan ditolak backend saat initial recovery pending.
4. Watcher event yang datang ketika recovery berjalan masuk ke per-game queue dan diproses setelah pass aktif; event tidak hilang dan terminal result selalu merepresentasikan versi terbaru.
5. SourceUnavailable, rename confirmation, dan folder conflicts tetap structured result. Async startup tidak mengubahnya menjadi sukses atau menghapus last-valid snapshot.
6. Satu game hanya memiliki satu recovery/reconcile aktif. Enter Mods, startup, focus, dan watcher tidak boleh membuat full-scan storm.
7. Progress event memiliki `game_id` dan `run_id`; event dari game/run lama tidak boleh mengubah UI aktif.

## Architecture Decisions

### 1. Pisahkan read readiness dari mutation readiness

`get_workspace_view_model` tidak lagi menunggu `ensure_initial_disk_recovery`. Command segera mengembalikan read model dengan status freshness:

- `Ready`: projection sudah terminal untuk source/generation aktif.
- `Syncing`: initial recovery berjalan; data adalah last-valid snapshot atau provisional shallow listing.
- `Failed`: recovery gagal; last-valid snapshot tetap terlihat dan error dapat ditindaklanjuti.

Mutation readiness tetap dimiliki backend. Semua mutation preflight memeriksa recovery gate sebelum menyentuh filesystem, sehingga stale frontend tidak dapat melewati guard.

### 2. Render cepat tanpa partial database commit

Urutan sumber data untuk initial workspace read:

1. Jika DB memiliki last-valid projection, gunakan projection tersebut untuk rich list segera.
2. Jika DB masih kosong (first onboarding/new game), lakukan shallow `read_dir` pada directory yang sedang dilihat: nama, path, enabled/disabled prefix, directory flag, dan entry metadata saja. Jangan parse INI, `info.json`, thumbnail, atau recurse.
3. Node shallow ditandai provisional dan read-only. Setelah terminal reconcile, satu query invalidation menggantinya dengan authoritative workspace model.

Dengan ini folder fisik dapat terlihat sebelum reconcile selesai tanpa menulis row parsial atau membuat cache baru.

### 3. Progress faktual per fase

Tambahkan event `disk_reconcile:progress` dengan payload minimum:

- `game_id`, `run_id`, `reason`;
- `phase`: `WaitingForLock`, `DiscoveringRoots`, `ScanningRoots`, `Projecting`, `Finalizing`, `Completed`, atau `Failed`;
- `completed_units`, `total_units: Option<usize>`, `unit_kind`;
- `elapsed_ms`, `eta_ms: Option<u64>`;
- optional current root display name, tanpa full sensitive path.

Aturan validitas:

- Discovery dan lock wait indeterminate jika total belum diketahui.
- `ScanningRoots` memakai jumlah top-level physical roots yang benar-benar selesai dari total hasil satu `read_dir`; tidak ada recursive pre-scan kedua hanya untuk progress.
- Projecting memakai row/object operation count hanya bila writer mengetahui denominator stabil; jika tidak, indeterminate.
- Persentase tidak digabung memakai phase weight buatan. Bar menunjukkan progress fase aktif dan phase stepper menunjukkan kemajuan keseluruhan.
- ETA hanya untuk fase dengan denominator stabil, setelah minimum sampel dan waktu warm-up; tampil sebagai perkiraan (`~`) dan dihapus bila sampel tidak stabil.
- Emit dibatasi pada perubahan unit/persentase atau interval 100–200 ms, tetapi terminal event tidak boleh di-throttle away.

### 4. Scoped reconcile benar-benar scoped

Ordinary watcher/internal mutation melakukan classification hanya pada affected root. Full deep scan tetap wajib untuk startup/offline recovery, path/game activation, watcher overflow/dropped events, manual repair, dan kondisi source generation berubah.

Untuk menjaga conflict queue lengkap:

- Scoped pass melakukan sibling/identity preflight pada affected scope.
- Jika scope menemukan conflict baru, jalankan full lightweight identity/name census untuk membangun seluruh queue, bukan full INI/metadata classification seluruh game.
- Protected conflict scopes tetap memakai partial reconcile behavior yang sudah ada; unrelated roots tetap konvergen.

### 5. Runtime side effects tidak menghalangi render

Projection commit, collection dirty handling, dan overlay refresh tetap berada dalam lifecycle reconcile dan terlihat sebagai fase `Projecting`/`Finalizing`. Workspace sudah dapat dirender selama fase tersebut. Mutation baru aktif hanya setelah terminal result, sehingga tidak ada concurrency window baru.

## Dependency Graph

```text
T1 baseline + characterization
 ├─ T2 recovery state/readiness contract
 │   ├─ T3 factual backend progress
 │   └─ T4 non-blocking workspace read model
 ├─ T5 true scoped reconcile + lightweight conflict census
 └─ T6 frontend sync UX + mutation gate

T2 + T3 + T4 + T5 + T6
 └─ T7 remove measured duplicate work and tune concurrency
     └─ T8 regression, E2E, performance verification, docs
```

## Task 1 — Characterize latency and blocking behavior

**Description:** Tambahkan stage timing dan characterization tests sebelum mengubah behavior. Ukur lock wait, disk snapshot, conflict detection, projection write/rebuild, collection/overlay finalization, workspace query, dan explorer listing secara terpisah. Gunakan cold dan warm run pada fixture kecil, nested, 10k folders, serta fixture INI besar.

**Acceptance criteria:**

- [ ] Log memiliki satu `run_id` dan durasi setiap stage tanpa per-file spam atau path sensitif.
- [ ] Test membuktikan workspace request saat ini menunggu initial recovery dan mutation tidak boleh berjalan selama gate pending.
- [ ] Baseline mencatat time-to-first-workspace-model dan total reconciliation time secara terpisah.

**Verification:** targeted Rust timing/initial-recovery tests, existing 10k benchmark, dan satu manual run pada Mods directory representatif.

**Dependencies:** None.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/orchestrator/run.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/state.rs`
- `src-tauri/src/services/disk_reconcile/reconcile.rs`
- `src-tauri/src/commands/app/workspace_cmds.rs`
- focused test modules

**Estimated scope:** M.

## Task 2 — Expose non-blocking recovery and mutation readiness

**Description:** Kembangkan existing initial recovery gate dengan read-only status snapshot dan run generation. Startup/game activation dapat claim/spawn recovery sekali tanpa membuat workspace reader menunggu. Mutation preflight menunggu atau menolak dengan structured `SyncInProgress` sebelum filesystem write; pilih fail-fast untuk direct UI action agar tidak menghasilkan tombol yang terlihat macet.

**Acceptance criteria:**

- [ ] Workspace reader dapat memperoleh `Syncing` dalam bounded time saat recovery masih berjalan.
- [ ] Concurrent startup/Mods entry/focus hanya menjalankan satu full recovery untuk game/source generation yang sama.
- [ ] Semua backend mutation boundary menolak `SyncInProgress`; tidak hanya mengandalkan disabled button frontend.
- [ ] Watcher batches selama recovery tetap ter-coalesce dan diproses sebelum terminal newest-version result.

**Verification:** Rust concurrency tests untuk simultaneous reader, mutation, watcher batch, duplicate startup claim, failure, dan source-generation reset.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/orchestrator/state.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/entry.rs`
- `src-tauri/src/services/disk_reconcile/emit.rs`
- `src-tauri/src/services/bootstrap.rs`
- orchestrator tests

**Estimated scope:** M.

## Task 3 — Emit factual and bounded reconcile progress

**Description:** Instrument existing snapshot and projection stages dengan optional progress sink. Setiap reconcile mempunyai `run_id`; collector melaporkan root completion dari actual worker completion dan orchestrator melaporkan phase transition/terminal state. Progress sink tidak mengubah output atau ordering projection.

**Acceptance criteria:**

- [ ] Event sequence valid dan terminal: waiting/discovery → scanning → projecting → finalizing → completed/failed.
- [ ] `completed_units <= total_units`, counter monotonic dalam phase/run, dan total berasal dari worklist nyata.
- [ ] Tidak ada recursive pre-scan tambahan, event flood, negative ETA, atau ETA sebelum warm-up.
- [ ] Cancellation/error/panic join menghasilkan terminal failed event atau structured command error; progress tidak tertinggal aktif.
- [ ] Parallel scan menghasilkan counter race-free dan tidak memengaruhi deterministic projection result.

**Verification:** Rust unit tests untuk phase order, concurrent counter, throttle, ETA estimator, empty root, one huge root, source unavailable, dan error terminal.

**Dependencies:** Task 2.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/disk_snapshot.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/run.rs`
- `src-tauri/src/services/disk_reconcile/types.rs`
- `src-tauri/src/commands/scanner/disk_reconcile_cmds.rs`
- generated bindings/event tests if required

**Estimated scope:** M.

## Task 4 — Return a workspace model before reconciliation finishes

**Description:** Hapus blocking await initial recovery dari workspace command. Tambahkan snapshot/provisional read mode: prefer DB last-valid projection; untuk empty DB gunakan shallow current-directory listing tanpa classifier/INI/info/thumbnail work. Sertakan freshness dan provisional state dalam workspace runtime contract.

**Acceptance criteria:**

- [ ] Workspace shell dan list response tersedia saat reconcile sengaja ditahan dalam test.
- [ ] Existing DB rows tampil sebagai last-valid snapshot; first-run physical folders tampil melalui shallow read-only nodes.
- [ ] Tidak ada DB write, recursive walk, INI parse, thumbnail load, atau `info.json` parse pada provisional path.
- [ ] Source unavailable menampilkan last-valid snapshot dengan recovery UI; empty valid source menampilkan empty state yang benar.
- [ ] Selection yang tidak dapat divalidasi selama provisional mode tidak dihapus permanen sampai terminal refresh.

**Verification:** workspace service/command tests untuk populated DB, empty DB, huge directory, missing source, pending conflict report, dan selected nested path.

**Dependencies:** Task 2.

**Files likely touched:**

- `src-tauri/src/commands/app/workspace_cmds.rs`
- `src-tauri/src/services/workspace_service.rs`
- `src-tauri/src/services/workspace_read_model/*`
- `src-tauri/src/domain/workspace/view.rs`
- workspace service tests

**Estimated scope:** M; pecah mapper provisional menjadi helper kecil bila file melebihi batas.

## Task 5 — Make ordinary reconciliation truly scoped

**Description:** Ubah snapshot request agar affected watcher/internal roots memakai scoped classification. Pisahkan lightweight identity/name census dari deep folder classification ketika seluruh conflict queue diperlukan. Full scan policy dibuat eksplisit dalam satu function, bukan tersebar di caller.

**Acceptance criteria:**

- [ ] Toggle/rename/metadata watcher event pada satu root tidak membaca INI root lain.
- [ ] Startup after app inactive, game/path change, watcher overflow/drop, dan manual repair selalu full scan.
- [ ] Conflict pada affected scope menghasilkan queue lengkap tanpa full deep classification.
- [ ] Nested parent/child rename, collection reference rewrite, thumbnail invalidation, and protected conflict scope behavior tetap benar.
- [ ] Scoped writer tidak prune row di luar changed roots dan full recovery tetap menangkap offline external changes.

**Verification:** filesystem read-count spies/test collector, scoped/full policy matrix, conflict queue fixtures, nested rename/delete tests, watcher overflow test, dan DB convergence assertions.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/reconcile.rs`
- `src-tauri/src/services/disk_reconcile/disk_snapshot.rs`
- `src-tauri/src/services/disk_reconcile/identity_conflicts.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/request.rs`
- reconcile tests

**Estimated scope:** M.

## Task 6 — Add responsive sync UX and authoritative frontend gating

**Description:** Subscribe once at app/workspace coordinator level to progress events. Simpan progress per game/run secara bounded, render persistent compact sync banner/progress above grid, dan pertahankan list yang sudah ada. Gunakan backend workspace freshness untuk disabled mutation state; terminal result melakukan satu atomic cache refresh dan membuka existing conflict/source/rename flow bila diperlukan.

**Acceptance criteria:**

- [ ] List tidak diganti fullscreen spinner atau di-unmount selama sync; scrolling, searching, switching view, dan navigation aplikasi tetap bekerja.
- [ ] Progress bar determinate hanya ketika `total_units` tersedia; label unit/phase dan approximate ETA sesuai payload backend.
- [ ] Stale event game/run lama diabaikan; remount tidak menggandakan listener.
- [ ] Semua single/bulk/collection/import/organize actions yang menulis disk terlihat disabled dengan alasan sync, dan stale click mendapat actionable backend error tanpa duplicate toast.
- [ ] Terminal Applied/AppliedWithFolderConflicts/NeedsRenameConfirmation/SourceUnavailable/Failed masing-masing membersihkan progress dan memperbarui UI sesuai kontrak.
- [ ] Accessible `role=progressbar`, `aria-valuenow` hanya saat determinate, live phase text, keyboard access, reduced-motion, serta i18n EN/ID/ZH tersedia.

**Verification:** Vitest event sequence, stale run, listener cleanup, mutation guard, query invalidation count, and status matrix tests; desktop E2E dengan reconcile yang sengaja diperlambat.

**Dependencies:** Tasks 2–4.

**Files likely touched:**

- `src/features/file-watcher/hooks.ts` atau focused extracted progress hook
- `src/features/workspace-runtime/useWorkspaceViewModel.ts`
- `src/features/folder-grid/components/FolderGridBanners.tsx`
- folder-grid mutation guard/view-model files
- `src/locales/{en,id,zh}/*.json`

**Estimated scope:** M.

## Task 7 — Remove only measured duplicate work and tune disk concurrency

**Description:** Setelah UI tidak lagi blocking, gunakan Task 1 timings untuk memilih optimasi backend terkecil. Kandidat: hindari reclassification langsung setelah terminal reconcile, streaming INI read tanpa whole-file allocation, dan fixed bounded disk concurrency bila real HDD/external-volume benchmark membuktikan Rayon global pool memperburuk latency. Setiap eksperimen yang tidak melewati noise dibatalkan.

**Acceptance criteria:**

- [ ] Tidak ada full reconcile kedua dari startup + ModsViewEntered untuk generation yang sama.
- [ ] Reconcile terminal memicu maksimal satu workspace refetch dan tidak menciptakan scan loop.
- [ ] Perubahan concurrency/parser hanya dipertahankan bila cold/warm p50 dan p95 membaik di atas run variance tanpa correctness regression.
- [ ] Tidak ditambahkan persistent cache atau fingerprint yang dapat melewatkan nested external changes.

**Verification:** before/after ledger untuk tiap eksperimen, scan-count assertion, real fixture benchmark, CPU/IO observation, and full reconcile parity tests.

**Dependencies:** Tasks 3–6.

**Files likely touched:** ditentukan oleh hasil profiling; utamanya snapshot/classifier/workspace listing dan benchmark modules.

**Estimated scope:** S–M; optimasi netral atau lebih buruk direvert.

## Task 8 — Full regression, performance budget, and documentation

**Description:** Jalankan matrix online/offline dan in-app/external untuk memastikan async readiness tidak membuka drift atau race baru. Dokumentasikan progress event, mutation readiness, dan command permission bila contract berubah.

**Acceptance criteria:**

- [ ] First onboarding, normal restart, restart setelah offline rename/delete/toggle, game switch, source unavailable, empty source, conflict, rename confirmation, dan watcher overflow mencapai terminal state benar.
- [ ] Enable/disable, rename, CRUD, bulk, import/extract, auto-organize/category, collection capture/apply, thumbnail/info/INI write tidak dapat berjalan sebelum readiness dan tetap konvergen setelah ready.
- [ ] Time-to-first-shell dan time-to-first-list memenuhi budget pada fixture CI; total reconcile dicatat terpisah agar tidak disamarkan oleh async UI.
- [ ] Tidak ada full-scan storm, lost watcher event, partial DB projection, command-not-allowed regression, duplicate toast, atau progress listener leak.

**Performance budgets:**

- Workspace shell visible: target <= 300 ms setelah route entry.
- Last-valid/provisional list response: target <= 500 ms pada representative local SSD fixture; budget platform-specific dicatat untuk slow/external disk.
- First progress event: target <= 500 ms setelah recovery mulai.
- Progress event rate: maksimum sekitar 10 event/detik per active run, terminal event exempt.
- UI interactions selama scan: tidak ada main-thread long task > 50 ms yang berasal dari progress handling/render.

**Verification:**

- [ ] Targeted Rust/Vitest suites per task.
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test`
- [ ] `pnpm test -- --run`
- [ ] `pnpm lint`
- [ ] `pnpm i18n:lint`
- [ ] `pnpm build`
- [ ] Relevant WebdriverIO E2E and manual Windows test with large real Mods folder.
- [ ] `git diff --check` and focused read-only review.

**Dependencies:** Tasks 1–7.

**Files likely touched:** tests, generated bindings/permissions if needed, and `docs/command-permissions.md` or reconcile architecture docs.

**Estimated scope:** M.

## Checkpoints

### Checkpoint A — Contract and data safety

- Tasks 1–2 complete.
- One recovery per game/source generation.
- Workspace read does not grant mutation readiness.
- Watcher events cannot be lost while recovery is active.

### Checkpoint B — Visible async experience

- Tasks 3–4–6 complete.
- List remains visible and interactive while factual progress advances.
- Backend, not UI, remains final mutation authority.

### Checkpoint C — Efficient convergence

- Task 5 proves scoped ordinary operations and full offline recovery.
- Task 7 keeps only measured wins.
- Task 8 regression and release gates pass.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Last-valid snapshot terlihat stale selama sync | Medium | Persistent Syncing label, provisional styling, mutation disabled, authoritative terminal replacement |
| Shallow listing salah dianggap fully classified | High | Explicit provisional flag; capabilities read-only; no collection or mutation decisions from provisional nodes |
| Async recovery membuka race dengan mutation | High | Backend recovery readiness check before every physical mutation; existing per-game lock and operation lock remain authoritative |
| Progress tampak macet pada satu root sangat besar | Medium | Show current phase/root and elapsed time; do not fabricate percent; consider sub-unit instrumentation only if measured necessary |
| Parallel disk reads memperburuk HDD/external drive | Medium | Benchmark serial/bounded/current behavior; retain only measured improvement |
| Terminal result tiba sebelum listener ready | High | Workspace freshness is queryable state, not event-only; events are enhancement, terminal state remains recoverable by query |
| Refetch memicu reconcile loop | High | Workspace read is side-effect free; recovery kick/claim is separate single-flight operation |

## Explicit Non-Goals

- Tidak ada partial DB commit untuk progressive rendering.
- Tidak ada automatic conflict winner atau silent skip.
- Tidak ada persistent directory fingerprint yang menggantikan startup offline scan.
- Tidak ada polling interval, filesystem journal, additional long-lived cache, generic task engine, atau event bus baru.
- Tidak ada optimasi parser/concurrency yang dipertahankan tanpa before/after measurement.
