# Indexing efficiency audit and measured changes

## Scope

Requested audit and local optimizations for onboarding, sync and finishing.
Owned synthetic fixtures only; no GIMI reads, production configuration writes,
installation, new dependency, daemon, cache, schema or concurrency increase.
CodeGraph's index was frozen; current source was inspected directly.

## Changes retained

1. Fresh mod projection previously awaited INSERT then identity UPDATE per row.
   The existing transaction now inserts at most 64 new rows per statement with
   identity already bound. Eleven binds per row stay below the historical 999
   parameter ceiling. New referenced rows flush before collection FK rebinding.
   Existing-row identity swaps, manual safety, info.json names, Unicode keys,
   collection repair and storage-size behavior are retained.
2. Both importer preflight traversals previously descended into Mods/assets,
   then excluded mod files. A shared traversal prunes the configured canonical
   Mods root before descent and canonicalizes that root once. Secondary include
   roots and Core configuration remain visible. Depth 5, no link following, INI
   limits, content hashes and snapshot verification remain unchanged.
3. Preparation progress counts completed roots; projection counts actual mod
   rows processed after successful SQL. Phase changes reset units, not ownership.
   Finalizing starts before reference repair/pruning/signature/read-model work;
   it has no percentage or ETA. Scan elapsed time cannot create a projection ETA.
   Late-event/session/retry guards and throttling remain. Onboarding and activation
   overlays use the phase's units instead of falling back to stale root totals.
   Failed classification cannot increase the classified count; a new projection
   pass clears the rechecking label. Rechecking is derived from the snapshot
   instead of a second boolean state. Both defects were reproduced before fixes.
4. Removed unused file-size-weighted progress and cross-game ETA helpers, their
   implementation-mirroring tests, and redundant frontend duration/progress state.
   They had no product consumers. Measured elapsed display now floors seconds
   and shows zero at zero; it no longer uses an estimate formatter that rounded up.
5. Existing onboarding width is divided by roots in the current batch. Spare
   slots process immediate child branches in bounded chunks, with serial recursion.
   Shared per-root throttling and indexed merge retain event rate and output order.
   `roots × floor(width / roots) ≤ width`; no pool or concurrency budget increase.

## Measurements

Single-root mixed discovery (same-build serial control / width 8):
**603.623 → 279.135 ms, 53.76% lower**.

| Measured scope                                      | Before median | After median | Result                     |
| --------------------------------------------------- | ------------: | -----------: | -------------------------- |
| First projection + read-model + commit, 2,048 mods  |    439.743 ms |   151.383 ms | 65.6% lower                |
| Unchanged sync, same 2,048-mod topology             |    127.066 ms |   126.813 ms | Within noise; no new guard |
| Paired INSERT + identity + commit, 2,048 rows       |    224.561 ms |    30.815 ms | 86.3% lower                |
| Preflight snapshot, 32 generated mods × 3 assets    |     14.042 ms |     1.102 ms | 92.2% lower                |
| Preflight snapshot, 2,048 generated mods × 8 assets |  1,746.774 ms |     1.130 ms | 99.94% lower               |

Projection: six fresh/unchanged pairs before then after, equivalent generated
topology with 16 objects and **683** info.json files. Fresh total ranges
414.699–480.157 ms before and 140.866–182.564 ms after. Discovery/setup excluded;
initial index, identities, objects, mod metadata, pruning, read-model and commit
included. Metadata timings also changed; do not attribute the entire total delta
to SQL alone. The separate SQL experiment uses five alternating pairs in one
owned DB, includes commit and proves selected persisted-field parity; ranges
195.565–259.202 ms before and 29.512–33.262 ms after.

Preflight: five alternating pairs per topology; exact snapshot path, identity,
length, modification time and SHA content parity across all 20 observations;
renderer/callback parity against an independent baseline capability traversal.
Visited entries dropped 204→7 and 22,540→7. This measures one snapshot traversal,
not whole preflight, KeyViewer, Home or startup. Cold/cache-miss preflight still
retains its parse and verification passes; correctness barriers were not removed.
Raw rows were transcribed from observed tool output and that provenance is explicit.

All timing uses debug Rust tests and uncontrolled/warmed OS caches. Fixtures
contain realistic file counts but small contents. No p95, OS-cold, release build,
real ratusan-GB throughput or whole-onboarding percentage is established here.
Raw logs/rows: `.audit-output/indexing-efficiency-20261010/`.

Single-root discovery uses five alternating width 1/2/4/8 rounds and 2,000
flat/variant/asset mods, with exact every-field census/projection parity. Median
widths 1/2/4/8: 603.623/575.202/352.093/279.135 ms. Serial range
596.377–662.743 ms; width 8 range 261.699–342.794 ms. Retain width 8.
The earlier serial median 1,652.994 ms warmed to 603.623 ms; that before/after
comparison is rejected for attribution. The gain uses the same-binary serial
control. A successful 21-test patch build supplied the hashed benchmark executable
while an unrelated lifecycle refactor temporarily broke fresh compilation.
Provenance and all 40 rows are in `single-root-classification.json`; final
source/native gates remain mandatory. Three-root and nine-root partial-tail
parity regressions also cover widths 1/2/4/8 on the final source.

## Review and stability

Independent source review found one stage-placement gap: 100% projection persisted
during pruning. Fixed by forwarding Finalizing before writer finalization. No
remaining confirmed scoped finding after recheck.

RED proved per-mod second identity writes and traversal through excluded Mods.
Regressions cover 130-row batch boundaries, metadata/manual safety/disabled Unicode
folders, monotonic processed counts, unchanged sync, later-batch failure rollback,
no false finishing, relative/canonical roots, Windows junctions, sibling and
secondary include trees, importer digest invalidation. Existing offline swaps,
collection references and scoped projection tests remain intact.

- Full Rust snapshot: **1,685 passed, zero failed, 25 ignored**, 64.67 seconds.
  Targeted writer: **23 passed / 3 ignored**, including later-batch rollback.
  Preflight: **15 passed / 1 ignored**, including owned Windows junctions.
- Final source after branch/accuracy fixes: **1,723 passed, zero failed,
  27 ignored**, 88.99 seconds. Discovery scope **23 passed / 4 ignored**,
  including mixed/partial root batches and unsuccessful classification count.
  Related frontend remains **53 passed** after elapsed/recheck state fixes.
- Related frontend: **53 passed across 8 files**. The reduced count reflects
  removal of six tests covering unused helpers and one new projection UI case.
- Strict all-target Clippy passes after replacing a seven-field test tuple with
  a named parity record. Frontend/E2E typechecks, scoped ESLint including
  architecture lint, Prettier and scoped Rust formatting pass.
- Full Rust formatting initially passed, then a concurrent edit outside scope
  left `organizer_move_tests.rs` unformatted. That file was preserved; final
  formatting certification is scoped to the changed indexing/preflight files.
  Other chats' changes mean these results do not certify an immutable whole repo.
  After cross-task coordination settled those edits, final full Rust formatting
  passed again. Final strict all-target Clippy (`-D warnings`) passes with no
  issues. Native final-source result follows below.
- First fresh native validation: **3 cases / 2 specs passed**, sessions
  `c842f59366c363de9bdcef2634c90819` and `30466e8acdeaaa23afa95ee687656cc0`.
  Actual picker/Confirm covers 6,000 foreground mods, 80 background mods, real
  intermediate/final mod counts in the UI and native restart. Three additional
  301-mod restarts each install one watcher and one startup baseline. All three
  owned fixtures were removed after their owning native process stopped.
  Home observed **7,359 ms**, versus the prior single observation 7,551 ms;
  this difference is insufficient to establish a total onboarding improvement.
  Page-clock shell observations 716/287/605 ms are not process-launch latency.

The native timeline identified the remaining dominant first-run stage:
preparation **4,612 ms** (census work 602 ms, classification work 3,544 ms),
projection **453 ms**, handoff **454 ms**. Wall stages do not sum to Home:
frontend/activation and observer overhead remain. A single-root branch-budget
experiment was retained because root-only parallelism could not distribute
its 6,000 independent mod children.

## Final native result

Fresh private Windows build: **3 cases / 2 specs passed**, sessions
`c485ac70d3a1ff55c0a2c147afa099a1` and `526f062850b909cc66456b3aa011ce69`.
Actual native dialogs/Confirm index 6,000 foreground mods plus 80 background
mods, expose intermediate/final projection counts in the real UI, and preserve
the index/selection after process restart. Three 301-mod restarts each retain
one watcher and one startup baseline. All three owned fixtures were removed
after their owning process trees stopped.

| Native observation, same generated topology | Before branch budget |            Final |
| ------------------------------------------- | -------------------: | ---------------: |
| Confirm to Home, 6,000 mods                 |             7,359 ms |         3,920 ms |
| Discovery/preparation wall                  |             4,612 ms | 1,210 / 1,211 ms |
| DB projection wall                          |               453 ms |           481 ms |
| Handoff wall                                |               454 ms |           458 ms |

Observed Home is about 46.7% lower, and discovery wall about 73.8% lower.
These are **two single debug observations**, with uncontrolled caches and
concurrent workspace evolution (including the other task's watcher refactor);
they are not an isolated causal estimate, p95, OS-cold, release-build result or
real hundreds-of-GB benchmark. The controlled same-binary branch benchmark
supports its independent 53.76% gain. Final summed classification work is
6,132 ms, greater than wall time because workers overlap; never add it to wall.
Restart page-clock shell observations 525/287/255 ms exclude process launch.

Tested binary: `src-tauri/target/debug/emmm-e2e.exe`, 92,597,248 bytes,
SHA-256 `8dd41297878d7a6d132bd7993fcf6ce6dd422e297fa05d19a005cab3555648e4`.
No installed app replacement. Read-only SQLite checks confirm the final private
profile contains only owned startup-fixture paths; all final fixture roots are
gone. The earlier private profile archive remains intact. Eleven validated
production source hashes and the tested binary digest still match after testing.
Evidence: `native-final-build.json`, `native-final-isolation.json`,
`native-final-stage.log` and `native-final-picker/native-progress.json` under
`.audit-output/indexing-efficiency-20261010/`.

## Remaining measured-investigation candidates

- KeyViewer duplicate migration traverses include trees inside the global revision
  and activation commit locks. Disabled-KeyViewer mode also scans for old owned
  entrypoints. These preserve migration/ownership safety and were not removed.
  Next step: measure commit-lock hold on owned fixtures, then design a candidate
  pre-scan plus identity/content and authority revalidation with race tests.
- Full runtime harvest awaits one blocking task per mod. Compare bounded widths
  1/2/4 with exact output/cache/cancellation parity before altering the flow.
- Onboarding shares its existing eight slots across roots/child branches.
  Normal full/scoped sync still classifies serially within a root, with different
  census/terminal/error semantics; benchmark that path before sharing a helper.
  Foreground priority holds its existing gate through a joined root batch;
  per-chunk yielding requires separate contention/cancellation proof.

These are source-evidenced risks/candidates, not newly verified defects or a claim
that their broader refactors are safe. Retain current guards until validated.

## Documentation checks

[SQLx QueryBuilder](https://docs.rs/sqlx/latest/sqlx/struct.QueryBuilder.html)
documents bound batched VALUES and database parameter limits.
[SQLite FAQ](https://www.sqlite.org/faq.html#q19) explains transaction amortization;
the existing single transaction is retained, including rollback and durability.
[Tokio spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
recommends bounding compute concurrency and explains cancellation limitations;
no additional unbounded workers were introduced.
