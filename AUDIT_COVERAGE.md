# EMMM2 Audit Coverage Reminder

Last updated: October 17, 2026.

This document is the audit continuation point. **"Reviewed" means the named
subflow was checked; it does not mean the entire feature is bug-free.** A
passing suite does not replace auditing every behavior combination. Update the
table, findings, and evidence after each pass; do not revisit closed areas
without a new reason.

## Reviewed and Still Open

| Feature                               | Reviewed                                                                                                                                                                                                                   | Not fully reviewed / not verified                                                                           |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Settings                              | General, Games, Trust, themes, AI draft/key, homepage, updater admission/progress, privacy/logs/maintenance, GameForm lifetime                                                                                             | Real updater install/restart; real vault/API; positive reset                                                |
| Catalog / MasterDB                    | Mapping, generation guard, import/review/rollback/retry, pending cleanup, pack validation, SQLite helpers, checksum runtime                                                                                                | Live GitHub/picker; abrupt crash; all cache invalidation paths during replacement                           |
| Mods Manager                          | Toggle/only-enable, metadata/safety, move, selection/paging, junction guard                                                                                                                                                | CRUD/conflict combinations; complete symbolic selection; large lists                                        |
| Preview / gallery / thumbnail         | Favorite/pin, fullscreen, hide/restore, clear/restart, texture, ownership/cache invalidation                                                                                                                               | Native clipboard/picker race; positive recycle; thumbnail age/cleanup                                       |
| INI editor / KeyViewer                | Draft/hash ownership, stale completion, save/discard/cancel, provenance, artifact repair, normal sync                                                                                                                      | Real in-game and 3DMigoto loader rendering/reload                                                           |
| Inbox / import / archive              | Source/destination ownership, Processed containment, cancel/retry/quota, extraction/publication, manifest/preview; native watcher remount and pending IPC; coordinator/staging and full import_commit/ready_to_move source | Mixed recovery-status and interruption matrices; OS drag; two physical volumes; native crash/restart; Linux |
| Matching / reconciliation             | Scoring/aliases, game-owned IDs, preview/commit parity, pinned publication, stale classification host                                                                                                                      | Full crash/restart; Linux                                                                                   |
| Collections / Safe Mode / Randomizer  | Safety, intent, preview/apply/restart, loadout ownership, recovery IPC, reference transitions, query/runtime helpers, metadata projection                                                                                  | Remaining query/transition helpers; deferred completion during app crash; see COL-1 below                   |
| Storage Optimizer / duplicates        | Physical identity, root-bound report, errors, whitelist/manifest, cancel/commit/event ownership                                                                                                                            | Non-exact/large scan; partial-error matrix                                                                  |
| Discover / Browser / Downloads        | Scheme/address, filenames/titles, reservation, transfer ordering, tab ownership, protected history, adblock source                                                                                                         | Live navigation/adblock/Internet; full native transfer matrix                                               |
| File-lock / mutation recovery         | Retry/unlock/cancel, owned process control, capability/path guards, durable receipts                                                                                                                                       | All structural entry points; kernel/directory holders                                                       |
| Watcher / suppression                 | Owner release, echo/convergence, switch/restart, runtime-parent/image catch-up; native Inbox remount                                                                                                                       | All coverage-loss/recovery interleavings                                                                    |
| Global hotkeys / overlay              | Native/physical keys, fullscreen/borderless, passive notices, foreground/expiry, installed hashes                                                                                                                          | Real loader rendering; hardware click-through; multi-monitor/DPI                                            |
| Onboarding / startup / Dashboard      | Index/restart/recovery, readonly GIMI, census/scan reuse, DTO/actions/review, synthetic onboarding; native Home executable setup                                                                                           | Cold-start SLA; chart/action matrix; runtime publication on a real library                                  |
| Active Conflict Review                | Ignore/restore/filter, stale/overlapping decisions, unit pending cancel, native idle Escape/reopen; native Ignore/Restore persistence, document reload and session restart; Top Bar and compact entry regressions          | Native cancel while pending; large lists                                                                    |
| Games / launcher / Mod Health / trash | Arguments/executable identity, provenance, health paths, viewer lease/receipt, trash ownership; native optional exe/Play routing                                                                                           | Real game launch; full health matrix; positive recycle; Linux                                               |
| App shell / Top Bar                   | Navigation, compact portal, resize/styles, editor lifetime, native runtime status                                                                                                                                          | Complete keyboard/focus matrix                                                                              |
| Diagnostics / telemetry               | Payload allowlist, disabled SDK producers, producer/writer/revision fencing, consent, explicit crash reporting                                                                                                             | Real collector/export; native consent interleavings                                                         |

## Collections Findings and Cleanup Pass

- [x] **COL-1 / P2 — code fix and unit regression:** an absolute `preview_path` can
      remain after a mod/object rename, re-enter as an Apply target, and produce
      `MissingMods`. The fix covers ordinary renames and staged identity
      transitions, including swaps, without breaking ancestor previews for
      mod/variant packs.
- [x] Remove `compute_signature`, which has no callers in the Collection projection.
- [x] Simplify `get_projection_context`: both callers always discard the
      `is_safe` field; retain the LEFT JOIN and the error when a collection is not found.
- [x] Native COL-1 validation: live save → rename mod/object → preview → disable
      → Apply with `ignoreMissing: false`; confirm the folder becomes active again.
- **Rejected as a confirmed bug:** COL-2, interior-segment replacement.
  The SQL probe shows this behavior, but there is no evidence that the triggering
  data shape is produced by an active producer. Avoid global replacement in the
  COL-1 fix.
- **Unproven:** the hypothesis that the Catalog runtime only uses size/timestamp.
  The inspected path already verifies bytes and the manifest checksum; do not
  label it a bug without additional evidence.

## Collections Fix Validation

Collections: 116 tests passed, with one manual benchmark ignored. The final
combined Rust suite passed 1,891 tests with 40 ignored. All-target Clippy with
`-D warnings`, Rust formatting, native-test typecheck and scoped native
spec/config lint passed. Both native Collections scenarios passed on the fresh
isolated Windows release build. The owned process tree stopped, one fixture was
removed, 1,652 source input hashes stayed unchanged, and all three protected main
application data files retained their hashes. Detailed evidence and limits are
linked below.

Parallel work also validated executable discovery: five game profiles through
native IPC, Settings cancel/save/empty-save behavior, onboarding through the real
Windows folder picker, and removed-executable rejection. Real installed-game
launch and the full provider/environment matrix remain open.

## Next Audit Priorities

1. Remaining Collections query/transition helpers and deferred completion/recovery.
2. Watcher behavior when coverage is lost and recovery fails or is retried.
3. Native Conflict cancellation while pending and large-list behavior; normal
   ignore/restore reload and restart validation is tracked below.
4. Ingestion mixed recovery-status and interrupted-restart behavior; ING-1/ING-2
   are fixed. Full import commit/compensation source is now read; real crash and
   multi-volume behavior remain open.
5. Environment matrix: real game/loader, positive recycle, two physical volumes,
   crash/restart, Linux, and live browser/downloads.

## Ingestion Continuation — October 17

Production fixes and verified cleanup applied; actual regression tests pass.

- [x] **ING-1 / P2:** Utility Skip now clears destinations through the normal
      transition. Utility import still requires an explicit existing destination.
- [x] **ING-2 / P2:** single/multiple roots with external or dynamic references
      retain the original package, including sibling Shared assets. Independent
      roots still split; out-of-package roots are rejected.
- [x] Remove two application refresh wrappers without callers; keep their
      registered IPC commands and implementations with the shared target index.
- [x] Restrict the default-index decision wrapper and default-options staging
      wrapper to tests.
- [x] Remove the two unused `execute_moves` parameters and update both test callers.
- [x] Remove the obsolete `common_parent` helper after package-boundary grouping.

New coverage: complete coordinator/staging production source; targeted commit
admission, planning, recovery, payload validation, move execution, and finalization.
The table's earlier "entire coordinator/staging flow through commit" gap is now
reduced to unreviewed commit/compensation sections and behavioral/environment
matrices. Full crash/restart remains unverified. Fresh isolated Windows release
verification passed all three native scenarios: single/multiple-root bundles
through actual commit and Utility Skip through the existing wizard button.

Probe compilation with `-D warnings` and execution passed; success reproduces the
two faulty behaviors. The possible sibling-staging retry cleanup issue remains
unproven and is excluded from confirmed findings.

Actual tests: import_batch 62 passed / 2 ignored; full Rust 1,901 passed / 40
ignored. All-target Clippy with `-D warnings`, Rust formatting, app/E2E typechecks,
and scoped native spec/config ESLint passed.

Native: 3 passed in 7.7 seconds, actual exit 0. Owned process shutdown and one
fixture cleanup completed; all 1,653 source inputs and all three protected main
application data file hashes stayed unchanged. The existing Edge WebDriver
compatibility warning for Edge 155 remains documented in validation.

Details: [Ingestion audit report](docs/plans/audit-ingestion-20261017/report.md)
and [fix validation](docs/plans/audit-ingestion-20261017/validation.md).

## Recovery Source and Native Exploration — October 17

Both confirmed findings and verified cleanup have been implemented. Validation:
[Recovery fix checks](docs/plans/audit-import-recovery-20261017/validation.md).

- [x] **Native ISSUE-001 / P2:** after the last active group is ignored and the
      document reloaded, the Top Bar Conflict Review entry disappears. Repeated
      with a second reload. The Ignored tab offered Restore group before closing;
      the observed App Menu has no Conflict Review destination. The entry now
      follows raw active groups, with the badge counting only unresolved groups.
      Remount and compact-menu unit regressions pass.
- [x] **REC-1 / P3:** completion counts created canonical folders per mod plan;
      two different costumes sharing one new object directory report two folders.
      Actual move/report regression now verifies shared new folders count once,
      preexisting folders count zero, and distinct new folders count separately.
      Full native dual-costume canonical import was not run.
- [x] Simplify the permanently empty new-commit collision set and its dead
      branches/helper parameters. Preserve active `recovered_collisions` and
      historical item collision recovery.
- [x] Remove `report_for`'s always-false `partial` parameter and the discarded
      reconcile result binding while keeping commit/event/rollback side effects.
      Remove the now-unused lease skip wrapper; retain underlying historical
      journal collision APIs and their tests.

Fully read this pass: `import_commit.rs` (2,421 lines) and `ready_to_move.rs`
(104 lines), with unchanged assigned source hashes. The relevant full working-tree
diff was read; `master...HEAD` is unavailable because this repo has no master ref.
This is not a whole-branch or repository audit.

Native: fresh isolated Windows release, 20 interactive commands, 21 screenshot/JSON
pairs, no command errors. Collector exit 0 confirms evidence collection, not that
all observed behavior was correct. Process tree stopped and one fixture removed;
three protected main app data hashes stayed unchanged. Console warnings and the
Edge 155 driver warning are recorded without inventing extra bugs.

REC-2 mixed ready/recovery and REC-3 partial source deletion hypotheses were
dismissed using active caller partitioning and identity-bound tombstone retirement.
No recovery guard, journal, migration, platform fallback or dependency is approved
for removal. Single-pass source verdicts: one confirmed, two dismissed.

Fix verification: 38 frontend tests, 20 import_commit tests, full Rust 1,902
passed / 40 ignored, all-target Clippy, formatting and scoped static checks passed.
Four native scenarios passed, including Conflict Ignore/Restore through document
reload and actual session restart. Two fixtures were cleaned; three protected main
data hashes stayed unchanged. The only source-snapshot difference was an unrelated
new landing capture file; Edge 155 driver compatibility warning remains recorded.

Details: [Recovery and native audit](docs/plans/audit-import-recovery-20261017/report.md)
and [fix validation](docs/plans/audit-import-recovery-20261017/validation.md).

## Cleanup Boundaries

Do not remove persisted-data migrations, recovery journals, filesystem identity
guards, or platform fallbacks that are still in use just because they look old.
Check runtime callers, facade/command registration, dynamic imports, and tests.
Dynamic demos have caused Knip false positives before. No dependency has been
verified safe to remove.

## Evidence and History

- [Previous audit coverage map](docs/plans/audit-gaps-20261012/coverage.md).
- [Previous Inbox/preview and native-fix validation](docs/plans/audit-gaps-fixes-20261012/validation.md).
- [Audit Collections 17 Oktober](docs/plans/audit-collections-20261017/report.md).
- [Collections fix validation](docs/plans/audit-collections-20261017/validation.md).
- [Parallel executable discovery validation](docs/plans/game-executable-detection-20261017/validation.md).

Note: the `docs/` directory is currently ignored by Git through the workspace
configuration. This reminder file is at the root so it remains visible as a
project artifact. Do not restore or stage deletions of docs owned by other work.
