# Test Case Scenarios: Epic 2 - Intelligent Mod Scanning

**Objective:** Verify scanner accuracy, archive handling, fuzzy matching, scan progress/cancellation, watcher suppression, and conflict detection.

**Ref:** [epic2-mod-scan-organization.md](file:///e:/Dev/EMMM2NEW/.docs/epic2-mod-scan-organization.md) | TRD §3.2, §3.5

---

## 1. Functional Test Cases (Positive)

### US-2.1: Archive Detection & Extraction

| ID            | Title                | Pre-Condition                   | Steps                     | Expected Result                                                                              | Post-Condition            | Priority |
| :------------ | :------------------- | :------------------------------ | :------------------------ | :------------------------------------------------------------------------------------------- | :------------------------ | :------- |
| **TC-2.1-01** | **Zip Extraction**   | - Valid `.zip` in `/Mods`.      | 1. Scan folder.           | - Archive detected.<br>- Content extracted to mod folder.<br>- Original zip moved to backup. | Valid mod folder created. | High     |
| **TC-2.1-02** | **Smart Flattening** | - Nested `A/B/C/mod.ini`.       | 1. Extract archive.       | - Result: `ModName/mod.ini` (depth reduced).<br>- No unnecessary wrappers.                   | Depth ≤ 2.                | Medium   |
| **TC-2.1-03** | **7z & RAR Support** | - Valid `.7z` and `.rar` files. | 1. Scan folder.           | - Both formats extracted via pure Rust crates (`sevenz-rust`, `rar`).<br>- Same flow as zip. | Valid mod folders.        | Medium   |
| **TC-2.1-04** | **ZIP Password**     | - Password-protected `.zip`.    | 1. Extract with password. | - Decrypted and extracted successfully.<br>- Files accessible.                               | Valid mod folder.         | Medium   |
| **TC-2.1-05** | **7z Password**      | - Password-protected `.7z`.     | 1. Extract with password. | - Decrypted and extracted successfully.<br>- Files accessible.                               | Valid mod folder.         | Medium   |

### US-2.2: Matcher Pipeline

| ID            | Title                    | Pre-Condition                                    | Steps    | Expected Result                                                                    | Post-Condition     | Priority |
| :------------ | :----------------------- | :----------------------------------------------- | :------- | :--------------------------------------------------------------------------------- | :----------------- | :------- |
| **TC-2.2-01** | **Name Match (Exact)**   | - Folder: `[Mod] Raiden`.                        | 1. Scan. | - Matched to "Raiden Shogun".<br>- Confidence: HIGH.                               | `object_type` set. | High     |
| **TC-2.2-02** | **Content Match (INI)**  | - Folder: `Unknown`.<br>- Contains `albedo.ini`. | 1. Scan. | - Matched to "Albedo" via INI filename.<br>- Confidence: MEDIUM.                   | `object_type` set. | High     |
| **TC-2.2-03** | **Fuzzy Match (strsim)** | - Folder: `Raidn_Shogn`.                         | 1. Scan. | - `strsim` Levenshtein ≤ 3.<br>- Matched to "Raiden Shogun".<br>- Confidence: LOW. | Suggest to user.   | Medium   |

### US-2.3: Scan Progress & Cancellation

| ID            | Title            | Pre-Condition          | Steps              | Expected Result                                                           | Post-Condition    | Priority |
| :------------ | :--------------- | :--------------------- | :----------------- | :------------------------------------------------------------------------ | :---------------- | :------- |
| **TC-2.3-01** | **Progress Bar** | - 100 folders to scan. | 1. Start scan.     | - Progress bar: "Scanning 1/100...".<br>- Updates in real-time.           | All scanned.      | High     |
| **TC-2.3-02** | **Cancel Scan**  | - Scan running.        | 1. Click "Cancel". | - Scan aborts cleanly.<br>- Partial results discarded.<br>- DB unchanged. | State consistent. | High     |

### US-2.4: Conflict Detection

| ID            | Title                      | Pre-Condition                       | Steps    | Expected Result                                                             | Post-Condition | Priority |
| :------------ | :------------------------- | :---------------------------------- | :------- | :-------------------------------------------------------------------------- | :------------- | :------- |
| **TC-2.4-01** | **Shader Conflict Notice** | - Two mods modify same shader hash. | 1. Scan. | - Warning: "Potential conflict: ModA vs ModB".<br>- Listed in scan results. | User decides.  | Medium   |

---

## 2. Negative Test Cases (Error Handling)

### US-2.1: Archive Errors

| ID            | Title                    | Pre-Condition                     | Steps               | Expected Result                                                      | Post-Condition   | Priority |
| :------------ | :----------------------- | :-------------------------------- | :------------------ | :------------------------------------------------------------------- | :--------------- | :------- |
| **NC-2.1-01** | **Corrupt Archive**      | - Broken `.zip` header.           | 1. Extract.         | - Error: "Extraction Failed: CRC Error".<br>- Item marked red in UI. | File untouched.  | High     |
| **NC-2.1-02** | **Password Protected**   | - Encrypted zip.                  | 1. Extract.         | - Error: "Password required".<br>- Cancel → Abort extraction.        | Skipped.         | Medium   |
| **NC-2.1-03** | **Disk Full**            | - 0 bytes free.                   | 1. Extract 1GB mod. | - Error: "Disk Full".<br>- Cleanup partial files.                    | No corrupt data. | High     |
| **NC-2.1-04** | **Wrong Password (ZIP)** | - Encrypted zip + wrong password. | 1. Extract.         | - Error: "Invalid password".<br>- No partial data written.           | File untouched.  | Medium   |
| **NC-2.1-05** | **Wrong Password (7z)**  | - Encrypted 7z + wrong password.  | 1. Extract.         | - Error: "Invalid password".<br>- No partial data written.           | File untouched.  | Medium   |

### US-2.2: Scanning Errors

| ID            | Title                 | Pre-Condition           | Steps            | Expected Result                                                  | Post-Condition     | Priority |
| :------------ | :-------------------- | :---------------------- | :--------------- | :--------------------------------------------------------------- | :----------------- | :------- |
| **NC-2.2-01** | **Permission Denied** | - File locked by Admin. | 1. Scan content. | - Log "Skip: permission denied".<br>- Continue to next.          | App active.        | Medium   |
| **NC-2.2-02** | **No Match Found**    | - Folder `XYZ_Random`.  | 1. Scan.         | - Categorized as "Uncategorized".<br>- User can manually assign. | Listed in results. | Medium   |

---

## 3. Edge Cases & Stability

| ID          | Title                          | Simulation Step                                     | Expected Handling                                                                   | Priority |
| :---------- | :----------------------------- | :-------------------------------------------------- | :---------------------------------------------------------------------------------- | :------- |
| **EC-2.01** | **Archive Bomb (Zip Bomb)**    | 1. Zip expands to 100GB.                            | - Monitor disk usage.<br>- Abort if > free space OR > 20GB limit.                   | High     |
| **EC-2.02** | **Infinite Nesting/Symlink**   | 1. `A/A/A/A...` (Symlink loop).                     | - Scan depth limit (5).<br>- Break loop, log warning.                               | High     |
| **EC-2.03** | **Non-ASCII CJK Paths**        | 1. Name: `神里綾華`.                                | - `PathBuf` handles safely.<br>- Match via INI content or preserve name.            | High     |
| **EC-2.04** | **Scan While External Delete** | 1. Start scan.<br>2. Delete folder externally.      | - `WalkDir` error caught.<br>- Skip item, no crash.                                 | Medium   |
| **EC-2.05** | **Zero-Byte INI**              | 1. `mod.ini` is 0 bytes.                            | - Skip content match.<br>- Rely on folder name.                                     | Low      |
| **EC-2.06** | **Watcher Suppression**        | 1. Scan extracts archive.<br>2. File watcher fires. | - Watcher suppressed during scan ops (TRD §3.5).<br>- No duplicate events or loops. | High     |
| **EC-2.07** | **Duplicate Destination**      | 1. Extract creates folder that already exists.      | - Prompt: "Folder exists. Merge / Rename / Skip?".<br>- No silent overwrite.        | High     |

---

## 4. Technical Metrics

| ID          | Metric               | Threshold   | Method                      |
| :---------- | :------------------- | :---------- | :-------------------------- |
| **TM-2.01** | **Scan Speed**       | **< 10s**   | 500 folders / 10GB total.   |
| **TM-2.02** | **Memory Usage**     | **< 500MB** | Peak RAM during extraction. |
| **TM-2.03** | **Watchdog Latency** | **< 300ms** | File event to UI update.    |
| **TM-2.04** | **Cancellation**     | **< 500ms** | Cancel signal to full stop. |

---

## 5. Data Integrity

| ID          | Object             | Logic                                                                        |
| :---------- | :----------------- | :--------------------------------------------------------------------------- | --------------------------- |
| **DI-2.01** | **Sanitization**   | File names must not contain `\ / : \* ? " < >                                | `. Regex replace with `\_`. |
| **DI-2.02** | **Scan Atomicity** | Cancelled scan must leave DB in pre-scan state. No partial inserts.          |
| **DI-2.03** | **Watcher State**  | Watcher must resume after scan completes. Never left permanently suppressed. |
