# Peta coverage fitur

Pemetaan 11 Oktober 2026 dari history sejak 7 Oktober dan laporan validasi terkait.
Hasil historis berlaku pada snapshot masing-masing, bukan pengujian ulang seluruh
checkout saat ini. "Sudah diperiksa" berarti subflow berikut punya bukti terarah;
tidak berarti seluruh fitur selesai diaudit.

| Fitur                                  | Bagian yang sudah diperiksa                                                                                                                                                                     | Batas yang masih terbuka                                                                                         |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Settings                               | General/game form/Trust native 44 kasus; theme selection dan revision save; sesi ini membaca 13 dari 20 file source Settings, termasuk AI/browser/updater/maintenance/logs/integrations/privacy | Updater install/restart nyata, positive database reset, credential vault/API dan telemetry nyata                 |
| Catalog / MasterDB                     | Cache generation, game type 0, destination mapping, rollback/retry dan bounded pending-preview cleanup                                                                                          | Live GitHub/native picker dan abrupt process crash                                                               |
| Mods Manager                           | Toggle/only-enable, metadata, bulk safety, Move-to-Object, selection, paging 100/100/5, empty search, junction refusal, apply/restart collection                                                | Seluruh kombinasi CRUD/conflict, symbolic selection lintas halaman dan virtual scroll besar                      |
| Preview / gallery / thumbnail          | Favorite/Pin, fullscreen, hide/restore, clear-gallery/restart, texture toggles dan thumbnail invalidation                                                                                       | Clipboard/native image picker, external editor/Mod Viewer dan positive recycle-bin flow                          |
| INI editor / KeyViewer                 | Draft/hash ownership, stale acknowledgement, failed-save retention, resource matching, immutable artifact repair; native INI save/discard/cancel dan tiga normal-sync/KeyViewer cases           | In-game rendering dan actual focused-game reload                                                                 |
| Mod Inbox / import / archive           | Clarification, object/subpath/source ownership, Processed containment, cancellation/retry, Windows quota; latest 50 ingestion + 49 archive tests serta native routing/publication               | OS drag gesture, dua volume fisik, interrupted native import/restart dan Linux                                   |
| Matching / reconciliation              | Scoring kosong/zero-score, game-owned object ID, aliases, preview/commit parity, pinned publication dan safety replacement                                                                      | Snapshot lama tidak menyertifikasi checkout baru; crash/restart dan Linux belum lengkap                          |
| Collections / Safe Mode / Randomizer   | Safety revalidation, persistent intent, dormant nonmembers, preview diff, apply/restart, initial-query cancellation, locked selection dan reviewed-loadout invalidation                         | Full-app crash pada deferred Safe Mode completion; native pending-response interleaving                          |
| Storage Optimizer / duplicates         | Physical identity, root-bound reports, strict hash/read errors, manifest/whitelist, cancel-vs-commit dan event ownership; targeted native cases                                                 | Seluruh nonexact/large scan, partial-error dan onboarding ID/safety matrix                                       |
| Discover / Browser / Downloads         | Address/scheme handling, titles, filenames, reservations, status/control ordering, stale tabs, owned publication/rename dan protected history                                                   | Full navigation/adblock/live Internet; semua transfer actions belum punya native coverage lengkap                |
| File-lock / mutation recovery          | Repeated retry, unlock/retry, cancel, explicit owned process control, forged capability rejection, source/destination races dan durable crash receipts                                          | Semua structural entry points; kernel/directory holders; full-app Safe Mode crash                                |
| Watcher / suppression                  | Last-owner release, echo expiry, external convergence, switch/restart reuse dan latest runtime-parent regressions; latest 284 workspace tests                                                   | Latest runtime-parent patch masih menunggu native revalidation menurut history terbaru yang dibaca               |
| Global hotkeys / overlay               | OS input Safe Mode/presets, stopped-game/disabled/background cases, overlay release replay                                                                                                      | Overall native green, held physical modifiers/focus loss dan actual Genshin/3DMigoto masih pending dalam history |
| Onboarding / startup / indexing / Home | Synthetic foreground/background index/restart, Home keybindings, recovery handoff; readonly real GIMI integrity/restarts dan census/scan reuse                                                  | Cold startup SLA, runtime publication/actual game dan seluruh chart/action UI                                    |
| Active Conflict Review                 | Ignore/restore/filter, overlapping decisions, stale summaries; sesi ini 27/27 existing frontend tests lulus dan cancel ownership diperiksa                                                      | Native Escape, focus, disk mutation/ignore restart dan besar daftar belum diperiksa di sesi ini                  |
| Games / launcher / Mod Health / trash  | INI quote parsing, executable/instance identity, resource provenance dan trash ownership                                                                                                        | Real game/loader launch, positive OS recycle, Linux dan path-only recycle replacement window                     |

## Sumber evidence

- [Client smoke baseline](../client-smoke-20261007/report.md) dan
  [hasil perbaikan](../client-smoke-20261007/fix-results.md).
- [Latest audit follow-up](../audit-followup-fixes-20261011/validation.md):
  13 native functional cases lulus lintas focused runs, bukan full application suite.
- [Mods lifecycle](../mods-manager-hardening/lifecycle-validation.md) dan
  [file-lock validation](../mods-manager-file-locks/validation.md).
- [Catalog lifecycle](../../history/202610100001-fix-catalog-setup-lifecycle.md),
  [matching/mutation/reconciliation](../../history/202610100012-fix-matching-mutation-reconciliation.md),
  [collections](../../history/202610080004-collection-safe-mode-stability.md).
- [Preview completion](../../history/202610100011-complete-mods-preview-and-folder-recovery.md),
  [KeyViewer](../../history/202610100004-fix-keyviewer-viewer-pipeline.md),
  [INI cleanup](../../history/202610100006-simplify-keyviewer-editor-architecture.md).
- [Optimizer lifecycle](../../history/202610090004-fix-storage-optimizer-lifecycle.md),
  [browser/duplicates](../../history/202610100025-fix-browser-duplicates-regressions.md).
- [Latest watcher status](../../history/202610110011-diagnose-watcher-runtime-parent-coverage.md),
  [overlay replay](../../history/202610110010-fix-overlay-release-replay-race.md),
  [OS hotkey validation](../../history/202610110005-validate-native-global-hotkey-input.md).
- [Onboarding](../../history/202610100015-fix-onboarding-first-index-progress.md),
  [readonly release GIMI](../../history/202610110004-release-gimi-readonly-certification.md),
  [latest scheduling](../../history/202610110009-reuse-folder-scans-and-schedule-census-continuously.md).
- [Active Conflict changes](../../history/202610100024-active-conflict-review.md),
  [stale decisions](../../history/202610100026-stabilize-active-conflict-review.md),
  [Games/launcher/health/trash](../../history/202610100009-fix-automation-games-library.md).

## Kelanjutan yang belum memiliki coverage lengkap

1. Native Active Conflict cancel/focus, ignore persistence dan mutation settlement.
2. Settings updater job admission/progress, draft ownership dan theme contracts;
   temuan sesi ini ada di [report](report.md).
3. External editor/Mod Viewer, clipboard/native picker dan OS recycling.
4. Dashboard chart/action interactions, penuh alur search/sort/filter serta UI besar.
5. Full-app Safe Mode crash, dua physical volumes dan Linux recovery contracts.

Watcher/hotkey native sedang dicatat sebagai scope verifikasi terpisah. Sesi ini
tidak mengambil alih app/driver, tidak menginstal updater, tidak memakai credential
nyata dan tidak menulis library pengguna.
