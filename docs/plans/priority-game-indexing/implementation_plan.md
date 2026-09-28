# Implementation Plan: Simplify Indexing and Disk-First Switching

Tanggal: 2026-09-28

Status: disetujui, implementasi bertahap berjalan; gate P0–P5 belum dinyatakan selesai.

Baseline audit: main pada `c355466`.

Menggantikan rencana sebelumnya pada path yang sama. Fitur existing yang sudah benar dipertahankan. Checklist adalah pekerjaan/bukti yang perlu diselesaikan, bukan klaim bahwa semua fiturnya belum ada. Perubahan kode parsial dan hasil verifikasi dicatat terpisah di `docs/history/`.

## 1. Outcome dan scope

Prioritas wajib:

1. Switch memberikan feedback optimistis langsung dan menerima intent terbaru.
2. Validasi keselamatan minimum, rename folder fisik, verifikasi disk, durable receipt.
3. Proyeksi DB tertunda diselesaikan melalui recovery journal.
4. Runtime, collection preview, ancestor read model, KeyViewer, dan UI refresh menyusul.

Identitas, path/ancestor, dan collection membership yang diperlukan menentukan target mutasi tetap authoritative pada preflight. Data keselamatan tidak boleh dipindahkan ke background lalu dibaca dari cache stale.

Goals:

- Onboarding menunggu core game pertama saja; game lain otomatis background.
- Memilih game belum siap mempromosikan job yang sama dan menampilkan loading workspace.
- Game ready tidak kembali initial-loading akibat switch sendiri atau optional runtime pending.
- Satu owner readiness backend, satu jalur request core, satu jalur publication ke UI.
- Tidak ada self-conflict, stale overwrite, duplicate core job, atau refresh karena TTL tanpa bukti perubahan.
- Folder disk adalah sumber kebenaran; posisi switch optimistis bukan bukti sukses disk.

Non-goals: rewrite switch engine, generic scheduler/event bus baru, dependency baru, schema migration spekulatif, persistent partial-scan checkpoints, redesign visual, push, atau build installer.

## 2. Baseline audit

| Source                                | Temuan                                                                      | Konsekuensi untuk plan                                                                |
| ------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| WelcomeScreen / App                   | First-game reconcile diikuti setActiveGameId dan waitForGameActivationReady | Trace handoff; jangan hanya mengukur scan                                             |
| settings_cmds / watcher lifecycle     | Activation reset recovery; watcher dapat reuse authority atau catch-up      | Hilangkan reset yang tidak perlu setelah proof valid; bukan asumsi selalu full rescan |
| workspace_cmds                        | get_workspace_structure memulai initial recovery                            | Getter harus pure setelah explicit ensure tersedia di semua entry point               |
| orchestrator/state                    | Readiness bergantung recovery result, revision, dan watcher authority       | Pisahkan core baseline, dirty scopes, dan optional projection                         |
| useFolderGridViewModel                | Gate menggabungkan activation/recovery/source error/rename report           | Satu ambiguity report dapat mengunci seluruh game                                     |
| useFileWatcher                        | TTL 5 detik; queued refresh dapat melewati shouldSync                       | Gabungkan kebutuhan refresh berdasarkan watermark                                     |
| workspaceSwitchOps / reconcileRefresh | Switch projection dan watcher sama-sama publish refresh                     | Satu publication entry point dengan revision dedup                                    |
| Onboarding reconcile                  | Runtime/KeyViewer sudah defer_overlay_sync                                  | Memindahkan await yang sudah deferred bukan solusi terbukti                           |
| Progress onboarding                   | Per-game events, empat tahap tidak mencakup seluruh handoff                 | Snapshot dan job-versioned progress                                                   |

Audit sebelumnya menjalankan 43 tes terkait onboarding, watcher, switch control, dan activation wait: lulus. Ini bukan reproduksi native end-to-end. Log lokal yang diperiksa kosong; bottleneck dominan Finishing, gate aktif pada screenshot, dan disk latency aktual belum terbukti. P0 harus mengukurnya sebelum klaim perbaikan performa.

## 3. Kontrak arsitektur

### 3.1 Satu owner, fakta yang tidak dicampur

Perluas DiskReconcileState/coordinator existing, bukan menambah service paralel. Reuse enum/counter existing jika semantiknya sesuai.

| Fakta            | Makna                                                 | Pengaruh terhadap operasi                                      |
| ---------------- | ----------------------------------------------------- | -------------------------------------------------------------- |
| Core proof       | Game + root epoch, accepted baseline, status job      | Belum valid: loading dan reject mutasi biasa                   |
| Disk authority   | Watcher continuity/watermark, dirty scopes, ambiguity | Revalidate affected scopes; unknown scope dapat memblokir root |
| Disk commit      | Durable journal/receipt revision                      | Bukti rename, path rewrite, settlement                         |
| Derived progress | Projected/published revision per consumer             | Non-blocking; tidak mengubah core-ready                        |
| Active selection | Activation generation                                 | Memilih tampilan, bukan membatalkan core game lain             |

- Root epoch berubah saat konfigurasi/identitas root berubah. Job generation berbeda dari activation generation.
- Ready bukan persamaan baseline revision dengan setiap revision runtime terbaru.
- Ready bukan izin permanen mengabaikan perubahan eksternal: mutation preflight tetap memeriksa authority dan identity.
- Status/progress membawa game/root/job identity dan monotonic sequence. Jangan menyamakan counters berbeda domain.
- Subscribe listener dahulu, ambil snapshot, buffer event selama snapshot, lalu merge menurut versi. Remount/reconnect mengambil snapshot lagi.
- Hasil/terminal event job lama tidak menghapus status job baru.
- Frontend hanya projection backend dan satu selector eligibility; jangan mempertahankan readiness authority kedua.

### 3.2 Core indexing dan handoff

- Core wajib: root/journal recovery, census, stable identity, path/ancestor, klasifikasi minimum, state enabled dari disk, real folder conflicts, DB baseline applied, dan watcher catch-up.
- Optional: full keybind harvest, hash/overlap diagnostics, exact size/statistics, thumbnails, runtime/collection preview turunan. Audit dependensi sebelum memindahkan kerja.
- Watcher mulai sebelum discovery; activation mengadopsi proof dan watcher onboarding yang cocok, tanpa reset/recovery kedua.
- Perubahan sampai watermark diproses sebelum Ready; perubahan sesudahnya ditangani watcher/preflight. Jangan menunggu disk diam selamanya.
- AppliedWithFolderConflicts boleh ready dengan blocked scopes. Ambiguity yang cakupannya belum terbukti tetap fail-closed.
- Getter workspace hanya membaca. Onboarding/startup/activation menggunakan explicit ensure/join/promote yang idempotent.
- Ready membuka dashboard tanpa menunggu background scan atau enrichment; enqueue background tidak boleh await penyelesaian scan.
- Cold startup memvalidasi disk dan pending journal; persisted DB bukan trusted-ready flag.

### 3.3 Switch disk-first, last-wins, dan konflik

Pertahankan executor, OperationLock, mutation journal, receipt, optimistic handling, dan toast aggregator existing.

```text
latest intent + optimistic UI
  -> lease/preflight core, root, identity, parent, destination
  -> journal prepare -> non-overwriting rename -> disk verification
  -> durable disk receipt -> settle current intent / aggregate toast
  -> deferred projection -> revision-aware UI/runtime/KeyViewer publication
```

- Tidak ada debounce sebelum dispatch rename pertama; debounce toast tidak menunda storage.
- Pending intent per physical identity diganti terbaru. Rename in-flight diselesaikan aman, lalu executor memeriksa intent terbaru.
- Receipt/error lama tidak menghapus optimism atau rollback hasil intent lebih baru.
- Parent/child diserialisasi pada scope yang memengaruhi path; disjoint folders tidak menunggu enrichment.
- Resolve source aktual dari identity/journal, bukan nama UI stale. Periksa source/target/parent/containment lagi setelah lease.
- Alias target yang merupakan folder sama adalah path reconciliation/no-op, bukan duplicate. Nama sama beda parent sah.
- Target ditempati entry lain adalah real conflict; rename tidak overwrite walau collision muncul setelah preflight.
- Hash/resource overlap/runtime keys tidak memblokir toggle.
- Path busy memakai bounded retry + identity revalidation; setelah batas habis laporkan error dan observed disk state. Tidak ada infinite retry.
- Ambiguity hanya memblokir subtree/dependents yang terbukti terkait; jika scope tidak diketahui, recovery tetap fail-closed.
- Conflict-fix revalidate candidate identity dan epoch ketika commit; tidak memakai report stale.
- Last-wins untuk toggle tidak berarti membatalkan collection/bulk transaction di tengah. Overlap memakai ordering/journal existing; request berikutnya revalidate sesudah transaksi.
- Bulk mempertahankan rollback/recovery. Jangan menjanjikan atomic multi-folder rename OS.
- No-op/superseded tidak menghasilkan success toast. Pertahankan ringkasan receipt sah 500 ms, serta discard feedback stale-game.
- Projection failure sesudah disk commit tidak membatalkan receipt; journal tetap pending. Mutasi berikutnya memakai journal-backed identity/scoped recovery bila DB belum aman dipakai.

### 3.4 Watcher, scheduling, dan publication

- Semua onboarding/startup/activation/prewarm menjadi requester coordinator existing; satu core job per game/root epoch.
- Prioritas: ready-game foreground storage, selected core, background core, optional enrichment.
- Promote bergabung ke job/progress valid. Background concurrency dan snapshot retention bounded.
- Tambahkan traversal chunk/yield hanya jika trace membuktikan root besar memblokir foreground. Reuse checkpoint lengkap per root; tidak membuat persistent traversal stack.
- Pertahankan lock ordering. Jangan menahan mutex status saat await atau melepas mutation lease di tengah commit.
- Expected watcher echoes diakui memakai journal evidence, bukan global suppression window yang membuang external events.
- Focus/ModsViewEntered memakai continuity/dirty-generation check; healthy+clean tidak traversal.
- Queued requests digabung dengan union scopes dan latest required revision; sesudah run, recheck watermark sebelum run berikutnya.
- Full scan hanya untuk initial baseline, root epoch berubah, event loss/unknown scope, recovery yang memerlukannya, atau explicit force-full.
- Satu publisher committed changes menerima receipt/projection dan watcher results: dedup revision, patch path/selection segera, invalidate affected scopes setelah projection tersedia.
- No-op tidak broad-invalidate; refetch/result lama tidak menimpa disk receipt terbaru.
- Optional workers coalesce target revision terbaru dan cek root/generation/activation sebelum publish.
- Hilangkan competing frontend hydration/full-reconcile maps setelah authoritative status menggantikannya.

## 4. Work packages

Semua checklist dimulai belum diverifikasi untuk revisi ini. Setiap package harus mempunyai regression test sebelum perubahan state/concurrency dan bukti gate setelahnya.

### P0 — Reproduksi, trace, baseline

Files: workspace_cmds timing, onboarding reconcile, orchestrator/run, watcher/lifecycle, App/WelcomeScreen, fixture/tests existing.

- [ ] Trace correlation job/mutation, root epoch, revision, gate reason, queue/lock wait, scan, DB apply, handoff, ready, projection.
- [ ] Ukur click -> optimistic paint, dispatch -> rename mulai, rename -> receipt, receipt -> projection.
- [ ] Hitung full/scoped scans, refetch, duplicate jobs, retained snapshots, foreground yield latency.
- [ ] Reproduksi onboarding -> first switch, rapid switch -> refresh, clean focus/re-entry.
- [ ] Fixture Windows terisolasi: 1/5 games, 100/1.000/10.000 folders, flat/nested/large root, cold/warm.
- [ ] Tetapkan noise/tolerance sebelum eksperimen. Telemetry mengikuti opt-in; tidak mencatat nama/path/file contents sensitif.

Gate: blocker punya trace dan regression yang reproduktif; keterbatasan reproduksi native dinyatakan. Tidak stress-test library asli pengguna.

### P1 — Single readiness owner dan mutation eligibility

Files utama:

- src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/state.rs
- src-tauri/src/modules/reconciliation/application/disk_reconcile/emit.rs
- src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs
- Reconciliation DTO/status adapter dan mutation service callers terkait.

- [ ] Core proof, dirty authority, dan projection status mempunyai kontrak terpisah pada owner existing.
- [ ] Semua entry point memakai ensure-core sebelum getter recovery side effect dihapus.
- [ ] Eligibility backend scoped; recheck root/generation/identity setelah lease.
- [ ] Audit single/bulk/object toggle, collection apply/restore, conflict fix, rename/move/trash, import commit, hotkey/randomizer, metadata write, direct service callers.
- [ ] Recovery exceptions narrow dan tidak deadlock.
- [ ] Snapshot/events versioned; Rust DTO, bindings, registry/permissions bila berubah, demo mocks dan i18n ikut diperbarui.
- [ ] Hapus competing readiness decisions setelah caller dimigrasikan.

Gate: initial/unknown/failed menolak unsafe write; optional pending tidak menolak safe toggle; root berubah saat menunggu lock menolak stale write.

### P2 — Onboarding/activation/background satu lifecycle

Files utama: disk_reconcile/onboarding_session.rs, disk_snapshot.rs, reconciliation/adapters/tauri/disk_reconcile_cmds.rs, settings/adapters/tauri/settings_cmds.rs, watcher/lifecycle.rs, system/application/app/bootstrap.rs.

- [ ] Semua requester memakai ensure/join/promote sama; prewarm tidak menjadi owner scan independen.
- [ ] Handoff mengadopsi proof + watcher continuity, tanpa unconditional reset.
- [ ] Background pending-game list tetap recoverable; checkpoint process-local tidak dipercaya setelah restart.
- [ ] Selection queued/partial/Ready reuse pekerjaan sah; stale activation tidak publish active state.
- [ ] Defer optional runtime konsisten; audit contention lock/DB, bukan hanya await caller.
- [ ] Bounded watchers/snapshots; abandoned epoch dibersihkan; partial baseline tidak melakukan pruning.

Gate: first core -> dashboard tidak menunggu game kedua; clean handoff tidak scan ulang; promotion tidak duplicate job.

### P3 — Watcher dan refresh convergence

Files utama: watcher/lifecycle.rs, reconciliation orchestrator, src/features/file-watcher/hooks/useFileWatcher.ts, reconcileProgress.ts, reconcileRefresh.ts, src/features/workspace-runtime/actions/workspaceSwitchOps.ts, existing query refresh bus.

- [ ] TTL traversal diganti continuity/dirty checks.
- [ ] Queued refresh memakai watermark recheck dan scope union.
- [ ] Receipt dan watcher result memakai satu publication entry point dengan dedup.
- [ ] Journal echo acknowledgement tidak kehilangan external changes.
- [ ] Event/snapshot/refetch/progress terminal tahan reorder.
- [ ] Listener lifecycle stabil; snapshot menutup registration/remount gap.
- [ ] Hapus maps/timers/branches refresh lama yang sudah digantikan.

Gate: clean focus menghasilkan nol scan; internal toggle tidak full-scan/initial-loading; external change tetap converges.

### P4 — UI gate tunggal dan disk-first settlement

Files utama:

- src/app/store/appStore/gameSlice.ts
- src/app/entrypoint/App.tsx, waitForGameActivationReady.ts
- src/pages/onboarding/WelcomeScreen.tsx, hooks/useOnboardingDiskProgress.ts
- src/widgets/top-bar/GameSelector.tsx
- src/widgets/mod-explorer/hooks/useFolderGridViewModel.ts, components/FolderGridSyncToast.tsx
- src/features/workspace-runtime/actions/useWorkspaceSwitchActions.ts
- FolderCard, WorkspaceSwitchControl, locale resources terkait.

- [ ] Satu selector authoritative eligibility, tanpa OR beberapa cache readiness.
- [ ] Safe switch tetap clickable selama optional work; receipt/error stale tidak rollback optimism baru.
- [ ] Scoped conflict tidak mengunci unrelated nodes; initial indexing tetap melindungi workspace.
- [ ] Progress scan/commit/catch-up/handoff jujur, tanpa fake percentage.
- [ ] Ready snapshot membuka dashboard; optional status tidak memakai blocking disk-refresh banner.
- [ ] Loading tetap menyediakan retry/ganti game/settings, tanpa delayed user mutation setelah ready.
- [ ] Pertahankan Explorer disabled-path fix, ancestor-effective state, disk-verified aggregated toast.

Gate: optional pending tidak membuat Enabled-grey; game switch tidak membawa stale progress/toast.

### P5 — Native regression, benchmark, cleanup

- [ ] Jalankan matriks bagian 5 dan baseline comparison pada hardware/fixture sama.
- [ ] Hapus scan owner/gate/polling/branches yang telah digantikan; pertahankan snapshot fallback recovery yang memang diperlukan.
- [ ] Review lock ordering, journal recovery, stale publication, collision, dan simplifikasi module.
- [ ] Jalankan typecheck/lint/build, Rust tests/checks, registry/bindings validation sesuai dampak.
- [ ] Catat hasil nyata di architecture/history; pisahkan baseline failure dari regression.

Gate: acceptance functional hijau, tidak dual-authority, storage-first tidak regress, dan pengukuran native terdokumentasi.

Urutan: P0 -> P1 -> P2 -> P3 -> P4 -> P5. Contract dan callers berpindah dalam slice kompatibel. Jangan merilis pure getter tanpa ensure entry points atau menghapus TTL sebelum continuity recovery teruji.

## 5. Matriks regresi wajib

| Skenario                                                         | Assertion                                                                                        |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Onboarding 1/5 games                                             | First-game ready membuka dashboard; background tidak menahan; clean handoff tanpa duplicate scan |
| NotReady/failed/source unavailable                               | UI, IPC, hotkey, bulk, collection tidak write; retry dapat pulih                                 |
| A -> B -> C -> A; queued/partial/Ready                           | Satu job per epoch, latest selection, reuse valid progress                                       |
| Getter/focus/remount clean                                       | Tidak memulai recovery/scan; status snapshot tidak kehilangan event                              |
| 100/1.000 rapid intents satu mod                                 | Final intent sesuai disk, pending bounded, tidak self-conflict                                   |
| Banyak mods + bulk/collection                                    | Ordering overlap deterministik; final disk/UI/DB converge                                        |
| Parent/child bersamaan                                           | Path rewrite benar dan effective-disabled mencakup ancestor                                      |
| Watcher echo + external rename                                   | Echo tidak full-scan; external change tidak hilang                                               |
| Nama sama beda parent                                            | Identitas berbeda, tanpa duplicate mods.id/false conflict                                        |
| Real enabled/DISABLED collision muncul sebelum/sesudah preflight | Tidak overwrite; conflict scoped dan berdasarkan disk                                            |
| Conflict-fix bersamaan indexing/switch                           | Candidate epoch/identity revalidated; stale report ditolak                                       |
| Busy/permission/DB failure                                       | Bounded retry, error actionable, receipt tidak bohong                                            |
| Watcher overflow/restart/root berubah                            | Authority revoked sesuai scope; recovery sebelum unsafe write                                    |
| Runtime/KeyViewer sengaja diblokir                               | Core-ready dan disk receipt selesai; latest output setelah unblock                               |
| Projection failure setelah rename                                | Journal recovery, disk success tidak di-rollback refetch lama                                    |
| Event/snapshot/query response reorder                            | Latest revision menang; old terminal tidak menghapus new progress                                |
| Crash sebelum/sesudah rename/sebelum projection                  | Recovery converges; persisted DB tidak false-ready                                               |
| Large root/background games                                      | Foreground latency dan memory bounded terukur                                                    |
| Collection preview/KeyViewer                                     | Hanya effective-enabled, empty-state tetap tersedia, preset overlay independen                   |
| Rapid/multi toast dan pindah game                                | Hanya receipt sah, tidak spam/stale feedback                                                     |
| Empty library                                                    | Core dapat ready; bukan infinite loading                                                         |

Race tests memakai barriers sebelum rename, setelah rename, sebelum DB/publication, dan handoff watcher. Native Windows/NTFS fixture wajib untuk identity, collision, sharing violation, watcher; browser mocks tidak membuktikan semuanya.

## 6. Target dan verification

Target berikut bukan klaim hasil saat ini:

- Optimistic paint p95 <= 100 ms pada fixture terkontrol; catat main-thread stalls.
- Nol await optional enrichment pada disk receipt/core-ready critical path.
- Satu core job per game/root epoch; nol extra scan pada clean handoff.
- Nol traversal pada healthy clean focus; no-op tidak broad-refetch.
- Pending work bounded pada 1.000 intents; intermediate stale intents tidak dieksekusi semua.
- Disk receipt p50/p95/max dan lock wait diukur background off/on. Budget numerik ditetapkan pada P0 sesuai hardware/noise sebelum optimasi.
- Tidak menjanjikan disk rename instan saat OS/antivirus memegang handle; kegagalan tetap terkontrol.
- First-game ready tidak menunggu seluruh game; biaya background I/O tetap diukur.
- Correctness simplification dibuktikan invariant dan penghapusan duplicate ownership; performance-only changes harus mengalahkan baseline noise.

Commands setelah implementasi, bounded sesuai setup:

```powershell
rtk pnpm exec tsc --noEmit
rtk pnpm lint
rtk pnpm lint:arch
rtk pnpm exec vitest run
rtk pnpm build
rtk cargo check --manifest-path src-tauri/Cargo.toml --lib
rtk cargo test --manifest-path src-tauri/Cargo.toml --lib
rtk cargo test --manifest-path src-tauri/Cargo.toml every_registered_command_is_allowed_by_the_app_permission
rtk cargo test --manifest-path src-tauri/Cargo.toml specta_tests::export_bindings
rtk cargo fmt --manifest-path src-tauri/Cargo.toml --check
rtk cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings
rtk git diff --check
```

Jangan bypass checks. Jika pnpm mencoba reinstall/purge saat validation, jangan auto-approve purge; pakai compatible installed runner atau laporkan blocker. Review generated bindings dengan exporter aktual, bukan menambah file duplikat.

## 7. Scope safety dan definition of done

- Tidak ada schema migration direncanakan; kebutuhan baru diajukan terpisah.
- Format mutation journal/recovery dipertahankan.
- Small coherent commits, tanpa dual-engine feature flag permanen.
- Rollback kode tidak membalik rename pengguna yang sudah committed; recovery memakai disk/journal aktual.
- Jangan mengorbankan validasi untuk benchmark hijau.
- Approval implementasi tidak mencakup push/release atau stress-test library asli pengguna.

Selesai jika first-game handoff tidak duplikasi indexing, satu readiness authority, disk-first last-wins terverifikasi, watcher tidak kehilangan external changes, konflik nyata aman, optional failure tidak membekukan switch, serta hasil native stress/benchmark tercatat.

## 8. Bukti implementasi lokal (2026-09-28)

- Fixture Windows terisolasi dengan 100/1.000/10.000 mod mengukur reconcile onboarding pertama sekitar 37 ms/212 ms/4,54 s. Reconcile full tanpa perubahan sekitar 25 ms/132 ms/2,98 s; sebelumnya pada fixture 10.000 mod sekitar 13,16 s dan melaporkan perubahan folder palsu. Ini bukan pengukuran latency UI pada library pengguna.
- Tes no-op membuktikan key relatif snapshot dan key absolut DB tidak lagi memicu identity staging berulang, size traversal opsional, atau UPDATE collection binding yang nilainya sama.
- Tes watcher nyata dan state terisolasi mencakup takeover clean, event terlambat yang membatalkan proof, startup reuse, prewarm yang tidak menahan activation lock, serta revisit setelah budget prewarm habis. Tes coordinator mencakup 1.000 intent cepat pada satu folder; tes bulk/konflik mencakup identity dan destination collision.
- Validasi akhir: 1.306 tes Rust lulus (12 benchmark/manual ignored), 1.100 tes frontend lulus (1 skipped), TypeScript, ESLint, architecture lint, Vite build, Cargo check, Cargo Clippy `-D warnings`, Rust format, dan diff check lulus.
- Validasi otomatis tidak menggantikan pengukuran click-to-paint atau disk receipt p95 di aplikasi terpasang dengan antivirus dan koleksi riil. Target numerik tersebut tetap perlu observasi pada lingkungan pengguna sebelum dinyatakan tercapai.
