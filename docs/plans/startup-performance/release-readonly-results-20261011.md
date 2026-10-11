# GIMI release read-only certification

Status: optimized Rust backend and real-library integrity checks PASS; historical UI/end-to-end observations used React development and do not certify a production frontend. Scope is process-cold startup with uncontrolled OS cache and runtime publication excluded.

## Frontend mode correction

The October 11 native UI follow-up found that WDIO sets `NODE_ENV=test` before
`onPrepare`. Vite keeps that inherited value even when its mode is production.
The real GIMI build log (`real-native-release.log`) contains the same
`vendor-core-B6m-b1L8.js` development bundle (362.76 kB) whose `jsxDEV` calls were
observed in the follow-up CPU profile. The ordinary application build correctly
uses React production; the harness now explicitly builds release children with
`NODE_ENV=production`.

Retain the timings below as historical observations of that exact executable.
The optimized Rust stage timings and complete library checksum evidence remain
valid, but the UI/frame/end-to-end values are not production-frontend
certification. The follow-up validates a production frontend on an owned UI
fixture; it does not replace this historical real-library run with a new GIMI
measurement. See [the follow-up report](scan-reuse-census-ui-20261011.md).

## Build and scope

- Fresh optimized release, `readonly-audit` + Tauri custom protocol, private identifier `com.reynalivan.emmm.e2e.readonly-audit`.
- Executable SHA256: `e6867fc204db42df6a183dc7668e7becff121af5f8ee9f72d98e64528d4f6e24`.
- 1,526 selected source/build/test inputs were unchanged through final calibration and real native run; preserved executable and snapshots are under `.audit-output/gimi-release-certification-20261010/`.
- Runtime artifact generation/publication/reload/cleanup is excluded to preserve the real library. This certifies census, classification, database projection, core readiness and Home reads, not full production runtime publication.
- Windows was not restarted; cache was not evicted. Full checksumming warms cache. Results are process-cold with uncontrolled OS cache, not storage-cold certification.
- User originally chose game-running measurement; Genshin subsequently disappeared. The preference clarification was unanswered and the run used the stated current-state assumption. No Genshin process appeared in the measured samples; no game process was closed by this audit.

## Dataset and machine

Protected root: `E:\Data\XXMI Launcher\GIMI`. 375,100 files, 30,317 subdirectories plus the root, 632,025,086,331 bytes; no filesystem links. Mods contains 355,786 files / 30,299 subdirectories, 30,673 INIs / 184,464,485 bytes and 3,193 `info.json` files / 172,171 bytes. The resulting index has 148 objects and 4,243 mods on every launch.

Intel i7-13700K, 16 cores / 24 logical processors; about 32 GiB RAM; drive E is NVMe KYO X70 2TB. Timing windows contained no compiler, checksum pass or other native harness. Ambient load remained measurable: system CPU mean 40–48%, with sampled peaks up to 91%. Main Rust process peak working set was about 87–91 MiB; WebView2 children are not included in that process-only number.

## Native observations

Release observes the initial native page without an extra navigation/remount. Wall times include WebDriver/session transport, audit reads and readiness polling. Frame observations begin after IPC bridge attachment. Optional settlement is an observation of Home's loading state, not runtime publication completion.

| Launch               | Home visible | Core Ready | Optional Home settled | Backend reconcile |
| -------------------- | -----------: | ---------: | --------------------: | ----------------: |
| Empty isolated index |      1.926 s |    9.103 s |              11.023 s |           6.848 s |
| Restart 1            |      1.070 s |    2.677 s |               2.861 s |           1.507 s |
| Restart 2            |      1.029 s |    2.615 s |               2.780 s |           1.490 s |
| Restart 3            |      1.086 s |    2.735 s |               2.895 s |           1.498 s |

Three-restart medians: Home 1.070 s, Core Ready 2.677 s, optional settlement 2.861 s. Distinct native PIDs: 32824, 58248, 80424, 60436. Process-start-correlated observed Core Ready was 8.418 s initially and 2.505–2.595 s on restarts; these remain observed upper bounds, not exact first-paint/readiness timestamps. No p95 is inferred.

| Backend phase          | Empty index | Restart 1 | Restart 2 | Restart 3 |
| ---------------------- | ----------: | --------: | --------: | --------: |
| Census                 |    1,021 ms |  1,049 ms |  1,074 ms |  1,048 ms |
| Classification         |    4,517 ms |    221 ms |    192 ms |    208 ms |
| Database projection    |    1,308 ms |    235 ms |    222 ms |    241 ms |
| Total logged reconcile |    6,848 ms |  1,507 ms |  1,490 ms |  1,498 ms |

Backend throughput: initial 619.6 mods/s; restart median 2,832.4 mods/s, range 2,815.5–2,847.7. End-to-end observed Core throughput: initial 466.1 mods/s; restart median 1,584.7 mods/s. Census observed 30,290 directories at about 28,203–29,667 directories/s. Classified visits were 5,050, not an indexed-mod progress total. Asset GB divided by these times is not I/O throughput.

## Residual performance limits

- Initial classification is the largest recorded first-index phase, about 66% of logged reconcile time. Repeat startups have existing DB hints and warmed filesystem state, so their much smaller classification time cannot be attributed solely to a code improvement.
- Census remains about 70% of restart reconcile time. A full scan after process restart supplies fresh filesystem authority; this run does not justify replacing that proof with a persisted ready flag or adding a new cache/service.
- Post-bridge UI still produced 2–5 long tasks per launch, up to 162 ms, and frame gaps up to 150.2 ms. This does not certify perfect frame smoothness. A focused native frontend profile would be the next small investigation if visible startup stutter remains.
- No additional production performance refactor was made from these measurements. The evidence supports the existing bounded flow; remaining work should be driven by a concrete latency/smoothness target.

## Safety and validation

Every launch used one watcher registration and one StartupBoot baseline, with no additional GameSwitched baseline. The release guard rejected mutation IPC directed at an owned output file; no mutation request targeted GIMI. Audit hotkeys were disabled. Native processes/driver were stopped, and active private configuration was restored with zero real-root bindings before the final checksum.

Final configuration evidence: `real-handle-v2/config-restored.json` reports `restored: true` and zero real-library bindings. No EMMM, tauri-driver, msedgedriver or Genshin process was present in the final process inventory. The preserved executable hash and all 1,526 selected inputs were reverified after the integrity checks.

V2 Before, After and Verify share the exact digest `F88F86A5AD7EBE814ADBFC224328FC294510E18EC22A8C71CBF03D66A976C922`. After hashed all 375,100 files in 964.618 s and reports `unchanged: true`; the subsequent 39.2 s metadata/structure verification reports `stableSinceChecksum: true`. Both report 30,318 directories including the root, 632,025,086,331 bytes and zero links. File contents, recorded metadata, identities and structure matched; no protected data was repaired or restored.

Guard tests: 7 release-feature cases passed with normal optimization. Ordinary release cfg cases passed 7/7 with command-only package optimization lowered to 0 after two opt3 test-library compiler OOMs; benchmark executables retained normal opt3. Scoped formatting, E2E typecheck and lint passed. Strict all-target Clippy with readonly-audit passed; a transient unused wrapper from concurrent work was resolved by its owning cleanup work. Release compiler still emits existing dead-code warnings for test/DAL fields; strict release-profile Clippy was not certified.

Earlier native fixture passes were retained but not accepted as current-source evidence after 4 then 18 source files drifted. One additional attempt was rejected before app launch because another harness owned port 4444. Final calibration passed against unchanged 1,526 inputs and exact fixture SHA/times/identity before real binding.

## NTFS manifest correction

The initial enumeration-based fixture manifest reported 66 directory write-time differences, all belonging to fixture creation, while file paths, counts, sizes and SHA remained identical. Metadata-only handle reads reproduced 65 apparent differences without launching the app. The corrected collector queries existing entries through read-only Windows handles, records root/identity, and uses the hashing stream's handle for file metadata. Fresh fixture Before/read-only observations/After/Verify and final native calibration matched exactly. This follows [Microsoft's NTFS enumeration guidance](https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-findfirstfilea) and [zero-access OPEN_EXISTING semantics](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew).

Original and corrected complete GIMI checksum baselines matched the size and SHA of all 375,100 files. Old failed metadata evidence is preserved; protected files/timestamps were never normalized or restored. Final V2 After/Verify passed. This remains a sequential observation, not an atomic filesystem snapshot or a write-prevention proof against other applications.

No installation, Windows reboot, cache eviction, worktree, commit, push, production configuration change or real-library repair/deletion was performed.
