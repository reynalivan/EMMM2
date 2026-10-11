# Runtime indexing implementation — 2026-10-10

User approved implementation of `runtime-indexing-audit-20261010.md` with
“oke perbaiki semua”. Scope is the existing runtime queue, scoped harvest cache,
projection metadata and verified catalog inputs. No dependency, schema, worker
pool, persistent cache or second scheduler was added.

## Implemented flow and stability

- Accepted requests retain pending intent in the existing latest-wins queue.
  Hotkey saves await an optional oneshot on that job and return committed
  warnings. Overwritten pending callers receive Superseded; receiver cancellation
  does not cancel work. Replies preserve typed application errors.
- Superseded publication is distinct from legitimate disabled/readonly Skip.
  It cannot ACK pending effects or emit successful finishing. A current job that
  loses activation authority ends with Failed/pending rather than staying Running.
  A queued successor keeps ownership of its own terminal status.
- Legacy publication-only recovery checks the original queue generation under
  the existing admission lock before reserving/staging a recovery. A newer real
  request cannot be overwritten by that older recovery. Hotkey callers no longer
  restage old failures after a newer job has settled.
- Publication authority is checked before reload/maintenance. Generation cleanup
  uses the same revision/activation commit guard; cleanup errors remain deferred
  warnings. A stale stopped-game request cannot delete legacy resources while its
  successor has not published yet.
- Scoped composition collects old/new paths, deduplicates them and invalidates all
  capability variants in one cache retain. Other games stay warm. One unique path
  uses direct equality, avoiding per-entry hashing on small updates.
- `info.json` is read once. Each existing 64-row batch awaits one blocking job;
  ordered groups use at most eight parsers on the existing Rayon pool. Observations
  keep optional name/safety fields. Current manual policy, physical identity and
  desired fields remain inside the writer transaction. No shorter writer lock hold
  is claimed. Progress still advances after successful SQL.
- INI content carries its already-computed digest into parsing. Catalog work is
  off the async executor, loads the manifest once, fingerprints those loaded bytes
  and parses the exact catalog buffer that passed checksum verification. Avatar
  validation remains complete. Existing logs now distinguish catalog/matching and
  game-lock wait time.

## Paired measurements

Owned OS TempDir fixtures only; debug test executable, uncontrolled OS cache,
alternating algorithm order and exact result/cache parity assertions. Metrics are
stage wall time, not total native indexing/startup. Final runs observed no CPU
increase in the user's installed EMMM process; the process remained running and
was never operated or stopped by this task. Other system activity is uncontrolled.

| Workload                                            | Previous algorithm median | Implemented median | Samples per algorithm |
| --------------------------------------------------- | ------------------------: | -----------------: | --------------------: |
| Cache invalidation, 2,048 affected mods             |                387.066 ms |           8.651 ms |                     5 |
| Cache invalidation + harvest, same root             |              1,194.578 ms |         804.123 ms |                     5 |
| Cache invalidation + harvest, one mod               |                  1.154 ms |           1.096 ms |                     5 |
| Metadata, 2,048 mods / 683 realistic JSON, one root |                 69.899 ms |          56.654 ms |                     6 |
| Metadata, same corpus / 16 roots                    |                 62.958 ms |          54.529 ms |                     6 |
| Catalog, 512 entries + 512 fully validated avatars  |                548.566 ms |         539.845 ms |                     7 |

Cache fixture: two games × 2,048 mods × two capabilities = 8,192 entries,
capacity 14,336; 3,939-byte INIs. Root invalidation reduces 4,096 retain passes to
one. Invalidation + harvest improves about 32.7%; the one-mod ranges overlap,
so its small median difference is not a reliable separate speedup claim.

Metadata serial baseline already uses the new single-read helper: the paired
13.4–18.9% reduction measures bounded offloading/concurrency, not the removed
exists/read roundtrip. The current-thread scheduling probe improves from roughly
63–70 ms to 63–70 microseconds median; this is executor responsiveness, not UI
paint/input latency. Retained observation fields peak at 2,290 bytes in this
fixture, excluding parser/path/transient allocation.

Fresh projection + read-model + commit median is 125.172 ms; unchanged projection
is 104.037 ms. Metadata subsets are 50.351 / 48.399 ms and read-model subsets are
2.523 / 2.577 ms. This is a current snapshot, not a controlled total-projection
before/after comparison. Discovery, rename preflight, runtime and UI are excluded.

Catalog contains 732,135 JSON bytes and 33,657,344 avatar bytes. The ~1.6% median
change is small, with overlapping ranges; retain byte reuse for snapshot
correctness without claiming a large catalog speedup. Full avatar integrity work
remains a measurable cost, not an excuse to bypass validation or add speculative
caching. Shared INI decoding and a lighter avatar reader remain deferred.

Whole-process observed peak working sets are 49.3 MiB cache, 19.3 MiB metadata,
33.8 MiB projection and 28.2 MiB catalog. These include fixture setup and both
algorithms, not per-algorithm allocations. Batches limit entry count, not file
bytes; unusually large metadata can still increase transient memory. No artificial
name/file limit was introduced that would change existing metadata policy.

Raw logs, exact executable digest, observed source hashes, process measurements
and machine-readable medians/ranges are under
`.audit-output/runtime-indexing-implementation-20261010/`. The first cache probe
and its process evidence are preserved separately; it ran while the installed app
was CPU-active and exposed the single-path hashing overhead.

## Verification

- Semantic RED then GREEN: revision-only supersession retained pending effects;
  duplicate INI SHA count; changed-after-checksum catalog bytes; stale stopped-game
  legacy cleanup. Optional reply ownership/cancellation, typed failure, valid Skip,
  manual reload, merged intent, metadata order/current policy/rollback and catalog
  error parity have focused regression coverage.
- Backend checkpoint: 1,765 passed / 32 ignored; cache fast-path follow-up: 68
  passed / 1 ignored. Four ignored performance fixtures were explicitly executed
  and passed. Latest frozen backend suite passed 1,785 tests / 32 ignored in
  153.89 seconds. The intermediate conflict test failure was an escaped raw-string
  expectation; its corrected three-case group suite passed before the full rerun.
- Strict Clippy `--lib` and `--all-targets -- -D warnings` passed, including the
  reviewed runtime source and the v4 browser/duplicates checkpoint from the
  coordinated chat. Its targeted 57-case suite also passed.
- Application and native spec typechecks, scoped ESLint, Rust formatting and
  architecture lint passed. The composed file-lock dialog suite was moved to app
  integration ownership and uses public feature APIs; 15 tests in two files passed.
- Fresh native final validation passed: four cases across
  `normal-sync-keyviewer.e2e.ts` and `startup-reopen.e2e.ts`, including full/scoped
  projection, committed hotkey publication failure/retry, overlapping runtime
  requests with actual terminal events, owned duplicate migration and three process
  restarts. Each restart had exactly one baseline and one watcher installation.
  All 1,446 captured native build inputs remained unchanged through the run.
  Native executable SHA256:
  `52f24a67230be45d95bd578014df1cf39ddfce5f2c282d5265f68902a3978298`.
  Its matching executable was preserved as
  `.audit-output/runtime-indexing-implementation-20261010/emmm-native-validated.exe`
  before the shared target directory was released to other validation work.
  The harness stopped its owned process tree and cleaned its registered fixtures.
  The earlier build was interrupted before app/driver launch when another chat's
  source changed; it is not counted as completed native validation.
- Restart shell observations were 543 / 564 / 319 ms of WebView page time. They
  exclude OS process launch/WebView creation and are neither a controlled before/
  after startup benchmark nor a p95 claim. The real native onboarding picker and
  Finish Setup were not exercised in this session.

## Residual limits and deliberate deferrals

- The metadata writer transaction still spans observation; memory limits entry
  count rather than arbitrary JSON bytes. Changed metadata validation/name limits
  require an explicit compatibility decision.
- Verified catalog/avatar work and filesystem enumeration still cost time.
  Catalog timing did not justify a new cache or weaker integrity checks; shared
  INI decoding and incremental aggregate caches remain deferred.
- Native coverage uses synthetic fixtures and a stopped/unknown mock game. Real
  game reload, OS-global shortcuts, cold-disk workloads and a real hundreds-of-GB
  library are not certified by these results.
- A cross-cutting source review found no unresolved findings in this change's
  queue/publication/settlement boundaries. This is not a claim that every possible
  application bottleneck or unrelated feature risk has been eliminated.

All benchmark/library writes target owned synthetic fixtures. No command in this
task points at `E:\Data\XXMI Launcher\GIMI`; the installed application and its
configuration are unchanged by this task. Fixture native profile remains
`com.reynalivan.emmm.e2e` and OS global shortcuts stay disabled there.

The finite blocking jobs follow [Tokio's blocking-work guidance](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html): started jobs cannot be aborted,
so cancellation may finish one bounded observation batch. Ordering uses the
existing [indexed Rayon iterator](https://docs.rs/rayon/latest/rayon/iter/trait.IndexedParallelIterator.html), without a new pool or unbounded task fan-out.
