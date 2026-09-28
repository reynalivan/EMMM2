# Implementation Plan: Priority Game Indexing and Core Readiness

Tanggal: 2026-09-28

Status: implementasi inti berjalan (2026-09-28). Gate mutasi, promosi scan, handoff watcher, loading game terpilih, dan defer runtime sudah masuk; matriks acceptance di bawah tetap dipakai untuk pekerjaan serta verifikasi yang belum terbukti.

## 1. Goals dan batas scope

1. Onboarding membuka workspace setelah indeks wajib satu game selesai dan tervalidasi; tidak menunggu seluruh game.
2. Indexing game lain tetap otomatis di background. Memilih game mempromosikan job yang sama dan melanjutkan progres validnya, bukan memulai scan duplikat.
3. Selama indeks wajib target belum siap, tampilkan loading page pada workspace target dan tolak operasi mod di backend, termasuk hotkey dan bulk.
4. Setelah core-ready, enable/disable tetap optimistis, last-wins, storage-first. KeyViewer, runtime turunan, atau indexing game lain tidak menjadi syarat penyelesaian switch.
5. Disk tetap sumber kebenaran. Readiness, progres, identitas, dan hasil worker tidak boleh drift akibat perubahan folder, pergantian game, retry, atau restart.
6. Gunakan coordinator, watcher authority, mutation journal, dan runtime queue yang sudah ada. Tidak menambah dependency, generic job framework, atau persistent partial-index database pada tahap ini.

Non-goals: rewrite switch engine, mengubah semantik nama DISABLED, memblokir toggle karena hash overlap, merombak desain aplikasi, menaikkan versi/release, push, atau build installer. Resume parsial setelah process crash bukan janji tahap pertama.

## 2. Baseline audit

| Area existing                                                                           | Temuan                                                                                                              | Perubahan yang diperlukan                                                                         |
| --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `src/pages/onboarding/WelcomeScreen.tsx`                                                | Menunggu reconcile game pertama, lalu menyerahkan game lain ke background.                                          | Pertahankan first-game gate; sambungkan ke pemilik job yang juga dipakai aktivasi.                |
| `src-tauri/src/modules/reconciliation/application/disk_reconcile/onboarding_session.rs` | Preparation worker FIFO untuk semua game; snapshot single-use melalui oneshot; cancellation diperiksa antargame.    | Dedup job per game/root, promotion, cooperative yield, dan reuse hasil root valid.                |
| `src-tauri/src/modules/workspace/application/scanner/watcher/lifecycle.rs`              | Aktivasi memulai `prewarm_inactive_games`, di samping background onboarding.                                        | Jadikan prewarm requester pada scheduler yang sama, bukan pemilik scan terpisah.                  |
| `src/widgets/top-bar/GameSelector.tsx`                                                  | Menunggu background status; tidak mempromosikan job. Status yang tidak ditemukan mengizinkan aktivasi berikutnya.   | Pemilihan selalu meminta status/job authoritative; tidak menganggap status hilang sebagai ready.  |
| `.../disk_reconcile/emit.rs`                                                            | Helper gate hanya menolak `Syncing`.                                                                                | Allowlist readiness yang benar-benar siap; reject unknown, unstarted, failed, source unavailable. |
| `.../disk_reconcile/orchestrator/state.rs`                                              | `Completed(result)` dipetakan ke Ready tanpa memeriksa status result pada mapping tersebut.                         | Readiness mengikuti hasil core yang diterima, bukan sekadar selesainya future.                    |
| `.../disk_reconcile/disk_snapshot.rs`                                                   | Onboarding sudah melewati per-asset size metadata dan menggabungkan census dengan klasifikasi.                      | Pertahankan; refactor unit kerja tanpa mengembalikan traversal penuh tambahan.                    |
| `.../reconciliation/adapters/tauri/disk_reconcile_cmds.rs`                              | Onboarding sudah defer runtime/KeyViewer; snapshot dibandingkan dengan journal revision global.                     | Konsistenkan deferred runtime dan gunakan validitas per game/root.                                |
| `src-tauri/src/modules/system/application/app/bootstrap.rs`                             | Startup memakai jalur reconcile tersendiri dan melanjutkan pending onboarding. Request startup belum defer runtime. | Resume/startup memakai kontrak core yang sama; opsional tidak menahan readiness.                  |

Temuan ini adalah audit kode, bukan hasil benchmark. History lama tidak menggantikan perilaku source saat ini.

## 3. Kontrak arsitektur

### 3.1 Satu pemilik job dan satu sumber readiness

Refactor session indexing menjadi coordinator domain khusus indexing di modul reconciliation. Tetap gunakan `DiskReconcileState` untuk game authority dan mutation lease. Jangan membuat mutex operasi, readiness map frontend-authoritative, atau event bus baru.

- Key job: game ID + identitas/configuration revision Mods root. Nama folder/display name bukan identitas.
- Satu live core job untuk key yang sama. Request onboarding, startup, selector, dan prewarm bergabung ke job tersebut.
- Index generation mengidentifikasi pekerjaan core. Activation generation mengidentifikasi game pilihan UI. Berpindah A ke B tidak otomatis membuang progres indeks A yang masih valid.
- Hasil job A boleh memperbarui status indexing A setelah dipilih B, tetapi tidak boleh mengaktifkan A, menutup loading B, atau menerbitkan runtime A sebagai active game.
- Gunakan snapshot status saat subscribe/reconnect, lalu event versioned. Event lama tidak boleh menurunkan status/revision yang lebih baru.

State konseptual core: `NotReady -> Queued -> Scanning -> Finalizing -> Ready`; kegagalan menjadi `Failed` dengan sebab terstruktur dan opsi retry. Pekerjaan yang dipause tetap memiliki fase dan counters, tetapi tidak running. Adaptasikan enum existing; jangan mempertahankan dua enum readiness sebagai dua sumber keputusan independen.

`Ready` hanya boleh terbit jika:

1. Target root dan generation masih cocok.
2. Census, klasifikasi wajib, identitas, dan proyeksi inti lengkap.
3. DB commit berhasil; hasil reconcile benar-benar applied.
4. Perubahan disk yang teramati selama scan sudah diproses sampai watermark yang disepakati dan watcher continuity tersedia.
5. Tidak ada kegagalan recovery/root yang membuat authority tidak dipercaya.

`AppliedWithFolderConflicts` boleh core-ready: konflik folder dilaporkan dan dibatasi pada target terkait, bukan membekukan seluruh game. `SourceUnavailable` dan `NeedsRenameConfirmation` tidak otomatis dianggap ready; sediakan alur pemulihan yang dibatasi scope.

### 3.2 Gate mutasi

- Semua entry point mutasi biasa mensyaratkan core-ready game/root yang dituju: single/bulk toggle, object/ancestor switch, rename/move/trash, collection apply/restore, randomizer, metadata writes, import destination commit, serta preset/Safe Mode hotkey.
- Audit direct service callers selain command Tauri agar hotkey/background import tidak bypass gate.
- Preflight cepat boleh dilakukan sebelum lock untuk UX, tetapi readiness/generation harus diperiksa kembali setelah memperoleh lease yang melindungi perubahan authority. Ikuti lock ordering existing; jangan menahan mutex status saat await.
- Error typed, misalnya `GameIndexNotReady`, membawa game dan fase aman untuk UI; tidak memakai generic busy sebagai hasil normal indexing.
- Tidak antrekan intent mutasi dari loading page untuk dijalankan belakangan. User mengulangi tindakan setelah Ready. Intent yang sedang menunggu lock harus revalidate game/root/generation sebelum menulis disk.
- Background projection tertunda akibat switch sukses tidak mengembalikan game menjadi initial-indexing. Pertahankan bukti disk/journal dan settlement storage-first existing.
- Recovery internal, bootstrap journal recovery, dan resolusi konflik terarah memiliki jalur terbatas yang sah saat core belum siap. Jangan membuat deadlock dengan mewajibkan Ready untuk operasi yang justru menghasilkan Ready.

### 3.3 Pemisahan core dan enrichment

| Core wajib                                                          | Enrichment opsional                                      |
| ------------------------------------------------------------------- | -------------------------------------------------------- |
| Validasi root dan recovery journal yang relevan                     | Full KeyViewer/keybinding harvest dan publikasi overlay  |
| Census folder, stable identity, full relative path, parent/ancestor | Hash/overlap diagnostics yang tidak menentukan identitas |
| Klasifikasi minimum untuk menentukan mod/container                  | Exact storage sizes dan statistik dashboard              |
| Enabled/disabled dari filesystem                                    | Thumbnail generation dan enrichment tampilan             |
| Konflik nama folder nyata dan DB projection inti                    | Runtime/collection read model turunan                    |
| Watcher catch-up dan authority handoff                              | Telemetry aggregation                                    |

Audit dependensi sebelum memindahkan pekerjaan: collection membership atau metadata yang diperlukan menentukan target mutasi tetap core/on-demand authoritative, bukan membaca cache turunan yang stale. Jika safety classification belum tersedia, tampilkan unknown; jangan menganggap aman. Kegagalan enrichment tidak mengubah core-ready menjadi failed.

## 4. Scheduling, resume, dan anti-drift

### Prioritas kerja

1. Mutasi storage foreground pada game yang sudah Ready.
2. Core indexing/catch-up game yang dipilih.
3. Background core indexing game lain.
4. Enrichment opsional dengan concurrency dan beban I/O terbatas.

Urutan ini adalah kebijakan scheduling, bukan penambahan satu global lock besar. Transaksi/journal commit yang sudah berjalan diselesaikan pada batas aman; tidak diputus di tengah. Foreground intent memakai mekanisme coordinator existing untuk meminta background yield.

### Unit kerja dan checkpoint

- Mulai dari satu active core scan job, dengan parallelism internal terbatas dan diukur; jangan menjalankan full scan semua game sekaligus.
- Pecah discovery pada root yang sudah menjadi unit scanner. Simpan hasil root lengkap beserta validity evidence di memori.
- Tambahkan cooperative checks pada batas traversal/klasifikasi dalam root besar. Saat promotion perlu cepat, hentikan root yang belum lengkap pada batas aman; hanya root tersebut yang perlu diulang jika continuation lebih rumit daripada manfaatnya.
- Scheduler membaca ulang prioritas setelah yield. Promote game yang sedang running tidak membuat worker baru. Repeated selection bersifat idempotent.
- Pisahkan mahalnya discovery dari critical section DB commit. Setelah memperoleh lease untuk apply, periksa kembali validity evidence sebelum memakai snapshot.
- Jangan publish DB partial sebagai baseline lengkap atau menjalankan pruning terhadap library yang belum selesai ditemukan.
- Batasi retained paused snapshots dan watcher buffers. Budget ditentukan dari baseline fixture; jika terlampaui, evict checkpoint dan tandai perlu recheck secara eksplisit, bukan mempertahankan persentase palsu.

### Validasi perubahan dan handoff

- Watcher mulai sebelum discovery. Reuse existing authority, dirty-root tracking, expected-rename echo suppression, dan event-loss handling.
- Perubahan game A tidak menggugurkan snapshot game B hanya karena journal revision global bertambah. Validitas diikat ke target game/root dan relevant mutation evidence.
- Root yang dirty di-scan ulang secara scoped. Event yang ambigu/overflow atau watcher continuity putus mengeskalasi recheck dengan alasan tercatat.
- Finalization memakai watermark/generation, bukan syarat filesystem harus diam selamanya. Event setelah watermark tetap ditangani watcher dan preflight mutasi existing; perubahan sebelum publication tidak boleh hilang.
- Handoff onboarding ke activation membawa authority hasil core yang sah. Jangan menjalankan full scan lagi hanya karena entry point berganti.
- Path sama namanya pada parent berbeda tetap identitas berbeda. Enabled/disabled aliases hanya dianggap benturan jika dua entry disk berbeda benar-benar hadir pada scope parent yang sama; jangan memakai snapshot UI lama sebagai bukti konflik.

### Startup, restart, retry

- Pertahankan mutation-journal crash recovery sebelum membuka operasi pengguna.
- Runtime memory setelah restart berstatus belum terbukti ready. Gunakan DB terakhir sebagai cache, bukan bukti disk masih sama.
- Resume persisted pending-game list existing menjadi request coordinator. Tidak ada trusted-ready boolean baru atau schema migration wajib.
- Dahulukan core validation game aktif; game lain tetap background. Progress root process-local tidak dijanjikan survive restart.
- Retry menggunakan checkpoint yang masih valid; root/config berubah membatalkan generation lama. Source unavailable, permission error, dan scan failure tidak boleh memicu retry loop tanpa batas.
- Berhenti/TTL/eviction harus membebaskan watcher dan buffer job yang sudah tidak digunakan. Readiness tidak boleh tetap Ready setelah kehilangan authority relevan.

## 5. UX dan kontrak IPC

- Onboarding: pilih game awal; jika tidak ada pilihan eksplisit, gunakan game pertama existing. Simpan seluruh konfigurasi lalu tunggu core game awal saja.
- Game selector: langsung pilih target dan request activation/index priority; jika core belum siap tampilkan loading workspace target berdasarkan job authoritative.
- Loading: nama game, fase, jumlah root/folder yang sudah diproses, dan keterangan revalidation bila ada. Navigation kembali ke game Ready dan settings tetap bisa digunakan.
- Tidak ada persentase global buatan ketika denominator belum diketahui. Scan 100% belum berarti core 100%; final validation/commit ditampilkan sebagai fase tersendiri.
- Saat game sedang background 60% lalu dipilih, UI menampilkan progres job yang sama tanpa reset jika evidence masih valid.
- Error state menawarkan retry, ganti root, atau pilih game lain. Tidak ada overlay tanpa jalan keluar.
- Setelah Ready, loading hilang tanpa menunggu KeyViewer. Enrichment memakai status terpisah yang non-blocking.
- Konsolidasikan frontend status existing melalui satu selector/hook; React store hanya proyeksi backend. Missing status berarti belum diketahui, bukan ready.
- Utamakan perluasan command/status existing. Jika kontrak berubah: Rust DTO, registry, explicit permission allowlist, Specta generated bindings, mock/demo handlers terkait, dan i18n EN/ID/ZH berubah bersama.
- `docs/knowledge/tauri-command-registration.md` memuat lokasi generated file lama; gunakan exporter/repository path yang aktual, bukan menambah file binding duplikat.

## 6. Work packages dan acceptance gates

Checklist berikut adalah acceptance gate, bukan klaim bahwa setiap butir sudah lulus. Hasil verifikasi aktual dicatat di history implementasi.

### P0 — Baseline dan dependency map

- [ ] Catat timeline onboarding/activation: queue wait, discovery, classification, dirty-root recheck, DB apply, authority handoff, core-ready, enrichment.
- [ ] Inventarisasi semua mutation callers dan field yang benar-benar wajib core.
- [ ] Jalankan fixture 1 dan beberapa game, nested/flat, 100/1.000/10.000 folder, termasuk satu root sangat besar; ukur cold/warm terpisah.
- [ ] Catat jumlah full/scoped scans, duplicate jobs, RSS/retained snapshots, root rescans, dan foreground yield latency.

Gate: ada baseline repeatable dan tabel caller/gate; jangan mengklaim speedup hanya dari pemindahan await. Telemetry remote mengikuti opt-in existing dan tidak mengirim path/nama/file contents.

### P1 — Core readiness yang fail-closed

Files utama: `disk_reconcile/orchestrator/state.rs`, `disk_reconcile/emit.rs`, `disk_reconcile/types.rs`, shared errors, mutation command/service callers.

- [ ] Test unknown/unstarted/syncing/failed/source unavailable semuanya menolak mutasi biasa.
- [ ] Perbaiki mapping terminal result; hanya accepted applied result membuka gate.
- [ ] Hubungkan readiness ke root/generation dan lease-bound revalidation.
- [ ] Tutup bypass UI, direct command, hotkey, bulk, collection, dan import commit.
- [ ] Jaga recovery/conflict-resolution exception tetap narrow; buktikan tidak deadlock.

Gate: tidak ada filesystem mutation biasa sebelum core-ready, termasuk pada race readiness berubah ketika command menunggu lock.

### P2 — Single-flight indexing dan foreground promotion

Files utama: `disk_reconcile/onboarding_session.rs`, `disk_reconcile/disk_snapshot.rs`, `disk_reconcile_cmds.rs`; ekstrak module coordinator/chunk state yang kohesif bila tanggung jawab session tidak lagi jelas.

- [ ] Ganti FIFO preparation ownership dengan job dedup per game/root.
- [ ] Implementasikan promotion, cooperative yield, root checkpoint validity, dan bounded retention.
- [ ] Gabungkan subscriber ke job yang sama; snapshot status + monotonic events.
- [ ] Terapkan per-game/root validity; hilangkan invalidasi karena unrelated global revision.
- [ ] Jaga complete-baseline semantics pada apply/pruning dan scoped recheck.

Gate: memilih game ketiga tidak menunggu seluruh scan game kedua; completed valid roots dipakai ulang; tidak ada dua core workers untuk key yang sama.

### P3 — Satu alur onboarding, activation, background, dan startup

Files utama: `settings/adapters/tauri/settings_cmds.rs`, watcher `lifecycle.rs`, system `bootstrap.rs`, onboarding recovery persistence, reconciliation adapters.

- [ ] Route seluruh request core melalui coordinator; prewarm tidak lagi memulai scan independen.
- [ ] First-game onboarding handoff ke activation tidak men-scan ulang baseline yang masih valid.
- [ ] Preserve inactive watcher continuity untuk game yang sudah diindeks; batasi resource job yang belum selesai.
- [ ] Resume pending jobs startup dan prioritaskan active game; reject stale activation publications.
- [ ] Recovery/watcher catch-up selesai sebelum core-ready; dirty event tidak hilang pada pergantian owner.

Gate: satu lifecycle core dapat ditelusuri dari awal sampai Ready, termasuk restart dan rapid A -> B -> C -> A.

### P4 — Loading page dan status integrasi

Files utama: `WelcomeScreen.tsx`, `GameSelector.tsx`, `App.tsx` indexing overlay, `gameSlice.ts`, `useBackgroundIndexingStatus.ts`, workspace switch/query hooks, existing indexing UI, locale resources, IPC bindings.

- [ ] Tampilkan progress authoritative untuk selected game dan promote request yang idempotent.
- [ ] Hilangkan ready-by-absence dan competing frontend readiness decisions.
- [ ] Gunakan loading workspace yang tetap membolehkan pindah game; error/retry flow lengkap.
- [ ] Backend snapshot + events tahan event reorder, remount, dan response activation terlambat.
- [ ] Jangan load/render query workspace berat sebelum core-ready kecuali shell/status yang dibutuhkan.

Gate: selected game tidak operasional sebelum core 100%; loading terbuka segera tanpa menunggu scan; game lain yang Ready tetap dapat dipilih.

### P5 — Enrichment tidak memblokir core atau switch

Files utama: `orchestrator/run.rs`, `bootstrap.rs`, reconciliation `runtime_sync.rs`, existing runtime/KeyViewer scheduler dan consumers.

- [ ] Konsistenkan defer runtime pada onboarding/activation/startup.
- [ ] Queue enrichment setelah core commit, menggunakan existing latest-wins generation.
- [ ] Pastikan optional reads/work tidak memegang mutation lock selama pekerjaan mahal.
- [ ] Priority/yield berlaku untuk beban optional yang bersaing dengan storage; jangan membuat core gate menunggu optional worker selesai.
- [ ] Invalidate/publish derived results berdasarkan revision agar tidak menimpa state setelah toggle.

Gate: fake KeyViewer yang sangat lambat atau gagal tidak menahan core-ready, toggle disk commit, atau pergantian game Ready.

### P6 — Regression, benchmark, dan cleanup

- [ ] Jalankan matriks verifikasi di bawah; perbaiki failures sebelum menandai task selesai.
- [ ] Bandingkan baseline dan hasil pada fixture/hardware yang sama, terpisah cold/warm.
- [ ] Hapus owner scan, polling/status fallback, dan branches legacy yang sudah digantikan; tidak meninggalkan dua sistem aktif.
- [ ] Update architecture docs dan history berdasarkan perubahan serta hasil tes nyata.

Gate: semua acceptance functional hijau, tidak ada regression storage-first, dan laporan performa mencantumkan batas pengukuran. Push/build installer hanya pada permintaan release terpisah.

## 7. Matriks pengujian wajib

| Skenario                                                     | Bukti kelulusan                                                                                                  |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| Onboarding beberapa game                                     | Core game awal selesai tanpa menunggu game lain; background tetap berjalan kemudian.                             |
| Select game queued / partially scanned / Ready               | Promote atau reuse job; tidak duplicate scan; tidak reset progres valid.                                         |
| Rapid game selection dan repeated same-game clicks           | Latest selection menang; core job valid yang lama boleh lanjut background tanpa stale UI/runtime publication.    |
| Huge single root                                             | Cooperative priority check tidak menunggu seluruh game/root selesai tanpa batas; commit tidak diputus di tengah. |
| Semua mutation entry points saat NotReady/Failed             | Tidak ada rename/write; structured error; tidak ada delayed mutation setelah Ready.                              |
| Gate berubah ketika command menunggu lock                    | Revalidation menolak generation/root lama sebelum disk write.                                                    |
| Rage toggle setelah Ready, same/different mods, bulk         | Last-wins disk state; tidak muncul self-conflict atau indexing ulang karena runtime tertunda.                    |
| Background game B + storage switches game A                  | A tidak menunggu scan/enrichment B; snapshot B tidak invalid hanya karena revision global A.                     |
| External rename/create/delete selama scan/finalize           | Dirty roots diproses; final projection cocok disk; tidak ada orphan/prune dari partial snapshot.                 |
| Root diganti, watcher overflow/stop, permission/source error | Authority invalid; recheck/failure actionable; tidak false-ready.                                                |
| Nama sama beda parent; enabled/disabled real collision       | Beda path sah; real collision scoped; hash/key overlap tidak memblokir switch.                                   |
| Runtime result terlambat setelah toggle                      | Revision/generation lama tidak menimpa current projection/output.                                                |
| Crash sebelum/sesudah DB commit dan restart pending          | Journal recovery terjaga; cache tidak dipercaya tanpa validasi; pekerjaan tidak stuck Running.                   |
| DB apply gagal; retry; TTL/cancel/eviction                   | Tidak publish Ready; resource dibersihkan; retry memakai hanya checkpoint valid.                                 |
| IPC status snapshot/event race dan UI remount                | State terbaru tidak mundur; missing status tidak membuka operasi.                                                |
| KeyViewer blocked/failure; empty library                     | Core tetap bisa Ready secara benar; optional retry terpisah; empty library bukan forever-loading.                |

## 8. Target performa dan verification commands

Target berikut adalah acceptance target, bukan klaim hasil saat ini:

- Tidak ada await KeyViewer/enrichment dalam critical path core-ready atau toggle disk commit.
- Tidak ada duplicate core job untuk game/root/generation yang sama.
- Tanpa perubahan disk, promotion dan onboarding-to-activation handoff tidak mengulang completed roots.
- Di fixture terkontrol, selection -> loading shell dan optimistic switch feedback ditargetkan p95 <= 100 ms, diukur terpisah dari IPC/disk latency.
- Promotion dijalankan pada checkpoint aman berikutnya; ukur request-to-yield p95/max. Tidak menjanjikan batas wall-clock ketika OS filesystem call sedang blocking.
- Waktu core-ready game pertama tidak bertambah proporsional jumlah game terkonfigurasi. Ukur first-game-only versus multiple-games untuk membuktikan background contention terkendali.
- Performa storage-first dengan background indexing harus dibandingkan kondisi background off. Tentukan tolerance regression dari baseline noise sebelum mengubah scheduler, bukan sesudah melihat hasil.

Validation setelah implementasi: targeted Rust state/concurrency tests, frontend interaction tests, existing storage-first regression suites, command registry test, full library/frontend suites sesuai dampak, typecheck, lint, formatting, dan production frontend build. Native Windows fixture diperlukan untuk filesystem identity/watcher behavior; browser mocks tidak membuktikan race NTFS.

Perintah dasar yang tersedia (jalankan bounded/background bila lama):

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

Pisahkan baseline failures yang terverifikasi dari failures baru. Jangan menonaktifkan checks atau menandai gate hijau jika belum dijalankan. Export bindings mengikuti setup exporter aktual; review hasil generate dan rerun typecheck setelahnya.

## 9. Risiko dan kontrol scope

- Strict gate dapat membuka caller yang sebelumnya bergantung pada default permissive: selesaikan inventory/gate tests sebelum UI rollout.
- Chunking dapat menambah state rumit: simpan hanya complete-root checkpoints; partial root boleh diulang, tidak perlu serialisasi traversal stack lintas restart.
- Scanning disk aktif tidak pernah menjamin tidak ada external write sesudah Ready: pertahankan identity-validating preflight dan watcher authority, bukan menjanjikan snapshot kekal.
- Background otomatis tetap punya biaya I/O/RAM. Batasi kerja, yield ke foreground, dan ukur; jangan menyebutnya gratis hanya karena async.
- Jangan menambah durable schema bila existing recovery list dan in-memory status cukup. Perubahan schema yang terbukti perlu harus diajukan terpisah dengan alasan dan migration plan.
- Implementasi diselesaikan sebagai satu perubahan perilaku utuh sebelum release; jangan merilis strict gate tanpa bootstrap/activation path yang dapat menghasilkan Ready.

Definition of done: core-readiness terbukti fail-closed, automatic indexing tetap efektif dan promotable, UI melanjutkan progres yang sah, disk/DB convergent setelah finalization, serta operasi storage-first sesudah Ready tidak menunggu pekerjaan opsional.
