# File-lock follow-up fixes

2026-10-10. Scope: BUG-FL-01, BUG-FL-02, BUG-FL-03 and DRIFT-FL-01 from the focused follow-up audit. This is not a new repository-wide coverage claim.

## Changes

- Windows recovery pins request data/list access instead of metadata-only access, retaining delete-denying sharing. Root/target rename is blocked while the handles live; an incompatible existing DELETE holder causes safe rejection. Existing post-pin original-identity validation and process eligibility checks remain.
- Dialog diagnostic preflight no longer enters the terminal mutation-error handler. Bulk preflight uses a typed marker only for structured diagnostic I/O failures before mutation. Later Retry can recover; stale context, actual mutation I/O and failed rollback cannot use that marker to reopen unsafe replay.
- Bulk toggle creates one forward retry budget before the chunk loop and passes it through every chunk/re-entry and all three execution branches. Standalone execution still owns its budget; compensation remains separate.
- Initial/protected INI reads and successfully restored INI/metadata publication failures preserve structured lock diagnostics. Failed restore retains its recovery-path I/O error; no earlier committed structural mutation is replayed.

## Verification

- Frontend: 26 tests pass across the actual dialog, bulk recovery helper, shared operations and error decoding. Regression tests were observed failing before the dialog/bulk diagnostic fixes, then passing. Full frontend typecheck, build, scoped ESLint and architecture checks pass.
- Windows: standalone execution of the current locking module's seven native tests passes, including root/target namespace protection, incompatible handle rejection, shared-reader compatibility, bounded file discovery, PID identity rejection and control of an owned fixture process. Only submodule path resolution was adapted for the standalone wrapper; production functions/tests were unchanged.
- An additional extracted-production native probe failed before the fix and passes afterwards: target/root rename protection, rejection of existing DELETE access, restored metadata lock diagnostics, original bytes preserved and failed restore not classified as generic lock replay.
- Focused Rust regression tests were added for initial locked INI reads, restored INI publication, metadata restore classification and shared bulk budget. Bulk's old signature produced the expected failing compile-time contract before being corrected.
- Full Rust library run subsequently executed: **1,710 passed, two failed, 26 ignored**, 99.06 seconds. All eight added Rust regression tests passed in that run. The collection failure passed its isolated rerun; the watcher rerun was blocked by a subsequent private-API compilation error during concurrent changes.
- Scoped rustfmt and whitespace checks pass. No new dependency, migration, elevation, worktree, commit or user-process termination.
- Fresh isolated Windows application build and native E2E: **4/4 pass**, 28.6 seconds of tests, session `2000c0468382b59029f9a9f3b47243e0`. Covers persistent manual Retry until unlock, Cancel preventing replay, explicit owned-process termination followed by separate Retry, and forged-capability IPC rejection. The harness confirmed native shutdown and removed one owned fixture; production app data was not reset.

## Integration limitation

**Coordinated closure, 2026-10-10:** the active task owners corrected the collection/watcher follow-up and ran the combined source: **1,723 Rust tests passed, zero failed, 27 ignored**; strict **all-target Clippy with `-D warnings` passed**. Relevant source hashes stayed unchanged across validation. The paragraphs below retain the earlier failed-run evidence, not the current gate status. See [coordination and provenance](../../../.bug-hunter/coordinated-suite-fixes/coordination.md).

The shared checkout has concurrent watcher refactoring outside this patch. Early builds were blocked by missing/private watcher APIs and production reexports of test-only event-filter helpers. A later full library run failed in `completed_collection_task_does_not_rollback_disk_after_journal_gap` and `canceled_receiver_lifetime_finishes_pending_recovery`. The first passed alone; the second could not be re-certified after another watcher API change. Final strict all-target Clippy reached an unrelated `len_zero` violation in `scanner/tests/watcher_tests.rs:91`. These files were not modified for this fix. Standalone module/component checks do not certify native application behavior; the earlier native E2E result is historical, not rerun evidence for this patch.

The two full-suite runtime failures are not attributed to a proven cause in this session. Their location outside changed files and one successful isolated rerun do not establish that they were pre-existing or harmless.

The frontend, added Rust regressions, owned native module and fresh native application evidence establish the scoped fixes above. The previously pending full Rust/Clippy gates are now green in the coordinated closure. Native UI coverage is the four existing file-lock scenarios, not every CRUD or planning-matrix case.

## Reproduction and records

- Audit: [confirmed findings and evidence](../../../.bug-hunter/file-lock-followup/report.md).
- Standalone Windows tests: `rtk proxy node .bug-hunter/file-lock-fixes/run-locking-tests.cjs` from the repository root; uses existing cached Rust dependencies and only owned fixtures.
- Focused native probe: `rtk proxy node .bug-hunter/file-lock-fixes/native-regressions.cjs`.
- Canonical remediation artifact: [fix report](../../../.bug-hunter/file-lock-fixes/fix-report.json).

The canonical report's numeric baseline/final fields cover frontend regression selections (initial dialog RED and final four-suite GREEN), not a whole-repository baseline comparison. `build_pass: true` records the passing frontend and isolated native builds; it does not claim all global tests/lints pass. The drift item is included as the fourth explicitly authorized remediation, not counted as an additional discovered runtime bug.

Windows share/access semantics were checked against [Microsoft CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew) and Windows Rust API documentation. Native tests, not a metadata-only open assumption, verify the pin invariant.
