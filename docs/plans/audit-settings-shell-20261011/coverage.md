# Cakupan audit lanjutan — 11 Oktober 2026

Peta ini melanjutkan [cakupan sebelumnya](../audit-dashboard-preview-20261011/coverage.md)
dan [validasi perbaikan Dashboard/preview](../audit-dashboard-preview-fixes-20261011/validation.md).
Status selalu berlaku pada subalur dan snapshot yang disebutkan. Tidak ada fitur
yang dinyatakan selesai seluruhnya hanya karena pernah dibaca atau diuji.

| Fitur                                       | Alur yang sudah diperiksa                                                                                                                                | Masih terbuka                                                                                   |
| ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Settings                                    | General, Games, Trust, theme, AI draft/key, homepage, updater admission; pass ini privacy/logs/maintenance dan lifecycle GameForm                        | Install/restart updater nyata, credential/API, positive reset, sink consent race runtime        |
| Catalog / MasterDB                          | Cache generation, mapping, rollback/retry, pending-preview cleanup                                                                                       | Live GitHub/picker dan abrupt crash                                                             |
| Mods Manager                                | Toggle, metadata, bulk safety, move, selection/paging, empty search, junction refusal                                                                    | Kombinasi CRUD/conflict lengkap, symbolic selection dan daftar besar                            |
| Preview / gallery / thumbnail               | Hide/restore, fullscreen, clear/restart, texture toggles, captured mutation target/cache invalidation; sembilan Dashboard/preview native terbaru lulus   | Clipboard/picker race native, recycle positif, thumbnail age/cleanup race                       |
| INI editor / KeyViewer                      | Draft/hash ownership, stale completion, repair, save/discard/cancel, normal sync                                                                         | Render dan reload 3DMigoto nyata                                                                |
| Mod Inbox / import / archive                | Destination/source ownership, containment, cancellation/retry, quota, native routing/publication                                                         | OS drag, dua physical volumes, interrupted native import dan Linux                              |
| Matching / reconciliation                   | Scoring/aliases, game-owned IDs, preview/commit parity, pinned publication, host overlapping/stale callbacks                                             | Full crash/restart dan Linux matrix                                                             |
| Collections / Safe Mode / Randomizer        | Safety revalidation, persistent intent, preview/apply/restart, loadout ownership                                                                         | Full-app deferred completion crash dan native interleaving                                      |
| Storage Optimizer / duplicates              | Identity, root-bound report, strict errors, whitelist, cancel/commit, event ownership                                                                    | Nonexact/large scan dan partial-error matrix                                                    |
| Discover / Browser / Downloads              | Address/scheme, filename/title/reservation, transfer order, tab ownership, protected history                                                             | Live navigation/adblock/Internet dan semua transfer actions native                              |
| File-lock / mutation recovery               | Retry/unlock/cancel, owned process control, capability refusal, path race, durable receipts                                                              | Semua structural entry points, kernel/directory holders, Safe Mode crash                        |
| Watcher / suppression                       | Owner release, echo/convergence, switch/restart reuse, image catch-up/source integration                                                                 | Seluruh failed-coverage/recovery interleavings                                                  |
| Global hotkeys / overlay                    | Ordinary/physical native, Borderless/Fullscreen, passive notices, expiry/foreground; production install terverifikasi di history 032                     | Installed runtime/actual loader rendering, hardware click-through dan multi-monitor/DPI         |
| Onboarding / startup / indexing / Dashboard | Index/restart/recovery, census/scan reuse, Dashboard DTO/actions/async review; native release 6.000+80 onboarding serta sembilan Dashboard/preview cases | Cold SLA/WPR attribution masih di task performance; chart/action matrix lengkap                 |
| Active Conflict Review                      | Ignore/restore/filter, overlapping/stale decisions, busy unit cancel, idle native Escape/reopen                                                          | Busy native cancel, ignore persistence/settlement dan daftar besar                              |
| Games / launcher / Mod Health / trash       | Parsing/identity, provenance, viewer lease/receipt ownership; pass ini active-game reads dan launch/auto-close callers                                   | Real game/loader execution, health matrix penuh, positive recycle dan Linux                     |
| App shell / Top Bar                         | Baru: navigation composition, compact launch target lifecycle, resize cleanup, shared action portals                                                     | Compact native navigation/gesture dan keyboard/focus matrix lengkap                             |
| Diagnostics / telemetry                     | Baru: frontend sanitizer vs SDK, consent/producer/persistence boundaries, logs/reset protections                                                         | Fake-transport privacy test, deterministic Rust opt-out concurrency, real export/consent native |

## Hasil pass ini

- Lima temuan melewati challenge independen dan penilaian akhir; lihat [laporan](report.md).
- Empat assertion RED membuktikan tiga masalah frontend: compact portal, resize
  cleanup, serta dua subkasus GameForm. Ini repro defect, bukan suite hijau.
- Dua masalah telemetry ditelusuri dari source terpasang/backend; tidak ada data
  yang dikirim ke endpoint telemetry dalam audit ini.
- Primary ledger: 23 file dibaca penuh, settings commands hanya alur terkait.
  Backend maintenance review menambah sembilan source penuh; file besar lainnya
  dibaca pada range yang dicatat di `backend-reviewed.json`. Tidak ada klaim bahwa
  file yang dibaca sebagian sudah tercakup penuh.
- Primary 24-input snapshot tetap sama sejak triage/repro/review. Ledger backend
  32 input yang diambil setelah pembacaan juga cocok pada verifikasi akhir; ini
  tidak membuktikan tidak ada perubahan selama pembacaan awal backend.

## Bukti dan batas

- Canonical Hunter/Skeptic/Referee/scan-report/coverage di ignored
  `.bug-hunter/audit-settings-shell-20261011/`; kelima schema checks lulus.
- Repro/config/log di `.audit-output/audit-settings-shell-20261011/`.
- Native 15/15 sebelumnya dan installed update berasal dari task terkait,
  [history 026](../../history/202610110026-measure-indexing-and-enable-native-release.md)
  dan [history 032](../../history/202610110032-install-hotkey-feedback.md).
- Audit ini tidak mengubah source/config/test produksi, dependency atau data
  pengguna; tidak mengambil Cargo/build/native/driver slot. Isolated Vitest hanya
  berjalan pada slot yang diberikan task performance dan sudah dilepas kembali.
- Dugaan reset browser retention, thumbnail age/cleanup dan input besar get_logs
  belum dinaikkan menjadi bug terkonfirmasi; kebutuhan buktinya ada di laporan.
