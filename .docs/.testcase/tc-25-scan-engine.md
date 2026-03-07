# Test Cases: Scan Engine (Epic 25)

## A. Requirement Summary

- **Feature Goal**: Autonomously discover all mod folders within the user's`mods` directory, emitting progress updates, extracting relevant structural signals (name/INI tokens) for categorization, and discovering preview thumbnails without blocking the UI thread.
- **User Roles**: End User
- **Success Criteria**:
 - Scanning 1,000 folders completes ≤ 30s strictly on SSD.
 - Progress streams back every ≤ 2s (or per 50 items).
 - Skims intelligently past permission-denied directories gracefully completing 100% of execution.
 - Cancel command halts the walker within ≤ 1s of receipt.
 - Includes thumbnail discovery paths.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-25-scan-engine.md`

| Acceptance Criteria | Covered by TC IDs | Requirement File |
| :-------------------------------- | :---------------- | :---------------------- |
| AC-25.1.1 (Full Scan) | TC-25-01 |`req-25-scan-engine.md` |
| AC-25.1.2 (Progress Emissions) | TC-25-02 |`req-25-scan-engine.md` |
| AC-25.1.3 (Inaccessible Folder) | TC-25-03 |`req-25-scan-engine.md` |
| AC-25.1.4 (Symlink Cycle) | TC-25-04 |`req-25-scan-engine.md` |
| AC-25.2.1 (In-Flight Cancel) | TC-25-05 |`req-25-scan-engine.md` |
| AC-25.2.2 (Preserved Partial) | TC-25-06 |`req-25-scan-engine.md` |
| AC-25.2.3 (Late Cancel) | TC-25-07 |`req-25-scan-engine.md` |
| AC-25.3.1, AC-25.3.2 (Thumbnails) | TC-25-08 |`req-25-scan-engine.md` |
| AC-25.4.1, AC-25.4.2 (Signals) | TC-25-09 |`req-25-scan-engine.md` |
| AC-25.4.3 (Malformed INI) | TC-25-10 |`req-25-scan-engine.md` |

## D. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| :------- | :----------------------------------- | :------- | :------- | :--------------------------------------------- | :---------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------- | :------------------- |
| TC-25-01 | Standard Scan Completion Performance | Positive | High |`mods_path with 1,000 folders` | 1. Navigate to Dashboard or Mod Manager.<br>2. Click "Scan Now".<br>3. Measure time to completion. | Scan completes in ≤ 30s. All 1,000 folders are identified.`ScanState` returns`Completed`. | S1 | AC-25.1.1 |
| TC-25-02 | Scan Progress Emissions | Positive | High |`mods_path with 500+ folders` | 1. Open Console/DevTools.<br>2. Click "Scan Now".<br>3. Observe Frontend event payload for`scan_progress`. | Event emitted exactly every 50 files processed. Payload contains`{scanned_count, total_estimate, current_path}`. UI reflects. | S2 | AC-25.1.2 |
| TC-25-03 | Inaccessible Folder Safely Bypassed | Negative | High |`Mods/Secret` with locked Read | 1. Initiate Full Scan.<br>2. Check backend Rust logs. | Scan bypasses`Secret`, logs a`warn` entry, and finishes scanning the rest of the directory hitting`Completed` state. | S1 | AC-25.1.3 |
| TC-25-04 | Symlink Cycle Safety | Edge | Medium |`OS symlink loop LoopA -> LoopA` | 1. Initiate Full Scan. | Walker respects`follow_links = false`. Maximum recursion limits hit without hanging/crashing. Returns`Completed`. | S1 | AC-25.1.4 |
| TC-25-05 | In-Flight Cancellation | Positive | High |`Benchmark directory (1,000 folders)` | 1. Click "Scan Now".<br>2. Wait 2 seconds.<br>3. Click "Cancel". |`CancellationToken` signaled. Halt occurs ≤ 1s. UI displays cancelled immediately.`ScanState` ->`Cancelled`. | S1 | AC-25.2.1 |
| TC-25-06 | Preserved Partial Results on Cancel | Edge | Medium |`Same as TC-25-05` | 1. Start Scan.<br>2. Cancel midway.<br>3. Inspect Tauri returned`Vec<ScanResult>`. | The partial results collected up to the cancellation point are returned intact reliably and NOT discarded. | S2 | AC-25.2.2 |
| TC-25-07 | Late Cancellation Check | Edge | Low |`Quick scan (1 folder)` | 1. Allow scan to fully complete.<br>2. Invoke`cancel_scan` via console. | Command returns`AlreadyCompleted` status. | S4 | AC-25.2.3 |
| TC-25-08 | Valid Thumbnail Extracted | Positive | High |`ModA with preview.png, ModB without` | 1. Trigger Full Scan.<br>2. Inspect`ScanResult` array for`ModA` and`ModB`. |`ModA` ->`thumbnail_path = Some(absolute_path)`.`ModB` ->`thumbnail_path = None` physically. | S2 | AC-25.3.1, AC-25.3.2 |
| TC-25-09 | Folder Signal & INI Extraction | Positive | High |`Folder with.ini containing TextureOverride` | 1. Trigger scan on target.<br>2. Inspect`FolderSignals` in result. | Name extracted manually. INI headers parsed. | S1 | AC-25.4.1, AC-25.4.2 |
| TC-25-10 | Malformed INI Parsing | Negative | Medium |`.ini filled with random binary garbage bytes` | 1. Trigger scan on target.<br>2. Inspect`FolderSignals`. | Scanner skips binary INI manually falling back protecting. | S2 | AC-25.4.3 |

## E. Test Environment Setup

- **Preconditions**: OS: Windows 10/11. App: EMMM2 dev build. Benchmark directory injected.
- **Context Injection**:
 -`mods_path` containing exactly 1,000 folders to validate speed.
 - Revoked Read OS Permissions folder `Mods/Secret` to test I/O error skipping.
 - Symlink loop `LoopA` constructed to simulate extreme recursion and ensure loop breaking.

## H. Cross-Epic E2E Scenarios

- **E2E-25-01 (Scan Engine to Deep Match Pipeline)**: User clicks "Scan Now" (Epic 25). The background thread parses the disk and emits progress signals to the UI. Once the disk scan completes, the payload is handed off to the Deep Matcher (Epic 26) which categorizes the new folders, committing the final structured data to the SQLite database (Epic 27).
