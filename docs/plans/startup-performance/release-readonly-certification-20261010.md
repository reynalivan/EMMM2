# Release GIMI read-only certification

User requested certification on the real GIMI library and confirmed closing the
installed app. Existing requirements remain: no library writes, renames or deletion;
restore the isolated audit configuration after testing. User declined Windows reboot.

## Authorized execution

1. Extend the existing read-only bootstrap guard to an explicit, default-off Cargo
   feature so optimized release builds retain identifier, root, empty-journal,
   disabled-hotkey and IPC protections. Ordinary release behavior stays unchanged.
2. Add a release flavor to the existing native harness; retain fresh build/hash,
   profile identity and owned process cleanup. Use current Tauri devtools support.
3. Calibrate on owned temporary fixtures with full before/after digests and denied
   mutation IPC before binding the real root.
4. Capture complete SHA256/metadata manifests of `E:\Data\XXMI Launcher\GIMI`,
   outside that root, with bounded readers. Refuse links, overlapping output,
   installed/other audit processes, incomplete baseline or nonempty journals.
5. Measure the first empty isolated index plus process-cold native launches and
   subsequent restarts. Record visible shell, Core Ready, optional completion,
   stage wall time, actual row/folder counts, throughput, build digest and hardware.
6. Recheck every file digest/metadata and late metadata stability. Restore only
   isolated audit configuration, verify zero real-root bindings and own process
   shutdown, retain evidence. Never repair or roll back real library files.

## Measurement contract

- Normal optimized release code is used, with an explicit read-only audit feature
  and isolated app identifier. Runtime file publication/reload/cleanup is excluded
  to honor the immutable-library requirement. This certifies read-only startup,
  disk census/classification/projection and Home reads, not production publication.
- Indexing throughput is mods/folders per second. Total asset GB divided by index
  time is not storage throughput: routine indexing does not read all payloads.
- Release timing observes the initial native page without an extra navigation
  that would remount React startup. Wall observations still include WebDriver
  overhead; backend stage timings remain separate. Record wall-clock boundaries
  and process start times to correlate workload observations.
- Full baseline checksumming warms the OS cache. The current process is not admin,
  RAMMap is not available, and the user declined reboot. No global cache eviction,
  software installation or privilege escalation is performed automatically.
- Report process-cold (new process) and OS-cache-uncontrolled measurements
  separately. Strict storage/OS-cache-cold certification cannot be asserted under
  these conditions. No p95 claim from three samples, or arbitrary unstated SLA.
- Builds, checksum passes and other native runs stay outside measurement windows.
- User chose to keep GenshinImpact running. Record ambient CPU load, game and
  audit working sets; results describe that workload, not an idle machine.
- Fixture calibration is a functional guard/integrity check and may overlap the
  real baseline checksum. Its collected timings are not certification evidence.
- Manifest metadata comes from read-only Windows handles (`WindowsHandleV2`),
  including the root and filesystem identity. NTFS enumeration attributes can
  be stale: a fixture-only reproduction changed apparent directory timestamps
  through metadata reads alone. Keep old failed evidence and require a fresh
  full Before baseline after this correction; do not normalize protected files.

No production performance refactor, dependency, schema migration, installation,
commit, push or change to the installed app is part of this certification.
