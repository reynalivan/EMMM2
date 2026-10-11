# Real GIMI native startup audit

Three actual native Windows process restarts passed against
`E:\Data\XXMI Launcher\GIMI` using the rebuilt debug application, tauri-driver
and its native WebView2 window. No browser demo was used. Home appears promptly;
core recovery and optional keybindings still have measurable costs.

## Dataset and protection

The protected root contains 375,099 files, 30,317 directories and no filesystem
links: 632,024,978,228 bytes (632.0 GB / 588.6 GiB). Mods contains 355,785 files
and 30,299 directories, including 30,673 INI files totaling 184,464,485 bytes.
The native index contains 148 objects and 4,243 mods.

The installed application was observed closed before testing; it was not closed
by automation. The audit uses its own identifier, executable, app-data directory
and SQLite snapshot. Hotkeys are disabled, mutation IPC is rejected and the
entire KeyViewer publication/reload/cleanup pipeline is bypassed. Real watcher,
census, classification, projection and Home INI reads remain active. No real
library write, rename, deletion or configuration change is part of the test.

The audit configuration has been restored: `config-restored.json` records
`restored: true` and zero real-library bindings. The ordinary E2E configuration
and installed application were not replaced.

Whole-root SHA256 verification passed for every file. Before/after manifests
match exactly, including relative paths, kinds, sizes, creation/write timestamps
and attributes. Their shared manifest SHA256 is
`0D326FEB9C2C9D31BDEE6C37ED420EA03651BDC39EC0955941C20DE318041849`.
The before pass took 1,055.126 seconds; the after pass took 939.743 seconds.
Fresh metadata verification after the long checksum pass also matched the same
manifest in 21.148 seconds (`stableSinceChecksum: true`). This catches late
structural/metadata changes after the initial checksum enumeration. Access-time
metadata is not part of this manifest; content SHA256 covers every regular file.

## Restart measurements

Wall-clock measurements start before creating the next native driver session.
Home means its visible heading was observed. Core means selected-game activation
became Ready. Optional means the Home keybinding skeleton settled. These are
observation boundaries, not exact first-paint timings.

| Restart | Home visible | Core Ready | Keybindings settled | Native baseline |
| ------- | -----------: | ---------: | ------------------: | --------------: |
| 1       |     1,274 ms |   4,933 ms |           11,965 ms |        3,427 ms |
| 2       |       903 ms |   4,444 ms |            6,512 ms |        3,199 ms |
| 3       |     1,011 ms |   4,781 ms |            9,701 ms |        3,480 ms |

Each restart registered one watcher, ran one full StartupBoot baseline and ran
zero full GameSwitched baselines. The database loaded 148 objects and 4,243 mods.
The initial empty isolated index took 10,042 ms: census 1,834 ms, classification
4,584 ms and projection 3,622 ms. This is an empty-database measurement, not an
OS-cold startup.

| Restart |   Census | Classification | Projection |
| ------- | -------: | -------------: | ---------: |
| 1       | 1,904 ms |         549 ms |     971 ms |
| 2       | 1,935 ms |         447 ms |     813 ms |
| 3       | 2,064 ms |         565 ms |     847 ms |

Native foreground frame samples numbered 710, 356 and 557. Frame-gap p95 was
15.1 ms in each run; maxima were 103.3, 104.7 and 118.2 ms. Each run recorded
three main-thread long tasks: 77/87/82 ms, 76/78/94 ms and 82/82/128 ms. Most
sampled frames were smooth, with intermittent visible hitches still possible.
Sampling begins after navigation and IPC availability, so it omits initial paint.

## Remaining costs and recommended order

1. Dashboard keybinding discovery omits the existing effective-enable rule.
   Of 1,125 status-enabled mods, 838 have a DISABLED ancestor folder. Existing
   runtime/collection predicates exclude these, but dashboard harvest only
   skips disabled descendants. Reuse the existing path-component rule before
   harvesting: candidate roots become 287, or 74.5% fewer. Keep each mod's own
   status unchanged. This is a verified correctness/work-reduction opportunity;
   its wall-time benefit has not yet been measured.
   Evidence: `dashboard/adapters/sqlite/dashboard/mod.rs::fetch_enabled_mods_for_keybindings`
   filters only own status; `system/application/app/dashboard.rs::harvest_active_keybindings_for_mods`
   harvests all rows; `library/application/ini/document.rs::collect_ini_files`
   skips descendants. Existing `mods/listing.rs::is_effectively_enabled_path`,
   collection `is_mod_effectively_active` and post-apply `stored_mod_path_is_enabled`
   already define ancestor exclusion. Counts come from the archived isolated
   SQLite database. The game/loader was not launched; successful INI reads per
   root were not separately counted.
2. Census consumes roughly 58–60% of the reopened native baseline. It walks
   roughly 30,300 directories and enumerates asset names serially. Benchmark
   bounded parallel traversal of independent roots on the existing Rayon pool,
   preserving ordering, error handling and watcher proof. Classification already
   uses parallel roots and costs substantially less.
3. Home keybindings settle 2.1–7.0 seconds after Core Ready. The query selects
   1,125 mod rows with status enabled; discovery recursively enumerates assets
   for INI files and parses full editor documents. Profile enumeration, reading,
   parsing and response/rendering separately before choosing bounded concurrency
   or a lighter parser. Preserve keybinding semantics and deterministic output.
4. Projection includes metadata reads for 4,243 mods as well as SQLite work and
   read-model refresh. Its 0.81–0.97 seconds cannot be attributed solely to SQL.
   Measure unchanged object updates and read-model rebuilds before adding guards.

These findings favor focused improvements over a new startup architecture or
persistent cache. The cost tracks file/directory/INI counts more directly than
total asset bytes; routine startup does not hash or sum all asset contents.

## Limits and anomalies

The application is a debug build and OS cache is uncontrolled, including warming
from the pre-test checksum pass. Three starts do not establish startup p95 or
storage-independent performance. KeyViewer publication and reload are excluded
to honor the read-only requirement; their normal production cost is unmeasured.

During the third restart the driver output also recorded three setup rejections
with the audit's identifier/root guard message. Three valid private-identifier
measurements passed, but the reason for the extra rejected startup attempts is
unresolved. Rejected setup fails before config/recovery; do not label this noise
as expected or claim an entirely clean driver run.

Raw measurements, full manifests and backups are retained under ignored
`.audit-output/gimi-readonly-20261010/`; native logs are under the private audit
identifier's Local app-data directory.

## Validation

- Real native E2E: one passing test with three process restarts; owned teardown
  and offline configuration restoration completed.
- Real-library integrity: all 375,099 file SHA256 digests and complete before/after
  manifests match; `after-summary.json` records `unchanged: true` and final
  `verify-summary.json` records `stableSinceChecksum: true`.
- Isolated calibration: unchanged owned fixtures pass full before/after SHA256
  and final metadata verification; same-length content changes with restored
  timestamps are detected by SHA256.
- Read-only guard: six focused tests pass. Post-apply suite: 50 pass, two ignored.
  Native compilation, cargo check and strict library Clippy pass.
- Audit-scoped E2E TypeScript passes. Whole E2E TypeScript has an unrelated
  concurrent Window-cast error in `client-workspace-smoke.e2e.ts`; it was preserved.
- Focused safety review found no remaining actionable blocker before real testing.
- Focused E2E ESLint passes without warnings; nine driver lifecycle tests pass.
