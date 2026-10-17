# EMMM2 Audit Coverage Reminder

Last updated: October 17, 2026.

This document is the audit continuation point. **"Reviewed" means the named
subflow was checked; it does not mean the entire feature is bug-free.** A
passing suite does not replace auditing every behavior combination. Update the
table, findings, and evidence after each pass; do not revisit closed areas
without a new reason.

Current user direction: skip difficult/unavailable runtime tests and continue
feasible source audit. Such cases are **skipped, unverified**, never passed. The
18-row disposition map is linked in the latest continuation below.

## Current Audit Status

The requested reminder audit is complete with explicit runtime skips authorized
by the user. All 18 feature groups have a terminal **source-reviewed / runtime-
skipped** disposition in the [final matrix](docs/plans/audit-remaining-20261017/completion.md).
This does not mean all runtime behavior passed. Seven new bugs and eleven cleanup
candidates are reported below and remain unapplied.

## Historical Feature Summary

The following detailed inventory preserves the earlier review record. Its gap
column is superseded by the final matrix above: feasible source gaps were checked,
and difficult runtime cases were explicitly skipped rather than left in a loop.

| Feature                               | Reviewed                                                                                                                                                                                                                               | Not fully reviewed / not verified                                                                           |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Settings                              | General, Games, Trust, themes, AI draft/key, homepage, updater admission/progress, privacy/logs/maintenance, GameForm lifetime                                                                                                         | Real updater install/restart; real vault/API; positive reset                                                |
| Catalog / MasterDB                    | Mapping, generation guard, import/review/rollback/retry, pending cleanup, pack validation, SQLite helpers, checksum runtime                                                                                                            | Live GitHub/picker; abrupt crash; all cache invalidation paths during replacement                           |
| Mods Manager                          | Toggle/only-enable, metadata/safety, move, selection/paging, junction guard; symbolic selection/conflict wiring, backend paging and SEL-1 completion ownership                                                                         | Full native delayed CRUD/conflict matrix; >10k selection and large-list benchmarks                          |
| Preview / gallery / thumbnail         | Favorite/pin, fullscreen, hide/restore, clear/restart, texture, ownership/cache invalidation                                                                                                                                           | Native clipboard/picker race; positive recycle; thumbnail age/cleanup                                       |
| INI editor / KeyViewer                | Draft/hash ownership, stale completion, save/discard/cancel, provenance, artifact repair, normal sync; INI/text/status generation, immutable publication and reload/focus helpers                                                      | Real in-game/3DMigoto rendering, reload acknowledgment and retained generation handles                      |
| Inbox / import / archive              | Source/destination ownership, Processed containment, cancel/retry/quota, extraction/publication, manifest/preview; native watcher remount and pending IPC; coordinator/staging and full import_commit/ready_to_move source             | Mixed recovery-status and interruption matrices; OS drag; two physical volumes; native crash/restart; Linux |
| Matching / reconciliation             | Scoring/aliases, game-owned IDs, preview/commit parity, pinned publication, stale classification host                                                                                                                                  | Full crash/restart; Linux                                                                                   |
| Collections / Safe Mode / Randomizer  | Safety, intent, preview/apply/restart, loadout ownership, recovery IPC, reference transitions, query/runtime helpers, metadata projection; full apply/rollback/path transition/runtime helpers and durable receipt subprocess recovery | Production-app crash/deferred config matrix; unproven concurrent full-query publication                     |
| Storage Optimizer / duplicates        | Physical identity, root-bound report, errors, whitelist/manifest, cancel/commit/event ownership                                                                                                                                        | Non-exact/large scan; partial-error matrix                                                                  |
| Discover / Browser / Downloads        | Scheme/address, filenames/titles, reservation, transfer ordering, tab ownership, protected history, adblock source                                                                                                                     | Live navigation/adblock/Internet; full native transfer matrix                                               |
| File-lock / mutation recovery         | Retry/unlock/cancel, owned process control, capability/path guards, durable receipts; complete mod create/rename/delete boundaries, Windows holder/context tests                                                                       | Remaining structural bulk/move/conflict/object implementations; directory/kernel holders                    |
| Watcher / suppression                 | Owner release, echo/convergence, switch/restart, runtime-parent/image catch-up; native Inbox remount                                                                                                                                   | All coverage-loss/recovery interleavings                                                                    |
| Global hotkeys / overlay              | Native/physical keys, fullscreen/borderless, passive notices, foreground/expiry, installed hashes                                                                                                                                      | Real loader rendering; hardware click-through; multi-monitor/DPI                                            |
| Onboarding / startup / Dashboard      | Index/restart/recovery, readonly GIMI, census/scan reuse, DTO/actions/review, synthetic onboarding; native Home executable setup                                                                                                       | Cold-start SLA; chart/action matrix; runtime publication on a real library                                  |
| Active Conflict Review                | Ignore/restore/filter, stale/overlapping decisions, unit pending cancel, native idle Escape/reopen; native Ignore/Restore persistence, document reload and session restart; Top Bar and compact entry regressions                      | Native cancel while pending; large lists                                                                    |
| Games / launcher / Mod Health / trash | Arguments/executable identity, provenance, health paths, viewer lease/receipt, trash ownership; native optional exe/Play routing                                                                                                       | Real game launch; full health matrix; positive recycle; Linux                                               |
| App shell / Top Bar                   | Navigation, compact portal, resize/styles, editor lifetime, native runtime status                                                                                                                                                      | Complete keyboard/focus matrix                                                                              |
| Diagnostics / telemetry               | Payload allowlist, disabled SDK producers, producer/writer/revision fencing, consent, explicit crash reporting                                                                                                                         | Real collector/export; native consent interleavings                                                         |

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

## Source Queue Completed in Latest Pass

1. Catalog replacement/alias cache producers reviewed; CAT-1 recorded.
2. Ingestion mixed recovery-status source matrix reviewed; no new confirmed bug.
3. Storage non-exact/partial-error branches reviewed; STO-1 recorded.
4. Structural object helper source read completed; STR-1/STR-2 recorded, alongside
   post-apply PA-1/PA-2 and thumbnail MHC-1. Findings await remediation.
5. Difficult environment/native cross-products are explicitly skipped per user:
   game/loader, kernel holders, recycle, hardware, two volumes, full app crash,
   Linux and real service integrations. Do not repeatedly rebuild/retry them as
   gates while feasible source work remains.

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

## Collections Lifecycle / Query Continuation — October 17

CCR-1, CCR-2 and the verified cleanup below are implemented. All 18 feature groups
above remain partially reviewed, not certified complete. Actual checks:
[Lifecycle fix validation](docs/plans/audit-collections-crud-20261017/validation.md).

- [x] **CCR-1 / P2:** deleting a named collection referenced by a PENDING/RUNNING
      recovery task could remove its rollback snapshot. Delete now invokes the
      shared rollback-reference guard inside its transaction, before writes.
      Actual application/database regressions preserve snapshot/members/pointers
      for open tasks and permit COMPLETED/FAILED task deletion.
- [x] **CCR-2 / P3:** rename now returns the owning game's actual active status.
      Regressions cover active, inactive and absent-runtime responses.
- [x] Remove two unreachable empty-vector branches in `preview_tree.rs` and their
      unnecessary argument/local plumbing; retain real stored metadata fallbacks.
- [x] Remove the unnecessary owned-row clone in the Collection list loop.

Fix regressions: behavioral RED (two failures, one passing control), followed by
Collections GREEN (119 passed, one ignored). Independent diff review found no
actionable regression. Full Rust: 1,908 passed, 40 ignored, zero failures;
all-target Clippy with `-D warnings`, Rust formatting and scoped whitespace checks
passed. Native UI remains outside this fix validation.

New full reads: 16 production files covering lifecycle, current/live state, runtime,
preview/path helpers, direct SQLite and capture/runtime IPC boundaries. Six
application files remain queued for full reads in this pass. All reviewed backend
hashes and three protected main application data hashes stayed unchanged.

Native exploration is incomplete: initial Collections frame captured, then driver
connection loss before the first interaction evidence completed; launch retries
hit another owner's port 4444. No native defect or passing scenario claimed.
The failed run fixture is retained; cleanup was not confirmed.

Details, full file list, security checklist and limits:
[Collections lifecycle report](docs/plans/audit-collections-crud-20261017/report.md).

## Collections Apply / Safe Mode Continuation — October 17

The audit was scan-only; its confirmed fix and verified cleanup have now been
implemented at the user's request. Validation is tracked in
[Collection dismissal fix checks](docs/plans/audit-collections-apply-20261017/validation.md).
The six application files queued above
are now fully read: `apply.rs`, `projection.rs`, `references.rs`, `safe_mode.rs`,
`safe_mode_references.rs` and `safe_target.rs`. Two supporting files were also read
fully: `apply/steps/resolve_target.rs` and SQLite `safe_mode.rs`. No new backend bug
was confirmed in these eight files. This does not complete Collections coverage.

- [x] **Native ISSUE-001 / P3:** Escape left idle Collection Apply and Delete
      dialogs open. Each reproduced twice; Cancel closes them. The cause was
      CSS/attribute-only opening. Both now use native modal sync and
      guard pending operations. Component regressions cover apply/replace/delete
      pending guards; actual native pending cancellation remains untested.
- [x] Remove the unused `apply_collection_durable_for_safe_mode` wrapper and its
      now-redundant helper argument/false plumbing. Keep the context scope field:
      active Safe Mode still sets it to true.
- [x] Gate test-only `apply_collection` and `restore_collection_with_baseline`
      wrappers with `#[cfg(test)]`; preserve active durable entry points.

Fresh isolated native release: 39 queued commands plus four direct WebDriver
interactions; 40 screenshot/state pairs. Save, duplicate-name rejection, rename,
reload persistence, filters, preview/apply and delete were exercised. Three
exploration/control errors are recorded separately from application findings.
Collector exit 0 confirms evidence recording, not a clean functional verdict.

The owned process tree stopped and this session's fixture was removed; three main
data hashes and 1,680 source-input hashes stayed unchanged. Fifty console warnings
and the existing Edge 155 compatibility warning are documented. The older failed
CRUD audit fixture remains separately retained. No Rust tests or Clippy were
rerun for the original scan-only audit.

Follow-up fix validation: 43 frontend Collections tests, 119 Rust Collections tests
(one ignored), both typechecks, scoped ESLint, all-target Clippy with denied
warnings and formatting passed. Two fresh native release scenarios passed:
Apply/Delete Escape and reopening twice each, with Delete preserving the saved
Collection. Eight screenshot/state pairs confirm modal dismissal. The owned
fixture was removed and three main data hashes stayed unchanged. The existing
Edge 155 driver warning remains recorded; actual native pending cancellation is
still outside this validation.

Details, cleanup caller evidence, security checklist and remaining gaps:
[Collections Apply / Safe Mode report](docs/plans/audit-collections-apply-20261017/report.md).

## Four Open Audit Areas — October 17

Scan-only follow-up across the four requested gaps: 48 selected production files
fully read (19 selection/CRUD, 11 Collections, nine KeyViewer, nine file-lock).
Exact full/partial paths, hashes, checklists and caller evidence are in the
[open matrix audit report](docs/plans/audit-open-matrix-20261017/report.md).
At that scan's conclusion twelve supporting files were queued/partial. The latest
continuation below closes that selected queue; wider coverage remains partial.

- [x] **SEL-1 / P2:** completion of an old bulk delete or its snapshot-expired
      error can clear a newer selection/preview. Inner inactive-game return does
      not guard the outer success callback; same-game newer selection also loses
      ownership. Fixed with semantic selection ownership and a commit epoch.
      Old snapshot errors refresh without resetting newer symbolic selection.
      Regression coverage includes game/query changes, ABA and deferred refetch.
- [x] Verified cleanup implemented: unused `useBulkDelete` and returned symbolic
      helpers; write-only Collection context rollback copies and empty warning
      parameter; KeyViewer writer/default generators/atomic wrapper; names-only
      lock helper; test-only old runtime batch execution. Preserve active backend
      delete, persistent rollback intent, loader legacy IDs and identity guards.

Implementation checks: 1,584 frontend tests passed (one skipped), 1,908 Rust tests
passed (40 ignored), app/E2E typechecks, scoped lint, Rust formatting and all-target
Clippy with denied warnings passed. Full compilation caught an incorrect cleanup
classification: synchronous `RuntimeRenamePlan::rollback` has a Collections test
caller and stays behind `cfg(test)`. No test coverage was dropped for that method.
The isolated native delayed-delete regression passed: newer checkbox/preview
survived completion with two owned locked mods; fixture/process cleanup and three
protected data hashes were verified. The full native suite was not consistently
green: an existing terminate/Retry dialog stayed open on one repeat and blocked
the next case. This remains a runtime repeatability gap. Final evidence is in
[implementation validation](docs/plans/audit-open-matrix-20261017/validation.md).

Actual Windows checks: seven platform locking tests, ten context tests, one
folder-create subprocess crash matrix and one Collection durable receipt
abrupt-exit/reopen test passed. These 19 tests use owned fixtures; they do not
certify kernel/directory attribution, real game/loader rendering, large-list
performance or production-app UI crash behavior. No production source was changed
in this audit. Source hash verification and adversarial review are recorded.

## Remaining Source Continuation — October 17

All 18 feature rows now have a concrete evidence/remaining-target/skip map.
Thirty-four primary production files were fully read with stable source
hashes: structural bulk/move/conflict/object entrypoints, workspace commands,
App startup, watcher recovery/registration/lifecycle, post-apply, hotkey manager,
recovery, thumbnail maintenance/cache, health and Dashboard branches. The previous
12 supporting files are closed (ten production, two fully read test contexts).
The final pass also closes Catalog, Storage and mixed ingestion source matrices.

Confirmed findings, **fixed locally with regression coverage**:

- [x] **STR-1 / P2:** object-create compensation recursively deletes replacement
      or newly added user contents after an awaited failure. Preserve ownership
      proofs and unfamiliar/nonempty contents; identity checks alone are insufficient.
- [x] **PA-1 / P2:** disable → enable → disable overlay fails on the retained
      fixed backup and leaves the active entrypoint present.
- [x] **PA-2 / P2:** failed duplicate-overlay restore still clears its durable
      journal, losing automatic retry intent. Backup bytes remain preserved.
- [x] **MHC-1 / P3:** nested fallback preview deletion can leave the root L1
      thumbnail stale until its 60-second TTL during scoped watcher reconciliation.
- [x] **STO-1 / P2:** overlapping duplicate groups can quarantine selected keepers,
      including every folder in a three-group Keep cycle. Reject conflicts before
      planning; enforce the rule in the backend.
- [x] **STR-2 / P2:** a missing-source object delete fallback can adopt and recycle
      a replacement that appears during its subsequent await.
- [x] **CAT-1 / P2:** removed objects' unique aliases remain in the warm matcher
      cache until explicit invalidation or restart.

Eleven cleanup candidates have caller/control-flow proof: test-only conflict chain,
test-only organizer wrapper, unused update-object state parameter, impossible
steady-state watcher branch, duplicate hotkey validation, unused constructor
argument, dead overlay context API/test-only helpers, unused internal thumbnail
argument, identical Dashboard class ternary, duplicate test-gating attribute and
test-only scanner convenience wrapper. All eleven groups are now applied. Test-only
conveniences retain their coverage behind `cfg(test)`; active async planners,
retry/recovery, command validation and identity guards remain intact.

Fix mitigations: owned empty-directory compensation with durable root/parent/child
proofs; retained lease for metadata-only missing-source delete; no-replace overlay
backups and fallible journal-preserving restore; post-commit matcher revision;
batch keeper/target rejection; bounded completed-thumbnail dependency invalidation.
Independent review caught an introduced junction-root mismatch before finalization.
The correction keeps physical staging/thumbnail writes consistent and validates the
original configured alias; a regression covers valid junctions and retargeting.

[Fix validation and limits](docs/plans/fix-remaining-20261017/validation.md) and
[cleanup disposition](docs/plans/fix-remaining-20261017/cleanup.md).

Passed this read-only pass: 20 hotkey tests, three recovery tests, 16 cache tests,
25 Dashboard tests and one manual 100k candidate benchmark. The latter measures
sort/memory, not rendered UI or 100k directories. Object and overlay probes compile
verbatim helpers with explicit shims; no full Tauri/DB failure injection is claimed.
Schemas and source integrity checks passed. Native/game/hardware/service checks
that are difficult or unavailable are skipped, not passed, following user direction.
Additionally, 59 duplicate tests (one ignored), seven Catalog cache tests, nine
staging tests (one ignored) and six resolution builder tests passed. Actual frontend
helper execution reproduced the STO-1 contradictory keeper requests without
mutating files. All seven findings completed challenge and Referee review.

[Current findings, coverage, skip disposition and evidence](docs/plans/audit-remaining-20261017/report.md).

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
