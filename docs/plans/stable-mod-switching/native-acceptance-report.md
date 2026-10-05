# Native switching acceptance — 2026-10-05 / 2026-10-06

Status: approved namespace/echo/journal fixes completed on 2026-10-06; latest native bulk and stability suites passed **9/9**. Earlier failed/interrupted runs remain documented. Functional disk-first/fairness/convergence closure passed on the available fixtures; repeated end-to-end latency targets and real-game/storage gates remain unverified. This is not release certification.

The previous complete stability run, including the stitched-echo fix, passed seven scenarios at 233.3267 ms trusted click-to-observed-disk and 11.7 ms next-frame proxy. The latest approved-closure run below supersedes it and includes successful 10k bulk and post-bulk opposite intents. Synthetic outcomes include superseded/no-op receipts, not a physical rename per sample. No push, commit or installer was performed.

## Environment and scope

- Repository HEAD `218c31d`, dirty approved working tree; Tauri debug/no-bundle with embedded Vite frontend, isolated identifier `com.reynalivan.emmm.e2e`.
- Windows 11 Pro 10.0.26200; WebView2/EdgeDriver 154.0.4258.53; Intel i7-13700K; RAM 34,084,311,040 bytes.
- Fixture/benchmark temp storage: C:, NTFS, NVMe PNY CS2241 500GB SSD. Repository: E:, NTFS, NVMe KYO X70 2TB.
- Host display query reports 74 Hz; actual WebView presentation cadence was not traced. DOM/rAF is a next-frame proxy, not pixel paint.
- pnpm 11.19.0, Node 24.18.0. Fake game executables are never launched; global hotkeys are disabled in owned fixtures.
- Storage benchmark: classifier-valid flat/nested corpus sizes 100/1,000/10,000, three repeats, eight warmups, 100 timed samples/repeat. Fresh isolated databases; exact DB bytes were not recorded for this benchmark.
- Native WebView bulk: one 10,019-mod corpus, depth two, one 10,000-sibling object plus three healthy leaves and 16 overlapping-bulk targets. Observed pre-fix DB: 10,940,416-byte main DB plus 12,162,272-byte WAL; this is a point observation, not peak storage.
- Current fixture d3dx.ini is a placeholder without effective Include configuration: optional runtime publication reports an explicit error. These runs do not prove KeyViewer artifacts or successful in-game runtime loading.
- Private native journal retains interrupted-run recovery evidence rather than being erased between samples. Native timings are history-loaded, unlike the fresh isolated storage benchmark; they are not a controlled empty-history comparison.

## Safety preflight and production corrections

The harness now rejects occupied driver ports without killing their owners, proves a fresh successful build by SHA-256 before each worker, and checks the running app identifier before reset. Cleanup targets only the owned process tree and registered temporary roots with matching ownership marker. Fixture deletion is deferred until the owned native process stops; completed stability sessions confirmed deletion of five owned roots.

Safety audit found that the old E2E app identifier did not isolate Windows AI credentials. Production retains its existing credential service; non-production identifiers use a separate service. The earliest stale-binary run had already invoked reset: an existing user AI key may have been deleted and may require re-entry. No key was read or logged. No user Mods root was reset or deleted.

Native acceptance reproduced two actual defects:

1. Canonical Windows journal endpoints (`\\\\?\\C:\\...`) did not match an ordinary configured root. Physical rename committed, but projection retried with “lineage is outside its Mods root”; checkpoint stayed zero. Regression was RED, then GREEN after aligning the exact physical namespace.
2. The same raw comparison survived in single/bulk trust predicates and changed-root extraction. Healthy known leaf toggles therefore missed the fast path and ran whole-library preflight. The canonical-guard leaf regression was RED. The fix reuses one lexical physical-path helper across trust predicates, projection and watcher matching.

Physical identity checks/scanner joins retain verbatim names; historical old endpoints are not canonicalized through the filesystem. Native Unix backslashes remain unchanged. Trusted batches align root, changed paths and rename events together. Folder whitespace is not trimmed. No additional coordinator, mutation writer, lock hierarchy, dependency, schema or public IPC command was added.

## Storage-only baseline

Command: `cargo test --lib native_storage_ack_benchmark -- --ignored --nocapture`, run from src-tauri. One ignored benchmark passed, 118.09 seconds execution. This measures admission/operation lease/safety/rename/native proof/durable DiskCommitted in the lower-level storage path, excluding IPC, UI, DB projection and runtime.

Raw samples and stage arrays: [native-storage-ack-raw.log](native-storage-ack-raw.log). The benchmark names its first sample “cold”; cache eviction was not controlled, so that sample is not cold-cache evidence. 1,800 warm timed samples total. The following table retains all repeats, including outliers (milliseconds):

| Corpus | Layout | Repeat | p50     | p95     | p99     | Max     |
| ------ | ------ | ------ | ------- | ------- | ------- | ------- |
| 100    | flat   | 1/3    | 16.635  | 18.452  | 21.638  | 24.704  |
| 100    | flat   | 2/3    | 22.950  | 26.986  | 30.722  | 30.953  |
| 100    | flat   | 3/3    | 23.387  | 29.429  | 30.474  | 30.556  |
| 100    | nested | 1/3    | 23.533  | 28.125  | 29.456  | 30.449  |
| 100    | nested | 2/3    | 22.996  | 28.724  | 33.374  | 35.520  |
| 100    | nested | 3/3    | 22.989  | 27.973  | 29.590  | 30.013  |
| 1000   | flat   | 1/3    | 130.768 | 222.074 | 336.213 | 477.053 |
| 1000   | flat   | 2/3    | 23.731  | 28.332  | 30.901  | 33.690  |
| 1000   | flat   | 3/3    | 24.884  | 28.562  | 31.088  | 32.011  |
| 1000   | nested | 1/3    | 24.280  | 29.296  | 30.734  | 38.692  |
| 1000   | nested | 2/3    | 26.797  | 30.298  | 32.216  | 32.349  |
| 1000   | nested | 3/3    | 23.555  | 28.271  | 29.585  | 36.286  |
| 10000  | flat   | 1/3    | 32.395  | 37.158  | 37.697  | 40.231  |
| 10000  | flat   | 2/3    | 26.819  | 31.440  | 32.740  | 34.542  |
| 10000  | flat   | 3/3    | 26.644  | 32.467  | 33.749  | 33.933  |
| 10000  | nested | 1/3    | 23.045  | 55.925  | 198.074 | 218.870 |
| 10000  | nested | 2/3    | 20.413  | 26.817  | 29.855  | 31.958  |
| 10000  | nested | 3/3    | 19.001  | 24.790  | 26.122  | 27.795  |

The 1,000-flat first repeat exceeded p95 100 ms and p99 300 ms (222.074 / 336.213 ms). The 10,000-nested first sample was 305.368 ms; warm p99 reached 198.074 ms. No measured cause was established for these outliers. Warm lease-wait p95 was approximately 0.017–0.027 ms; this does not establish native bulk fairness or UI latency.

## Native runs and regression evidence

- First attempt: frontend helper type errors prevented build, but the old WDIO lifecycle still launched a stale private binary. Missing get_workspace_structure failed setup. Not acceptance. Build-proof gating prevents this reuse now.
- Second attempt: aborted the owned build tree before launch to address credential namespace isolation.
- Third attempt: native stability failed on hidden checkbox selection and the real projection namespace error. Aborted before measuring bulk.
- Fourth attempt: rename/watcher and actual collision scenarios passed; 100/1,000 synthetic outcomes eventually completed, but tests failed due to capture/timing/selector problems. Current journals showed completed projection, not a pending storage mutation.
- Fifth attempt: trusted first UI toggle plus explorer/object/current-collection/historical-preset assertions passed. Next-frame proxy 21.3 ms; WebDriver click-to-observed-disk 190.358 ms (includes driver round-trips/polling, not pure disk acknowledgement). 100 synthetic intents passed. Deep structured WebDriver serialization timed out for 1,000 full receipts; game option selector chose a hidden duplicate. Those harness issues were simplified: JSON string transport preserves outcomes without deep BiDi encoding, and selectors choose displayed elements.
- Fifth attempt bulk 100 before trust-predicate fix: 24,804 ms WebView round-trip; unrelated leaf 6,778.1 ms, settled before bulk; first disk observation 2,342.422 ms; 100 fulfilled paths, no failures/cancellation. Sampled JS heap peak 51,742,271 bytes, settled 45,636,319 bytes. Parent native process observation 137,580,544 bytes working set / 141,348,864 bytes lifetime peak; WebView subprocess RSS was not included. Logs showed repeated full scans of 10,019 mods, and scoped scans of all 10,000 bulk descendants. Aborted the owned process tree to fix the reproduced fast-path defect; its temporary bulk payload was retained, not broadly deleted.

Sixth attempt after trust-predicate correction: bulk 100 took 29,686.5 ms, unrelated leaf 427.8 ms, first disk observation 338.4715 ms. Bulk 1,000 took 452,554.5 ms, unrelated leaf 8,140.6 ms, first observation 9,458.8086 ms; all 1,000 paths fulfilled without cancellation. These are measured failures of responsiveness, not acceptance passes. The owned process tree was stopped before 10,000 to investigate repeated scans. Pending projection advanced with live chunks; no journal proof error was recorded. The outer bulk foreground guard defers the projection worker until bulk completion. Timing logs previously omitted request reason, so scans could not be reliably assigned to preflight versus watcher.

A third Windows namespace regression then reproduced false dropped-event evidence in watcher authority: a canonical descendant event against an ordinary root returned Full instead of Scoped. The fix aligns physical paths when recording dirty scopes, clearing them and checking dirty overlap; foreign-root events remain lost-coverage evidence. It does not relax the pending-commit checkpoint barrier or remove write-through rename durability.

Current debug/opt-in command spans now use the existing INFO log sink, with preflight/rename/durable timings and bulk fast-path/trust flags. Debug traces do not turn on telemetry transport. Reconcile timing includes the request reason. Native harness transport now uses JSON strings for both requests and receipts, eliminating deep BiDi object encoding without replacing application IPC.

Raw WebView samples, including failed attempts: [native-webview-results.json](native-webview-results.json). The JSON transport change does not make failed historical tests passed. Final rerun and static gates will be recorded below.

## Resumed findings and validation — 2026-10-06

Actual outcomes and complete captured burst arrays are in [native-webview-results-20261006.json](native-webview-results-20261006.json). Historical failed/truncated captures are explicitly identified rather than reconstructed.

Final synthetic receipt round-trips (nearest-rank percentiles, milliseconds): 100 intents p50/p95/p99 **19.5 / 253.0 / 253.5**, max **703.1**; 1,000 intents **307.0 / 342.7 / 343.1**, max **736.8**. Functional accounting passed, but these results do not establish the p95 100 ms/p99 300 ms end-to-end latency targets. No three-repeat native latency cell or actual paint trace was measured.

- Scoped storage readiness and echo eligibility now share the same session-bound predicate. Unrelated dirty roots no longer make an otherwise healthy bulk/workspace toggle miss echo registration. Whole-game publication and collection snapshot barriers remain strict.
- Windows watcher filtering/runtime-config matching now uses the lexical physical namespace. The observer and classifier no longer consume the same edge twice; a verified duplicate cannot falsely dirty authority, while pending/replaced/unproven events retain conservative delivery.
- A native 10k attempt exposed readiness changing while the durable plan was prepared: one untouched 32-path chunk failed before rename. Both switch callers now repair once under their original game/operation guards and revalidate their unchanged source identities, bindings, epoch and admission. Failed or still-unproven repair aborts the unapplied journal; there is no blind rename retry or replacement rebind. The deterministic regression was RED, then GREEN.
- Rename-hint filtering formerly repeated normalization for every row × hint. A request-local canonical-component coverage index preserves existing filtering semantics. For the earlier 1k catch-up, the DB-labelled phase measured about 44 seconds; resumed 100-hint catch-up measured 0.83–0.91 seconds, versus earlier 100-hint measurements around 5.4 seconds. These are different hint-count cells, not a claim that a new complete 1k catch-up was measured below one second.
- A split `From` followed by stitched `Both` could be accepted without recording its missing half, then falsely require repair at expiry. The observer now completes that evidence only after the same native/session/lineage proof. Its expiry regression was RED, then GREEN; consume-once and replacement rejection remain intact.
- The A-B-A failure was an E2E option lookup before dropdown animation completed. Bounded fresh visibility polling and dropdown-scoped options fixed it; no production readiness bypass was added.

One run after scoped fixes measured bulk 100 at **1,363.6 ms** and 1,000 at **7,789.3 ms**, with zero path failures and unrelated leaf receipts **133.7 / 140.5 ms** before bulk completion. Earlier measurements were 31,251.5 / 452,554.5 ms. These are individual runs, not repeated controlled latency distributions. That same run failed its 10k preparation gate and was not an acceptance pass.

After the preparation repair, bulk 100 passed at 1,772.6 ms, but the 1k workload again incurred repeated Full preflight (~2.3 seconds) and scoped WatcherBatch (~3.1 seconds) per chunk. The owned run was stopped during 1k; there is no completed 1k/10k result from it. A second bounded diagnostic run confirmed `Modify(Any)` observations during this behavior and was stopped at the same reproduced bottleneck. This event can represent timestamps, attributes or security changes; identity and batch coincidence do not prove it harmless. No broad suppression or dirty-path exemption was implemented. A narrower namespace/ancestor proof would require a separately reviewed admission-policy change; user direction is requested before that expansion.

Debug diagnostics retain at most 16 unproven observations per watcher session (kind, path count/shape and coverage-loss flag; no paths). They do not enable telemetry transport. Final local gates: Rust **1,415 passed / 14 ignored**; frontend **1,141 passed / 1 skipped** across 207 files; Clippy all-targets `-D warnings`, rustfmt, main/E2E TypeScript, full ESLint, architecture lint and diff-check passed. Fresh debug Tauri/Vite builds passed in the native runs. Focused read-only review found no remaining actionable correctness issue in these changes. Optional runtime fixtures remain intentionally invalid and do not certify KeyViewer publication.

## Approved namespace-proof closure — 2026-10-06

The user approved the narrower admission-policy change after the previous session. Direct same-parent prefix switches now capture request-local native identities for every parent through Mods root before waiting. Native source ownership remains in existing intent admission. The original proof is validated under the final durable lease and before every rename attempt; replacement parents, junction rebinding, unknown identity, root/session replacement and actual watcher/suppressor coverage gaps still reject. Ordinary directory metadata dirt remains recorded, but no longer forces projection scans before a proven physical prefix action. Strict object/ancestor/collection snapshot barriers are unchanged. Ordinary watcher batches now use the existing nonblocking catch-up/retry owner instead of queuing full scans between foreground chunks.

This session's baseline bulk 100 took **17,225.0 ms**, first host-observed rename **902.804 ms**, unrelated leaf **578.8 ms**. The bounded run was stopped after reproducing the old scan bottleneck; its owned interrupted fixture was retained. Namespace-only bulk 100 measured **1,412.1 ms** but the 1k case failed, so it was not an acceptance pass. A diagnostic repeat passed bulk 100/1k at **1,454.2 / 10,252.1 ms**, then failed the final eight paths while restoring 1k; 10k had not started. Completed failed-run roots were cleaned by the owned harness after process shutdown.

The diagnostic rejection showed the original watcher session, indexed root and root identity still matched, with no suppressor repair, but authority was untrusted/dropped. At the preceding chunk, ordinary metadata dirt prevented echo registration (`trusted_mutation=None`), and the commit path invalidated physical authority itself. Echo eligibility now accepts the original already-validated storage proof/session, preserving metadata dirt and avoiding that self-invalidation. Unproven coverage is not cleared or bypassed.

Separately, the installed debouncer intentionally replaces multiple renames with one original-to-final `Both` event, including `A -> A`. Exact-edge accounting could miss those intermediate edges and create false repair at expiry. An observer-only matcher now accounts for the maximal contiguous same-ID chain before trying an individual exact edge. Verified native/session/lineage proof completes it; pending chains attach the existing observer to every covered edge until full commit proof. Classifier delivery remains conservative. The 2/3/4-edge and folded-before-commit regressions were RED, then GREEN; abort/expiry release dirty repair evidence, while replaced, discontinuous and foreign chains reject. The evidence TTL was not increased.

The next run completed all 100/1k/10k disk paths and their checkpoints, with round-trips **1,529.3 / 17,453.0 / 150,279.7 ms** and zero path failures. However, the 10k unrelated leaf waited **2,665.1 ms**, including **2,386.408 ms** native game-lease wait, and the subsequent opposite 16-path batch exceeded the driver's script-call deadline. It was not a full acceptance pass. Foreground priority was registered only after expensive source identity resolution/admission, while projection terminal settlement persisted a full journal snapshot per operation under the lease. Both are now corrected: foreground priority starts before resolution, and exact projection completion validates the whole batch, then performs one existing atomic durable snapshot before sidecar cleanup. Invalid/repair IDs and persistence errors leave the batch pending; no DiskCommitted durability is downgraded. The journal regression was RED on partial completion, then GREEN, and covers one-write completion, repair rejection, reopen and stale sidecars.

Raw current-session samples: [native-webview-results-namespace-20261006.json](native-webview-results-namespace-20261006.json). Individual history-loaded timing runs are not controlled repeated latency distributions.

### Final available-scope native result

Fresh debug Tauri/Vite build and `pnpm test:e2e --spec "tests/e2e/specs/switching-{bulk,stability}.e2e.ts"` passed both specs: **2 bulk + 7 stability scenarios**. Bulk 100/1k/10k and their automatic checkpoints completed with zero path failures/cancellation. First-game onboarding to trusted toggle, disabled-ancestor/current collection versus saved membership, 100/1k synthetic last-wins accounting, repeated external renames, A-B-A and real same-parent collisions all passed. Six registered fixture roots were removed after their owned processes stopped.

| Corpus | Bulk WebView round-trip | First host-observed disk change | Unrelated leaf receipt |
| ------ | ----------------------- | ------------------------------- | ---------------------- |
| 100    | 1,669.6 ms              | 490.719 ms                      | 305.0 ms               |
| 1,000  | 11,564.2 ms             | 483.699 ms                      | 293.9 ms               |
| 10,000 | 165,553.9 ms            | 1,834.923 ms                    | 297.2 ms               |

The leaf settled before bulk in every cell. The 10k leaf improved from the prior **2,665.1 ms** case, but the entire 10k operation remains a large durable workload, not an instant rename of all folders. Post-10k opposite intents both fulfilled at **15.3 / 14.9 ms**, with all 16 targets matching the latest desired enabled state; those receipts may be superseded/no-op. Journal settlement measured **143 ms / 33 operations** and **336 ms / 314 operations**, replacing per-operation snapshot loops.

Trusted first toggle observed disk at **215.522 ms**; next-frame proxy **10.8 ms**, not measured pixel paint. Synthetic 100 intents fulfilled 100/100, p50/p95/p99 **25.0 / 27.2 / 27.4 ms**, max **310.6 ms**. Synthetic 1k fulfilled 1k/1k, **192.9 / 263.1 / 264.5 ms**, max **379.5 ms**. These do not certify p95 100 ms/p99 300 ms end-to-end targets or three-repeat stability.

Sampled 10k WebView JS heap peaked at **50,477,996 bytes**, settled **35,386,297 bytes**. A mid-bulk native-parent observation measured **158,216,192 bytes** working set and **162,459,648 bytes** lifetime peak at that observation; it excludes WebView subprocesses and is not the final run's full process-tree peak or a retention plateau.

Final local gates: Rust **1,425 passed / 14 ignored**; frontend **1,141 passed / 1 skipped** across 207 files; fresh Tauri/Vite/main TypeScript build, E2E TypeScript, full ESLint, architecture lint, rustfmt, Clippy all-targets `-D warnings`, formatting and diff-check passed. Focused read-only review found no remaining actionable correctness finding. Frontend sources did not change after their full suite. No dependencies, schema, journal format, public IPC, commit, push or installer were added.

## Pending/deferred acceptance

- Actual rendered paint/frame trace, native OS Explorer window, real 3DMigoto overlay/reload and controlled no-key panel/catalog fixture.
- Complete ancestor/current-collection/runtime/KeyViewer snapshot matrix, source replacement and all grid/preview/context-menu surfaces in native UI.
- Consumed 4-second projector / 10-second runtime hold-and-release injection was not available in this native harness; no production delay command or setting was added.
- Repeated native idle/mixed 100-sample p95/p99 cells, diagnostics off/on overhead, long-task trace, full process-tree RSS/retention plateau and repeated bulk cycles.
- Controlled cold cache, network storage, ACL/persistent sharing lock and real power-loss matrix; require a specifically authorized environment. Existing Rust subprocess recovery and real watcher tests are distinct evidence, not substitutes for those scenarios.

Do not mark Section 9 fully green from storage benchmarks or fixture passes alone. No push, commit, release or installer is part of this execution.
