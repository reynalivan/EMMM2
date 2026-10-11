# Settings, shell, telemetry and warning validation

11–12 October 2026. Implements the five confirmed Settings/shell findings and four
verified cleanup items from [the audit](../audit-settings-shell-20261011/report.md).
The later request to fix all warnings includes full lint, architecture checks,
Rust warnings and repository line-ending diagnostics. No checks were disabled.

## Implemented behavior

- Frontend diagnostics disable automatic exception/trace/page/user/session
  producers and reconstruct only allowlisted diagnostic events. Sampling remains
  local to the running process. Explicit voluntary crash reports remain available.
- Backend producers capture consent epochs before waits; the existing bounded
  queue, writes and export acknowledgements reject retired epochs. A short writer
  gate serializes revocation and persistence. Whole-settings saves cannot change
  consent outside the dedicated command. Client IPC errors retain the original
  enabled settings revision; the backend captures an epoch before validating it.
- Compact launch actions follow the actual menu node across navigation. Game
  editor saves, errors, pickers and parent inspection callbacks belong to their
  originating session; pending native cancel is blocked. Resize termination
  restores prior global styles on completion, blur, responsive teardown or unmount.
- Removed two re-export shims, unused is_conflict_ignored and the test-only strict
  classifier wrapper. Coverage now exercises the active scan/classification path.
  Tables, migrations, recovery and filesystem identity checks remain intact.
- Removed an unused object runtime lookup and unused reconcile size projection.
  The old scalar INSERT helper now lives only in its independent benchmark test.
  Database size storage, insertion defaults and existing values are unchanged.

## Automated checks

| Check                                                                                                         | Result                                   |
| ------------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| Vitest: shell/editor/grid, telemetry, IPC, harness, affected QueryClient mocks and two corrected test imports | 128 passed across 20 files               |
| Rust: telemetry, classifier, conflict and ConfigService                                                       | 43 passed                                |
| Rust: originating-revision native metric admission                                                            | 4 passed                                 |
| Rust: runtime_sync after private completion tuple alias                                                       | 7 passed                                 |
| Actual Specta/registry tests and generated command bindings                                                   | 2 passed                                 |
| Final full frontend suite, direct Node and raw stderr                                                         | 1,517 passed; 1 existing skip; 243 files |
| Final full default Rust library suite                                                                         | 1,855 passed; 40 ignored; 0 failed       |
| App TypeScript and E2E TypeScript                                                                             | Passed                                   |
| Full ESLint, --max-warnings 0                                                                                 | Passed                                   |
| Full architecture ESLint, --max-warnings 0                                                                    | Passed                                   |
| Full authored-project Prettier, --check .                                                                     | Passed                                   |
| Clippy --all-targets --all-features -- -D warnings                                                            | Passed                                   |
| git diff --check                                                                                              | Passed without diagnostics               |

Regression assertions for async session ownership, portal lifecycle, resize and
diagnostic boundaries failed before their fixes and passed afterward. The
integrated run exposed incomplete QueryClient mocks; three demonstrated failures
were corrected with typed partial mocks. The three composition tests now live in
the app layer and use public widget APIs. Two older tests now import entity APIs
through their public barrels. The final 128-case run has no skipped cases.

Prettier warnings in eight ignored audit scripts/configurations were corrected
without behavior changes, including three performance-owned files with permission.
Before-content and SHA archives are retained in the task artifact directory.
Thirteen existing CRLF files were normalized to the repository's LF rule; every
result matched the original content with only CRLF replaced. A private tuple alias
removed the Rust type-complexity warning. The native launcher uses Node directly
for the existing Tauri script, removing DEP0190 without changing build arguments,
environment, signing preparation or isolation checks.

A final full-project format check initially flagged 173 files, mostly old audit
evidence and generated outputs. Added the already-planned audit/browser artifact
directories to .gitignore so immutable snapshots stay out of source control and
the formatter. Prettier leaves the generator-owned pnpm lockfile and Specta
bindings in their native output format; registry/serialization tests still
validate those contracts. No lint, type or Clippy rule was disabled. Reformatted
the remaining eleven documentation/data files and index.html. The JSON data is
identical and the HTML differs only by whitespace in the CSS font-family list;
all before/after content and hashes are archived. Full Prettier now passes.

Release-only warning cleanup additionally passed 47 catalog/projection/rename/
source-recovery tests. The independent paired INSERT benchmark passed separately
on its owned 2,048-row database across five alternating scalar/batched pairs.
The projection filter skipped three manual benchmarks before that explicit run;
no performance timing claim is inferred from this parity check. Production-only
Clippy `--lib --bins -- -D warnings` passed. A focused independent review found no
actionable defect in the seven-file cleanup.

The stricter release-profile Clippy check additionally found five internal game
enum acronym warnings and one shared suffix warning in refresh scopes. This
follow-up preserves numeric game serialization, explicit game/resource labels,
the nine existing refresh wire strings and the uppercase preflight cache key.
Two wire-contract tables and the exact cache-key assertion passed before the
rename. Afterward, wire contracts (2), game tests (41), switch tests (22), cache
assertion (1) and actual Specta export (1) passed. Release production Clippy
`--release --lib --bins -- -D warnings` and all-target/all-feature Clippy both
passed. All 81 files matched approved substitutions/serialization/cache edits;
78 were only qualified-name substitutions. Bindings before/after are identical:
SHA-256 `a2790111d5d4bfea4948bc8989eaee7bfde9f17cff64981a1c3c68015745917f`.
Two additional touched test files were normalized to LF without semantic changes.

Complete direct-Node logging exposed React warnings hidden by the pnpm launcher.
The first four affected test files now pass 44 tests with 36 act and two unknown
prop warnings reduced to zero. A full raw rerun confirmed those four repairs and
identified 25 act plus four mock-prop warnings in ten additional files; these
remaining ten files now pass 138 tests with all 25 act and four prop/handler
warnings reduced to zero. Final focused logs have no stderr contexts. Three
missing positive-test API mocks were completed in ImportBatchWizardHost. Expected
negative-case error logging elsewhere is retained. No React warnings are suppressed.
The VirtualList classification stub now honors getItemKey (7 tests passed), and
typed collection-query mock replies remove six undefined-query warnings from
useAppStore (44 tests passed). Production behavior and negative-case assertions
remain unchanged. The first complete rerun during native compilation had one
5-second listener-race timeout; the unchanged isolated test passed. The final
full run without compiler contention passed all 1,517 tests in 243 files with one
existing skip in 42.40 seconds. Raw frontend-publish-final.log has zero warning
lines, React/act/key/DOM warnings, missing mocks or undefined-query diagnostics.
All 18 remaining stderr contexts are explicit negative/error/retry/write-failure
or unavailable-listener cases. No timeout, logging or check was relaxed.

The initial full Rust library run passed 1,853 tests and failed two test-drift
checks: the INI geometry golden predated approved layout revision 7, and removal
of the legacy classifier wrapper omitted directory admission in the frozen
census reference. Corrected only those test expectations/reference behavior;
production paths and failure assertions are unchanged. The former failures
passed individually, with the census group passing 27 tests and skipping eight
manual benchmarks. Final cargo test --lib passed 1,855 tests, zero failed and
40 ignored in 92.78 seconds; rust-lib-final.log contains no compiler warnings.
Generated bindings still match the recorded a2790111 SHA-256 after the full run.

## Native verification

Fresh private release fixture verification runs through
`tests/e2e/specs/settings-shell-lifecycle.e2e.ts`, using the existing guarded WDIO,
tauri-driver and WebView2 lifecycle. Frontend production build passed without
warnings. The first Rust release build exposed three dead-code warnings hidden by
test callers; the validated follow-up cleanup is described above. The first native run passed
the resize case and failed two selectors: the actual English labels are
`Randomizer (Gacha)` and `Edit Game`. Corrected the spec against source translations;
these selector failures do not establish an application defect. Its 1,585 captured
inputs had zero drift, the owning process tree stopped and both fixtures were
removed. The installed executable and 13 protected installation/config/DB files
were unchanged.

The next capture passed compact navigation but found that the fixture had no
standalone game executable, correctly disabling Save. The spec now enters its
owned existing executable and waits for actual form validity. Dialog cleanup
between cases prevents a failed case from blocking later navigation. A subsequent
capture passed compact navigation and resize, but its save barrier did not attach:
Tauri's installed invoke property is read-only. No application failure is inferred
from that instrumentation defect. All three captures had 1,585 inputs with zero
drift and confirmed owned shutdown/two-fixture cleanup.

The replacement barrier holds original callbacks and unchanged native
SaveSettingsResult responses through the mutable callback map. Restoration disables
holding for callbacks registered earlier; an actual-hook Node VM regression
failed on late completion before that guard, then passed normal-release and
completion-after-cleanup checks. Independent review closed the cleanup finding.
Final real-response/native verification passed all three cases in 5.6 seconds on
WebView2 154.0.4258.62: compact navigation, actual Escape while the real native save
completion is held, and global resize-style restoration. Fresh release build and
frontend production build passed without warnings. Private binary SHA-256:
`6886b63c5a76c765c9d7c6a261c1974bfda927a3f421ea223bf97af77d769227`.
The harness confirmed owned shutdown and removed both fixtures.

The 1,585-input verification contains exactly five explicitly verified test-only
deltas: missing API mocks in ImportBatchWizardHost.test.tsx, the keyed VirtualList
stub in ObjectClassificationWizardHost.test.tsx, collection-query mock replies in
useAppStore.test.ts, the approved INI geometry golden, and directory admission in
the frozen census reference. Their before hashes match the original capture and
after hashes match current files; four have exact archived content, and the INI
golden receipt reconstructs the prior bytes by reversing its single literal edit.
The final formatter pass additionally changed index.html by whitespace only;
its archived bytes have identical content after whitespace removal. All other
production inputs and every harness input match. The generic verifier returns
exit 1 for these six byte deltas; the scoped verifier checks all five test-only
deltas, the separately classified HTML formatting delta and private binary hash.
No whole-tree zero-drift claim is made for this final capture. Installed
executable/config/DB preservation still matches all
13 protected files; installed SHA-256 remains
`521a04c3f577efaa88bc351d79f0841344824a555f3251557acb547ce8a78077`.

## Limits and preservation

An independent read-only review found no remaining actionable defects in the
changed consent/persistence, diagnostic payload, UI ownership or launcher scope.
No actual collector/network send, external picker interaction or real game launch
is asserted by the tests. Conservative revision fencing can drop a best-effort
client error metric after an unrelated settings update; operation telemetry uses
the captured consent epoch. Already-dispatched HTTP requests retain the existing
policy; stale completions cannot persist acknowledgements under a new epoch.

This pass made no migration, new dependency, installed executable update,
real-library mutation, commit or push. The separately authorized release chat
receives the stable source/build handoff after final documentation and hash checks.
Unverified reset, thumbnail-age and get_logs candidates remain
outside this fix. Ignored evidence is under
`.audit-output/audit-settings-shell-fixes-20261011/`.
Final full-suite raw logs and their diagnostic scan are under
`.audit-output/release-main-20261011/`.
