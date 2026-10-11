# Validation — Dashboard and preview fixes

Implementation, targeted checks, independent review and isolated native validation
are complete on 11 October 2026.

## Earlier regression evidence

- Dashboard/classification worker captured deferred RED regressions for dismiss,
  context changes, reversed launch results/errors and obsolete rejection. Targeted
  runs reached 29 tests before the last rejection follow-up; the latter run passed
  11 tests in its two files. Counts overlap and are not added.
- Preview worker captured picker-open target and deferred form delete/metadata
  regressions; its latest eight-file run passed 33 tests with one existing skip,
  real TanStack Query, app types and scoped ESLint.
- Original mounted dismissal RED is retained in the audit evidence. The tracked
  mounted case is GREEN; earlier viewer state/mounted runs passed eight tests,
  covering dismiss, relaunch, zero-diff clearing and deferred analysis ownership.
  The first zero-diff fixture used an absolute watcher root and did not dispatch;
  it was corrected to the backend's relative-root contract and asserts the actual
  analysis call. No verified pre-fix baseline replay is claimed.
- Rust guard tests: first contract RED was a missing implementation import;
  first runtime run passed four cases but its reconfiguration fixture incorrectly
  used the generic settings save. It was corrected to a fresh authoritative config,
  preserving source-recovery policy. The earlier guard run passed 16 tests (five ownership
  contracts plus eleven existing containment/ownership tests).
- Actual `cargo test --lib specta_tests -- --nocapture` passed two tests:
  registry permission consistency and binding export after legacy endpoint removal.
- Source/test/scripts caller search contains no exact legacy flat preview endpoint,
  generated camelCase command or deleted inner helper. Visibility-aware coverage
  retains filtering of hidden entries.

## Fresh Windows native release

The performance task owned one combined fixture-release capture using
`.audit-output/final-performance-audit-20261011/wdio.final-fixture-release.conf.ts`.
All 15 cases across five spec files passed (exit 0), including all nine selected
Dashboard/preview cases:

- TC-19-01 saved image listing; TC-41-01 thumbnail cache population.
- Dashboard/Mods/Collections navigation and isolated second-game selection.
- Metadata persistence with Unicode and empty description.
- Gallery/fullscreen navigation and cancel.
- Hide/restore without changing image bytes.
- Clear visibility, native restart, and restore.
- Fallback texture visibility through a disk toggle.

Verified artifacts in
`.audit-output/final-performance-audit-20261011/native-release/`:

- `native.log` records 2/2 preview and 7/7 workspace cases, fixture removal and
  owned process-tree shutdown after each spec.
- `native-verification.json` records 1,577 inputs with zero drift, private binary
  `src-tauri/target/release/emmm-e2e.exe`, SHA256
  `c5a99c53b39903bbc9ea51e4bc7af7430108ec97cd1f1a96130f6c52aa00c3ca`.
- `after-environment.json` records zero active native processes and installed
  database unchanged; owned onboarding fixtures were removed. The workspace spec
  also reports removal of its two scheduled fixtures.

Native coverage uses synthetic owned data. Deferred clipboard/picker races and
external viewer launch ownership are unit-tested; real clipboard image paste and
the user's external viewer executable were not exercised by these nine cases.
Implementation history: `docs/history/202610110031-fix-dashboard-preview-ownership.md`.

## Source-complete checks after the hotkey handoff

- Move-outside + junction-back was reproduced on Windows (identity proof accepted
  the escaped folder). Validation now re-canonicalizes and checks containment;
  all 17 guard tests pass.
- Prepared viewer launch owns the coordinator lease from before manifest hashing
  through final proof checks, INI inspection and process spawn. Two lease tests
  pass; all eleven settings command tests pass.
- Enabled/disabled relaunch previously kept two receipts for one review key;
  snapshot matching now uses the same identity key as review queries. The deferred
  old-analysis regression passes with the other five state tests.
- Actual Specta export and registry permission test pass after adding the injected
  coordinator State; frontend launch arguments remain unchanged.
- Unique targeted Rust coverage: 43 passed, one existing manual benchmark ignored
  (17 guard, 9 preview operations, 2 thumbnail, 11 settings, 2 registry/export,
  2 requested game-detector tests). Overlapping lease/filter reruns are not added.
- Combined frontend: eleven files, 67 passed and one existing skip. The final keyed
  per-game Dashboard composition was separately rechecked: four tests passed.
  This avoids render-time ref access while synchronously canceling old contexts.
- App and E2E TypeScript, strict scoped ESLint, architecture checks, scoped
  Rustfmt/Prettier and diff checks pass. An initial E2E command used a nonexistent
  root config; the correct tests/e2e/tsconfig.json check passes. The initial key
  warning in the wizard suite comes from its existing test-list mock.
- Independent refreshed review reports no actionable finding in the three final
  guard/lease/identity fixes.
- Cargo/compiler/native/driver slots were released. The performance task owns one
  fixture-release capture with six performance and nine Dashboard/preview cases;
  all source/build inputs stayed frozen during that completed capture.

This is scoped validation, not a clean Clippy or whole-repository certification;
existing Rust dead-code warnings remain.

## Shared workspace coordination

The hotkey task completed its owned native shutdown and handed off source. This
task closed the three reviewed follow-ups and released Cargo/shared-registry.
After the combined capture's verified shutdown, the performance task handed the
slot to the separately authorized hotkey production build/install task. This task
remains source-frozen and closes documentation only. Vite and hotkey notification
commands/controller/capabilities are preserved. No installed app, real library,
dependency, worktree, commit or production reset was changed by this task.
