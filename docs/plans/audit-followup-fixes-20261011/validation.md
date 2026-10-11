# Audit follow-up validation

## Implemented scope

All nine authorized findings and the verified cleanup candidates are implemented.
The randomizer dialog also permits vertical scrolling after native evidence showed
its confirmation footer clipped at 1280 × 800.

- Native drop routing uses the outer grid bounds, including an empty grid.
- Registered targets resolve relative or absolute object paths; nested import
  subpaths remain relative to the selected object.
- Listener disposal owns successful and late asynchronous registrations.
- Randomizer locks apply to selection, preview, payloads and displayed counts.
  Preview invalidation settles its request without admitting older completions.
- Collection publication cancels old reads before the shared refresh.
- Ordinary staging failures become retryable Failed/Partial state. Cancellation
  retains the existing wait-before-cleanup lease and interrupts folder copies
  between bounded reads. Permissions and modified time are preserved.
- Windows disk quotas match physical mount/path representations, including
  missing staging descendants.

## Cleanup and retained coverage

The unused direct archive extraction pipeline and its exclusive result/options,
backup/move/loose-file/name helpers are removed. Active staging retains password,
nested extraction, source retention, root discovery, progress, cancellation,
quotas, path/type validation and owned cleanup. Sixteen staging/analyze tests and
twelve security tests passed after migration and before production deletion.
Publication naming, object ownership, collision and ancestor guards remain covered
by active ingestion placement and mutation tests rather than obsolete overwrite
options.

Four unreferenced frontend barrels, the unused parent-dialog helper, the test-only
unsupported-path helper, six unused wrapper exports and stale mocks are removed.
Underlying active hooks, IPC registrations, migrations, recovery and identity
guards are retained. No dependencies, branch, worktree or commits were added.

## Verified checks

- Before fixes: deferred frontend regressions failed; ingestion had three expected
  failures; Windows disk matching had two expected failures.
- Frontend: ten-file suite passed 79 tests. The additional game-relative import
  regression subsequently failed then passed with the target resolution fix.
- Focused randomizer after the scroll fix: 18 tests passed; scoped ESLint with
  zero warnings passed. App and E2E typechecks passed before the final CSS-only
  change; subsequent fresh harness builds perform app typechecking again.
- Rust: freshly compiled default debug lib filters passed 50 ingestion tests and
  49 archive tests. Two preexisting manual ingestion benchmarks remain ignored.
  The Windows tests used the actual temporary directory and sysinfo volume list.
- Architecture lint passed. Shared strict Clippy passed all targets with
  `readonly-audit`, `-j1` and `-D warnings` after removing one orphaned tail return.
- Read-only review reported no actionable findings in the nine fixes/cleanup.

## Native execution evidence

All runs use the existing fresh private WDIO/WebView2 fixture harness, identifier
`com.reynalivan.emmm.e2e`; no browser demo or real GIMI binding.

The first build ran thirteen cases: nine passed and four failed. Its 1,422 input
hashes had zero drift; the validated private binary SHA-256 was
`4aab54336a46698eef80894cc7acfd5a5882c84f7192dc5e5ae9a349ccfc7aed`.

The audit-only rerun passed sidebar routing, nested subfolder publication,
ordinary-error retry and visible collection save. It exposed two remaining native
checks: the clipped randomizer confirmation footer and an invalid test attempt to
replace the immutable Tauri invoke bridge. DOM diagnostics established both
causes; the bridge replacement has been removed entirely.

The native preview test now checks actual review → selection invalidation → fresh
review. Pending-response interleavings remain covered by deferred frontend tests.
The final two randomizer cases passed on a fresh private debug build after the
one-class scroll fix: locked-object apply preserves its on-disk state, and
selection invalidates the reviewed loadout before a fresh native review settles.
Its 1,422 input hashes had zero drift; the private binary SHA-256 was
`0883b896c6613b7d7533fe8011014f500a0e6e55b789e3213e0d18f57f60b7ca`.
Thirteen distinct functional native cases therefore passed across focused runs;
they are not a claim that the entire application suite passed.

Other executed native cases passed: three shared import cases, three normal-sync/
KeyViewer cases, and one startup case containing three real process restarts.
Each finished native run confirmed owned process-tree shutdown and fixture cleanup.

Ignored evidence directory: `.audit-output/audit-followup-20261011/` contains
`native.log`, `native-rerun.log`, `native-diagnostic.log`, `native-final.log`, input
manifests, `native-final-verified.json` and randomizer DOM/button-geometry
diagnostic JSON. The first runs used a locked
desktop: they certify functional WebView2/IPC/disk behavior, not OS drag gestures,
keyboard replay, foreground rendering or performance.
