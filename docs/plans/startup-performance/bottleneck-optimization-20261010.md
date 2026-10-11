# Startup bottleneck optimization

## Scope and baseline

Implement the user's approved keybinding fix and bounded census experiment.
The real native audit measured Home 0.9–1.3 s, core Ready 4.4–4.9 s and optional
keybindings 6.5–12 s on 632 GB. Census consumed 1.90–2.06 s; classification
0.45–0.57 s and projection 0.81–0.97 s. No new architecture, dependency,
migration or persistent filesystem cache is needed for the first improvements.

## Ordered work

1. **Effective keybinding filter:** Expose the existing Library stored-path
   predicate through its API facade and reuse it in dashboard row selection.
   Exclude any DISABLED path component before filesystem harvest; retain own
   mod status, metadata fallback and stable query ordering. Reproduce the bug
   with nested ancestors, mixed separators/case, another game and an enabled
   control row. Expected real candidate roots: 1,125 → 287, not a promised
   wall-time percentage.
2. **Bounded census:** Compare serial traversal with two and four concurrent
   independent top-level roots using existing Rayon. Preserve every census
   field, directory inclusion, unavailable-source error, cross-root ambiguity
   and progress semantics. Keep only a repeatable improvement above variance;
   choose the smallest effective concurrency bound. Benchmark sorting separately
   if lowercase-key allocation remains material. Add no persistent cache.
3. **Native regression and measurements:** Run focused Rust tests, formatting,
   strict Clippy/check and owned-fixture Windows E2E. Pure census benchmarking
   against GIMI may only enumerate/read: no app launch/config changes, content
   mutation, rename, deletion or repair. Compare exact output with serial and
   use manifest metadata before/after. Actual app benchmarking requires the
   existing private read-only harness and full integrity/restoration gates.
4. **Next measured opportunities:** After the filter, isolate keybinding INI
   discovery/read/parse and response costs before adding bounded parsing or a
   lighter parser. Projection includes metadata and SQL; add no-change guards
   only after profiling and regression evidence. These are follow-up decisions,
   not speculative changes bundled into this patch.

## Acceptance

- Dashboard returns only effectively enabled mods and does not alter stored state.
- Census outputs and failure behavior remain equivalent; bounded work cannot
  publish a successful partial census or regress progress counts.
- Record alternating same-build serial/parallel samples, parity and retained /
  rejected experiments. Distinguish stage timing from total startup.
- Preserve unrelated changes and real GIMI contents; restore any temporary
  configuration to isolation before completing.

## Results

### Effective keybinding filter

Regression RED returned five candidates instead of one, including four disabled
paths. GREEN returns the one effectively enabled candidate, preserves catalog
metadata and leaves all five own-enabled statuses unchanged. Production reuses
the existing Library predicate through its API facade.

The archived audit DB was opened read-only and the real Mods library only read.
Five alternating warmed harvest pairs measured the original 1,125 roots against
the 287 effectively active roots. Production candidate rows matched the expected
filtered rows exactly, including ordering and metadata. Every filtered harvest
retained exactly the same 640 active binding records as the original output
restricted to effectively enabled paths.

Original samples (ms): 1,983.813, 1,758.348, 1,851.001, 2,435.079, 2,122.634.
Filtered samples (ms): 569.914, 544.368, 669.932, 710.689, 591.612.
Median 1,983.813 → 591.612 ms: 70.2% / 1,392.201 ms less warmed harvesting work.
The ranges are separated. This includes row cloning, INI discovery/read/parse;
it excludes query time, IPC, response serialization and frontend rendering.
Do not apply that percentage to full startup or OS-cold keybinding loading.

Raw samples are retained in ignored
`.audit-output/gimi-census-20261010/keybinding-filter-readonly.log`.

### Census experiment ledger

Final measurements use five alternating pairs against the original serial DFS
reference and the actual public production collector, on the same unchanged
GIMI Mods root. Every run matched 148 roots, 30,290 directories and all four
entry fields exactly.

| Collector                      |       Median | Range                  | Decision  |
| ------------------------------ | -----------: | ---------------------- | --------- |
| Original serial                | 1,838.443 ms | 1,794.984–1,877.743 ms | Reference |
| Production bounded, four roots | 1,344.905 ms | 1,329.332–1,367.853 ms | Retained  |

The median stage reduction is 26.8% / 493.538 ms. An independent final sweep
of concurrency one/two/four measured medians 1,785.111 / 1,608.811 / 1,336.437 ms.
Four is retained as the smallest tested bound with the stronger reduction.
Two was measured but is not the selected production bound.

An earlier sweep measured 2,322.247 / 2,042.247 / 1,449.396 ms while another
Rust compilation was observed. It is retained as exploratory evidence; the
direct original-serial control above is the reported comparison. No cached-sort
optimization was added. Five focused correctness tests pass, one manual test is
ignored by default; scoped formatting and review pass.

Progress callbacks remain ordered and terminal counts unchanged, but each batch
finishes before its callbacks are emitted. Startup uses no progress callback.
Errors still prevent a successful partial census and preserve serial progress
prefixes. No new thread pool, disk cache or content/size scan was added.

Raw census commands/samples are retained in ignored
`.audit-output/gimi-census-20261010/census-parallelism-worker.log`.
These are warmed/static census-stage measurements, not OS-cold startup, p95
or a measured percentage improvement of the full application startup.

### Integrity and validation

Both pure native benchmarks performed reads only; no real app launch, config
selection, repair, rename or deletion occurred. Full-root metadata inventories
before/after matched exactly: 375,099 files, 30,317 directories, zero links and
632,024,978,228 bytes. Their SHA256 is
`09D200FB0D4504CFEB73AFCA1E1AFF36864A400B0CD7073079250E19002D4CEA`.
This follow-up compares metadata manifests; it does not repeat the prior full
632 GB content checksum. The INI benchmark opens the archived audit DB read-only
and uses read-only INI discovery/document functions. Configuration remains
isolated; real GIMI was never selected in a live app during this follow-up.

- Dashboard/service tests: 17 passed, one manual benchmark ignored by default.
- Full E2E TypeScript and focused E2E ESLint pass.
- Census source and keybinding source reviews found no actionable defects.
- Reconciliation tests: 230 passed, three ignored (including manual benchmark);
  these include the five new census tests. Whole-workspace concurrent edits are
  included in that scoped suite count, not attributed to this change.
- Strict `cargo clippy --lib -- -D warnings` was attempted and blocked by
  unrelated ongoing changes: unused `ini/write.rs::write_synced` and two
  `nonminimal_bool` findings in `mutation/recovery.rs`. No lint allowance or
  unrelated fix was applied.
- After the native pass, another shared-workspace edit removed the unused INI
  helper. The resulting Clippy retry instead stopped at E0061 in
  `automation/application/hotkeys/manager.rs:435`: its two-argument safe-mode
  call does not match the current one-argument `safe_mode.rs:15` signature.
  These files are outside this optimization's scope and were preserved. The
  passing native build/tests describe the validated snapshot before that edit,
  not a claim that the entire concurrently changing workspace now passes.
- Scoped Rust formatting and `git diff --check` pass.
- Fresh frontend TypeScript/Vite build and native debug application build pass.
- Native Windows E2E: one passing test / three process restarts on 301 owned
  fixture mods. Each restart asserted F1 available, F12 beneath a disabled
  ancestor absent in IPC/visible Home, all 301 indexed mods, one watcher, one
  StartupBoot baseline and zero full GameSwitched baselines. Deferred staging
  and task maintenance assertions pass; owned process teardown/fixture cleanup
  completes. Page-clock shell observations were 975 / 789 / 964 ms; these are
  small-fixture observations after navigation, not real-library restart times.
- Initial native attempt was blocked by another driver on port 4444 and ran
  no app assertions; no process was terminated. The port became free before the
  successful fresh-build retry. The harness logged a failed session attempt
  after rejecting its prepare/build marker; this concurrency guard behavior
  merits a separate harness review, not a production startup conclusion.
- The passing test logged a stale hardcoded `fixture_mods=300` despite its
  301-mod assertion. Logging now uses the asserted object count; only logging
  and reuse of that lookup changed after the native pass, validated by lint/types.
- Both ordinary and read-only E2E profiles were checked to have zero GIMI
  bindings before native fixture launch. No production install/config changed.
