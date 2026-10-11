# Ignored Rust tests: disposition and executed checks

2026-10-10. All **27 originally reported ignored library tests were handled**:
**24 direct opt-in executions passed**, and **three crash workers were verified
through their passing parent harnesses**. There were zero failures in these runs.
Ignore attributes were not removed merely to reduce a counter.

The frozen debug test executable contains **29 ignored tests** because the
concurrent performance task added two more manual benchmarks. Their baseline
measurements passed in the owning task; validation of its subsequent optimizer
implementation remains with that task, not certified by this frozen snapshot.

## Classification

| Category                                | Original count | Why excluded from the default suite                                 | This session                                           |
| --------------------------------------- | -------------: | ------------------------------------------------------------------- | ------------------------------------------------------ |
| Synthetic/manual performance benchmarks |             19 | Large fixture creation, repeated I/O or 100k-entry measurements     | 19/19 pass, serial                                     |
| Dataset-dependent audit benchmarks      |              4 | Require explicit authorized Mods root and/or DB clone               | 4/4 pass on owned synthetic dataset                    |
| Live GameBanana API integration         |              1 | Network, rate limits and mutable community content                  | 1/1 pass for this run                                  |
| Crash worker entry points               |              3 | Intentionally exit their process and require parent-provided inputs | 3/3 parent harnesses pass; workers invoked as children |

The three worker ignore markers are safety boundaries, not missing test coverage.
Running the entire ignored set indiscriminately could invoke a crash worker
without its fixture or prematurely terminate the test process.

## Original inventory and outcomes

All benchmark names below are unique suffixes of the fully qualified names in
the [canonical coverage record](../../../.bug-hunter/ignored-tests/coverage.json).

| Test                                                   | Outcome                                                         |
| ------------------------------------------------------ | --------------------------------------------------------------- |
| `benchmark_p2_duplicate_read_paths`                    | PASS                                                            |
| `benchmark_staging_detach_versus_recursive_cleanup`    | PASS                                                            |
| `benchmark_same_name_collision_reobserves_target_root` | PASS                                                            |
| `preview_ini_discovery_and_parse_benchmark`            | PASS                                                            |
| `native_storage_ack_benchmark`                         | PASS                                                            |
| `benchmark_bulk_journal_persistence_baseline`          | PASS                                                            |
| `benchmark_onboarding_single_root_classification`      | PASS                                                            |
| `benchmark_disk_snapshot_fixtures`                     | PASS                                                            |
| `benchmark_onboarding_session_parallelism`             | PASS                                                            |
| `benchmark_onboarding_reconcile_fixtures`              | PASS                                                            |
| `benchmark_fresh_projection_and_unchanged_sync`        | PASS                                                            |
| `benchmark_paired_mod_inserts`                         | PASS                                                            |
| `benchmark_reopened_index_backfill`                    | PASS                                                            |
| `benchmark_100k_entry_composition_cache_update`        | PASS                                                            |
| `benchmark_100k_fallback_grouping`                     | PASS                                                            |
| `benchmark_runtime_preflight_pruning`                  | PASS                                                            |
| `benchmark_noop_projection_refresh_1000_objects`       | PASS                                                            |
| `benchmark_100k_listing_candidates`                    | PASS                                                            |
| `scan_10k_folders`                                     | PASS                                                            |
| `benchmark_readonly_census_sort_keys`                  | PASS, synthetic input                                           |
| `benchmark_readonly_census_parallelism`                | PASS, synthetic input                                           |
| `benchmark_effective_keybinding_filter_readonly`       | PASS, synthetic DB/root                                         |
| `benchmark_projection_phases_isolated`                 | PASS, owned writable clone only                                 |
| `test_live_api_fetch_and_validate`                     | PASS, public API metadata only                                  |
| `collection_receipt_exit_worker`                       | Covered by `collection_receipt_survives_abrupt_exit_and_reopen` |
| `folder_create_exit_worker`                            | Covered by `folder_create_process_exit_recovers_each_boundary`  |
| `native_exit_worker`                                   | Covered by `native_process_exit_recovers_each_disk_boundary`    |

## Changes and safety

- Added `scripts/prepare-ignored-audit-fixture.mjs`: generates fresh canonical
  schema and 148 object/4,243 mod fixtures, including disabled ancestors and
  metadata/keybindings. No user library/database is read or changed.
- Projection profiling accepts a strict canonical UUID opt-in, deriving only
  `.audit-output/ignored-tests/<UUID>/projection-benchmark.db`. Legacy expected
  path, fixed row counts, link/sidecar/overlap/exact-path guards remain unchanged.
- Four profiler correctness guards pass, including malformed run IDs, alternate
  DB rejection, overlap and Windows junction rejection. The new helper's RED
  E0425 was observed before implementation, then integrated GREEN was executed.
- The serial runner uses an explicit allowlist, cleans inherited `EMMM_*`,
  derives audit inputs from owned paths, rejects root escapes before launch,
  bounds each process, and never directly runs crash-only workers.
- The library input DB SHA-256 remained unchanged. Both input and projection
  DBs retain 148 objects/4,243 mods, SQLite integrity `ok`, zero FK violations.
- Independent read-only review found no actionable issue. Node syntax, fixture
  script ESLint/Prettier, scoped Rust formatting and whitespace checks pass.

## Provenance and limits

Private executable SHA-256:
`20b9c8f678b4c20c90311a336a2f7ee344656c4a97738f1e694589f5e9393ee8`.
Build succeeded through `cargo test --lib --no-run --message-format=json`.
The private copy prevents linker locks while other tasks edit/build shared code.

This is correctness/parity evidence on a frozen debug executable, not a cold-cache,
production latency, real-library benchmark or certification of source changed
after capture. The two additional ignored benchmark names are
`benchmark_normal_sync_child_classification` and
`benchmark_keyviewer_traversal_publication_lock`; their source is owned by
“Telusuri bottleneck startup app”. No competing native harness or source override
was introduced by this task. Early compiler errors in other tasks' in-progress
tests were coordinated rather than mistaken for ignored-test failures.

No extra ignored test is added by this task. The separate ignored integration
test `startup_watcher_audit.rs` is outside the original 27-library-test request;
its input contract was inspected but it was not executed here.

## Artifacts and rerun

[Coverage, individual log paths and cleanup evidence](../../../.bug-hunter/ignored-tests/coverage.json)
are also stored under
`.audit-output/ignored-tests/a79934c4-c9e8-47f6-929c-18cb52dccc09/`.
The exact owned OS-temp fixture and four scratch directories were removed after
verification; artifacts, DBs and executable are retained. Synthetic data can be
regenerated, but the removed temp paths cannot be reused as a live dataset.

To rerun, generate a fresh fixture with
`rtk proxy node scripts/prepare-ignored-audit-fixture.mjs`, build a current library
test executable, then use `.bug-hunter/ignored-tests/capture-snapshot.cjs` and
`run-selected.cjs` with the new descriptor. Coordinate the Cargo/measurement
slot first. Never use an unfiltered `--ignored` run containing crash workers.
