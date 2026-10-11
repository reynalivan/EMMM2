---
name: e2e-automation
description: Test EMMM Windows desktop flows in the native application with WebdriverIO, tauri-driver and WebView2. Use for app UI, startup/restart, IPC, watcher, persistence, import and recovery validation; documentation-only work needs documentation checks.
---

# Native Windows App E2E

Use the real EMMM Windows executable through the existing native harness.
Browser demo/Playwright testing requires an explicit user request; fixture
results cannot establish native correctness. If native E2E is unavailable,
report the actual blocker and untested scenario instead of changing surfaces.

## Harness and scope

- Read `wdio.conf.ts`, the affected `tests/e2e/specs/*.e2e.ts`, and relevant
  helpers in `tests/e2e/support/` before adding tests. Use focused specs rather
  than rerunning the whole suite for every edit.
- The harness builds the current source with `src-tauri/tauri.e2e.conf.json` and
  snapshots it as `emmm-e2e.exe`. It verifies build identity/hash and requires
  app identifier `com.reynalivan.emmm.e2e` before its database reset. Preserve
  these guards; do not bypass hooks or reuse an arbitrary/installed executable.
- Use owned fixtures from `tests/e2e/support/fixtures.ts` and existing typed
  app/IPC helpers. Never seed, reset or clean the user's real game library or DB.
- Preserve terminal hook failures: `abortNativeRun` stops the owning process
  after owned cleanup, because WDIO may log exceptions and continue. Keep the
  expected private identifier environment contract and bootstrap check; a
  successful file hash alone does not prove the embedded app profile.
- Keep production watchers, activation, reconciliation and recovery enabled.
  Do not suppress them globally to make a test pass. Expected mutations still
  use the application's own suppression protocol.

## Run

From the repository root:

```powershell
rtk proxy corepack pnpm exec wdio run wdio.conf.ts --spec tests/e2e/specs/startup-reopen.e2e.ts
```

Select the spec for the task. The existing harness owns the build, driver,
isolated app launch and teardown; do not start a second harness on its ports or
terminate unrelated processes. Native dialog/game-process scenarios require
their actual Windows path, not mocked browser behavior.

## Assertions and cleanup

- Wait for the existing workspace/core readiness for reads or mutations that
  require it. Test optional-widget skeletons while work is pending and ensure
  the rest of the native window stays usable; verify the terminal loaded/error
  state as appropriate. Do not replace observable waits with arbitrary sleeps.
- Assert visible elements. Responsive layouts can contain a hidden mobile copy
  before the visible desktop element; inspect visibility when text is duplicated.
- Pair UI outcomes with relevant IPC/DB/disk effects through existing helpers.
  Use actual `browser.reloadSession()` process restarts for restart coverage,
  then navigate the new WebView and wait for its IPC bridge. WebdriverIO's
  `browser` object controls the native WebView2 window, not a standalone browser.
- Schedule owned fixture deletion with `scheduleMockGameRemoval`; the harness
  cleans after confirmed process-tree shutdown. Retain fixtures when ownership
  or shutdown cannot be verified.
- Report executed specs and failures honestly. Component/unit tests support
  native evidence; a fixture browser pass is not a native app pass. Keep stage
  timings distinct from total startup and never infer p95 from three samples.

See [the E2E workflow](../../workflows/e2e-testing.md) for routing and
[demo boundaries](../../../docs/frontend-demo.md) for explicitly requested
fixture/browser work. No install, commit, push or deployment is implied.

## Authorized read-only library performance audit

Use `wdio.readonly-audit.conf.ts` for debug or
`wdio.readonly-release-audit.conf.ts` for an optimized release audit. Both require
`EMMM_READ_ONLY_AUDIT_ROOT`, an external `EMMM_READ_ONLY_AUDIT_OUTPUT`, and a
completed full `WindowsHandleV2` Before manifest from
`scripts/audit-library-manifest.ps1`. It reads metadata through existing Windows
handles because NTFS enumeration attributes can be stale. Calibrate
on an owned fixture before an explicitly authorized real-library run. Never use
the ordinary fixture/reset profile against real data.

The private audit identifier enables deny-by-default mutation IPC and bypasses
runtime file publication, reload and cleanup; release requires the default-off
`readonly-audit` Cargo feature. The harness seeds/restores only its isolated
configuration and aborts its own run if installed/development EMMM reopens.
Compare full After digests and Verify metadata, check configuration restoration,
and retain evidence on any difference; never repair the protected library.
Report the publication exclusion and distinguish a fresh process from a cold OS
cache. Checksumming warms the cache; uncontrolled cache measurements cannot
certify storage-cold startup.

For release startup measurements observe the initial native page without an
extra navigation/remount. Include harness/polling overhead in reported wall times
and correlate actual process creation when available; do not label those
observations exact first-paint timestamps.

## Native onboarding picker

`onboarding-indexing-native.e2e.ts` exercises the real manual form, Windows folder
dialog, Confirm, nonempty first index, background game and native restart. Set
`EMMM_E2E_NATIVE_PICKER_REQUEST_DIR` to an absolute owned artifact directory for
an interactive operator run. Without that capability the spec explicitly skips;
never count the skip or the seeded `onboarding.e2e.ts` boot case as Finish Setup
coverage.

The spec writes picker request JSON and a two-fixture ownership inventory. The
operator uses the Computer Use skill's `@oai/sky` APIs from `node_repl`, selects
the returned private `emmm-e2e.exe` dialog window, verifies the fixture marker,
chooses only its requested directory, refreshes the UI, then writes the matching
completion JSON. Never substitute IPC seeding, modify a readonly input, mock the
dialog, construct window handles or run PowerShell UI Automation. Retain any
fixture whose ownership or process shutdown cannot be proved after interruption.

The harness overrides the collector URL with whitespace before the private
build; unlike an empty Windows environment value this survives inheritance and
trims to no telemetry transport. Production environment/configuration is unchanged.
