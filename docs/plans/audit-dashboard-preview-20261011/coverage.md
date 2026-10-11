# Coverage audit fitur — 11 Oktober 2026

Ini memperbarui [peta sebelumnya](../audit-next-20261011/coverage.md).
Coverage historis berlaku pada snapshot validasinya; pembacaan source saat ini
tidak menyertifikasi ulang seluruh checkout. “Sudah diperiksa” selalu merujuk
subflow yang tercantum, bukan seluruh fitur.

| Fitur                                       | Sudah diperiksa                                                                                                                                                                                                        | Bagian yang masih terbuka                                                                                                 |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Settings                                    | General/game/Trust, theme, browser homepage, AI draft/key, updater admission/progress; tujuh perbaikan terakhir dan cleanup tema tervalidasi                                                                           | Updater install/restart nyata, credential/API, telemetry nyata, positive database reset                                   |
| Catalog / MasterDB                          | Cache generation, game type, destination mapping, rollback/retry, pending-preview cleanup                                                                                                                              | Live GitHub/picker, abrupt crash                                                                                          |
| Mods Manager                                | Toggle/only-enable, metadata, bulk safety, Move-to-Object, selection/paging, empty search, junction refusal, collection apply/restart                                                                                  | Seluruh kombinasi CRUD/conflict, symbolic selection lintas halaman, daftar besar                                          |
| Preview / gallery / thumbnail               | Favorite/pin, fullscreen, hide/restore, clear/restart, texture toggles; pass ini membaca clipboard/picker dan jalur penyimpanan gambar                                                                                 | Native clipboard/picker, race folder rename/replacement, gallery feedback dengan watcher gagal, positive recycle          |
| INI editor / KeyViewer                      | Draft/hash ownership, stale acknowledgement, failed-save retention, resource matching, immutable artifact repair, native save/discard/cancel dan normal sync                                                           | In-game rendering, actual focused-game reload                                                                             |
| Mod Inbox / import / archive                | Ownership/containment, cancellation/retry, Windows quota, ingestion/archive tests dan native routing/publication                                                                                                       | OS drag gesture, dua volume fisik, interrupted native import/restart, Linux                                               |
| Matching / reconciliation                   | Scoring, game-owned ID, aliases, preview/commit parity, pinned publication, safety replacement; classification host concurrency ditelusuri pass ini                                                                    | Host overlapping-launch regression, crash/restart/Linux lengkap                                                           |
| Collections / Safe Mode / Randomizer        | Safety revalidation, persistent intent, dormant nonmembers, preview diff/apply/restart, initial-query cancel, locked selection                                                                                         | Full-app crash pada deferred completion, native pending-response interleaving                                             |
| Storage Optimizer / duplicates              | Physical identity, root-bound report, strict hash/read errors, manifest/whitelist, cancel-vs-commit, event ownership, targeted native                                                                                  | Semua nonexact/large/partial-error dan onboarding ID/safety matrix                                                        |
| Discover / Browser / Downloads              | Address/scheme, title/filename/reservation, transfer ordering, stale tabs, owned publication/rename, protected history                                                                                                 | Full navigation/adblock/live Internet, semua transfer action native                                                       |
| File-lock / mutation recovery               | Retry/unlock/cancel, owned process control, forged capability refusal, source/destination race, durable crash receipts                                                                                                 | Semua structural entry point, kernel/directory holders, full-app Safe Mode crash                                          |
| Watcher / suppression                       | Owner release, echo expiry, external convergence, switch/restart reuse, runtime-parent tests; latest N/O native runs ikut memeriksa convergence                                                                        | Semua failed-coverage recovery; pass ini hanya menelusuri image catch-up dari source                                      |
| Global hotkeys / overlay                    | Ordinary native replay/preset/disabled/stopped-game; physical Borderless Safe Mode; original task menyelesaikan dua Fullscreen cases dan installed-file hash verification                                              | Installed app belum diluncurkan setelah update; actual 3DMigoto rendering, held modifiers/focus loss belum tersertifikasi |
| Onboarding / startup / indexing / Dashboard | Index/restart, recovery, readonly GIMI, census/scan reuse; latest private profiling/bundle split dengan empat launch native; pass ini membaca 11 source Dashboard, chart/actions/search/read model/identity suggestion | Chart/action native interaction, deferred review regressions, cold SLA, WPR attribution/real-library runtime publication  |
| Active Conflict Review                      | Ignore/restore/filter, stale/overlapping decisions; busy cancel unit; idle Escape/reopen native                                                                                                                        | Busy native cancel, disk ignore persistence, settlement, daftar besar                                                     |
| Games / launcher / Mod Health / trash       | Parsing, executable/instance identity, provenance/trash ownership; pass ini menelusuri Mod Viewer receipt/health review                                                                                                | Real launch/replacement interleaving, positive OS recycle, Linux                                                          |

## Tambahan pass ini

- Dashboard: 11 dari 11 source pada triage dibaca; tests yang dibaca adalah konteks
  coverage, bukan tes baru yang dijalankan.
- Integrasi preview: alur aktif clipboard/picker, game/folder ownership, native
  image writes, watcher refresh, viewer receipt dan completion ditelusuri.
- Lima bug terkonfirmasi (empat dari source, satu mounted-hook repro) dan tiga kandidat pending tercatat dalam
  [laporan](report.md). Dugaan feedback hilang dan thumbnail prefix mismatch gugur.
- Tidak ada perubahan source/config/test produksi, app build, Cargo, driver/native,
  clipboard nyata, executable launch, shared dist, dependency, atau data library.
  Dua cases isolated frontend dijalankan setelah release slot: regression dismiss
  gagal sesuai defect yang dicari; kontrol publikasi null lulus. Repro/config hanya
  ditambahkan di ignored .audit-output, tanpa menyentuh tests produksi.

## Evidence

- Coverage awal beserta history: [peta sebelumnya](../audit-next-20261011/coverage.md).
- Settings/Conflict: [validasi](../audit-next-fixes-20261011/validation.md) dan
  [history](../../history/202610110014-fix-settings-and-conflict-contracts.md):
  89 frontend, 26 Rust dan enam native cases terarah pada snapshot masing-masing.
- Follow-up sebelumnya: [validasi](../audit-followup-fixes-20261011/validation.md).
- Watcher/hotkeys: [history native](../../history/202610110021-validate-physical-global-hotkeys.md).
  [Completion/install](../mods-manager-hardening/hotkey-validation-install-20261011.md)
  mencatat dua Fullscreen cases PASS dan installed-file verification. Aplikasi
  terpasang belum diluncurkan setelah update; ini bukan real-library runtime smoke.
- Startup: [profiling](../../history/202610110013-correct-native-release-ui-profiling.md)
  dan [bundle/WPR](../../history/202610110022-audit-bootstrap-bundle-wpr.md).
  Pengurangan JS pada private comparison bukan klaim startup speed gain. Handoff
  startup terbaru telah menerapkan automatic split di Vite dan menyelesaikan empat
  launch private release; source fixture/config cleanup diverifikasi oleh task itu.
- Audit sekarang: JSON Hunter/Skeptic/Referee/report/coverage dan source-integrity
  berada di ignored `.bug-hunter/audit-dashboard-preview-20261011/`.
