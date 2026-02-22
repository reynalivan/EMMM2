# Test Case Scenarios: Epic 9 - Duplicate Scanner

**Objective:** Verify multi-signal duplicate detection using `blake3` hashing, `rayon` parallelism, scan progress/cancellation, bulk resolution with operation lock, and custom trash safety.

**Ref:** [epic9-duplicate-scan.md](file:///e:/Dev/EMMM2NEW/.docs/epic9-duplicate-scan.md) | TRD §3.4, §3.6

---

## 1. Functional Test Cases (Positive)

### US-9.1: Multi-Signal Detection

| ID            | Title                 | Pre-Condition                     | Steps        | Expected Result                                                     | Post-Condition | Priority |
| :------------ | :-------------------- | :-------------------------------- | :----------- | :------------------------------------------------------------------ | :------------- | :------- |
| **TC-9.1-01** | **Exact Hash Match**  | - Copy of mod folder.             | 1. Run scan. | - `blake3` hash identical → Duplicate found.<br>- Confidence: 100%. | Grouped.       | High     |
| **TC-9.1-02** | **Structure Match**   | - Same structure, renamed files.  | 1. Run scan. | - File tree similarity detected.<br>- Confidence: 70-90%.           | Grouped.       | Medium   |
| **TC-9.1-03** | **Name + Size Match** | - "Albedo" and "DISABLED Albedo". | 1. Scan.     | - Name match after normalization.<br>- Size within threshold.       | Grouped.       | High     |

### US-9.2: Resolution Actions

| ID            | Title               | Pre-Condition         | Steps                                                       | Expected Result                                                                         | Post-Condition | Priority |
| :------------ | :------------------ | :-------------------- | :---------------------------------------------------------- | :-------------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-9.2-01** | **Trash Duplicate** | - Pair found.         | 1. Select "Keep A".<br>2. Click Resolve.                    | - B moved to `./app_data/trash/`.<br>- Metadata JSON created for restore.               | Resolved.      | High     |
| **TC-9.2-02** | **Ignore Pair**     | - Pair found.         | 1. Click "Ignore".                                          | - Added to whitelist.<br>- Never flagged again.                                         | Whitelisted.   | High     |
| **TC-9.2-03** | **Bulk Resolution** | - 10 duplicate pairs. | 1. Select "Keep Original" for all.<br>2. Click "Apply All". | - 10 duplicates moved to trash.<br>- Progress: "Resolving 1/10...".<br>- Summary modal. | All resolved.  | High     |

### US-9.3: Scan Progress

| ID            | Title            | Pre-Condition       | Steps            | Expected Result                                                          | Post-Condition | Priority |
| :------------ | :--------------- | :------------------ | :--------------- | :----------------------------------------------------------------------- | :------------- | :------- |
| **TC-9.3-01** | **Progress Bar** | - 500 folders.      | 1. Start scan.   | - Progress: "Hashing X/500...".<br>- Percentage updates.                 | Complete.      | High     |
| **TC-9.3-02** | **Cancel Scan**  | - Scan in progress. | 1. Click Cancel. | - Scan stops cleanly.<br>- Partial results discarded.<br>- DB unchanged. | Clean.         | High     |

---

## 2. Negative Test Cases (Error Handling)

### US-9.2: Resolution Failures

| ID            | Title                     | Pre-Condition                | Steps                      | Expected Result                                                     | Post-Condition | Priority |
| :------------ | :------------------------ | :--------------------------- | :------------------------- | :------------------------------------------------------------------ | :------------- | :------- |
| **NC-9.2-01** | **File In Use**           | - Mod B open in another app. | 1. Click "Delete B".       | - Error: "File locked".<br>- Skip B, continue others.               | Safe.          | High     |
| **NC-9.2-02** | **Already Deleted**       | - B deleted externally.      | 1. Click "Delete B" in UI. | - Error: "Path not found".<br>- UI updated (remove entry).          | Synced.        | Medium   |
| **NC-9.2-03** | **Operation Lock Active** | - Another operation running. | 1. Click "Apply All".      | - Toast: "Operation in progress".<br>- Blocked until lock released. | Queued.        | High     |

### US-9.1: Scan Errors

| ID            | Title                 | Pre-Condition          | Steps    | Expected Result                                                                | Post-Condition   | Priority |
| :------------ | :-------------------- | :--------------------- | :------- | :----------------------------------------------------------------------------- | :--------------- | :------- |
| **NC-9.1-01** | **Permission Denied** | - Some folders locked. | 1. Scan. | - Skip locked folders with warning.<br>- Continue scanning accessible folders. | Partial results. | Medium   |

---

## 3. Edge Cases & Stability

| ID          | Title                       | Simulation Step                             | Expected Handling                                                                                 | Priority |
| :---------- | :-------------------------- | :------------------------------------------ | :------------------------------------------------------------------------------------------------ | :------- |
| **EC-9.01** | **10 Copies of Same Mod**   | 1. Create 10 identical copies.              | - Group all 10 in ONE cluster.<br>- "Keep 1, Delete 9" option.                                    | High     |
| **EC-9.02** | **False Positive Guard**    | 1. Two files, same name, different content. | - `blake3` hash differs → Confidence < 80%.<br>- Marked "Low Confidence".<br>- Not auto-resolved. | High     |
| **EC-9.03** | **Zero Duplicates**         | 1. Scan clean library.                      | - Result: 0 duplicates.<br>- UI: "No duplicates found! 🎉".                                       | Medium   |
| **EC-9.04** | **Nested Duplicates**       | 1. ModA contains ModB inside its folder.    | - Scan handles recursion correctly.<br>- No false inner matches.                                  | Medium   |
| **EC-9.05** | **Scan Cancelled Mid-Hash** | 1. Cancel during `blake3` hashing.          | - `rayon` thread pool stops.<br>- No resource leaks.                                              | High     |

---

## 4. Technical Metrics

| ID          | Metric                  | Threshold   | Method                                                   |
| :---------- | :---------------------- | :---------- | :------------------------------------------------------- |
| **TM-9.01** | **Scan Speed**          | **< 15s**   | `blake3` hash + structure analysis, 1,000 folders (SSD). |
| **TM-9.02** | **CPU Utilization**     | **80-90%**  | `rayon` parallelism during scanning.                     |
| **TM-9.03** | **False Positive Rate** | **0%**      | Manual verification on test set.                         |
| **TM-9.04** | **Cancel Speed**        | **< 500ms** | Cancel signal to full stop.                              |

---

## 5. Data Integrity

| ID          | Object             | Logic                                                                      |
| :---------- | :----------------- | :------------------------------------------------------------------------- |
| **DI-9.01** | **Whitelist**      | `whitelist` table persists across restarts. Ignored pairs never reflagged. |
| **DI-9.02** | **Hash Algorithm** | MUST use `blake3` (per TRD). NOT SHA1/SHA256.                              |
| **DI-9.03** | **Trash Metadata** | Deleted duplicates in `./app_data/trash/` with restore metadata JSON.      |
| **DI-9.04** | **Scan Atomicity** | Cancelled scan leaves DB in pre-scan state. No partial results persisted.  |
