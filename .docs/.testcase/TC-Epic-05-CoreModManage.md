# Test Case Scenarios: Epic 5 - Core Mod Management

**Objective:** Verify toggle enable/disable, bulk import, undo toast, operation lock, watcher suppression, and data consistency using `status` ENUM.

**Ref:** [epic5-core-mod-manage.md](file:///e:/Dev/EMMM2NEW/.docs/epic5-core-mod-manage.md) | TRD §2.2, §3.5, §3.6

---

## 1. Functional Test Cases (Positive)

### US-5.1: Toggle Enable/Disable

| ID            | Title           | Pre-Condition        | Steps                                 | Expected Result                                                                      | Post-Condition  | Priority |
| :------------ | :-------------- | :------------------- | :------------------------------------ | :----------------------------------------------------------------------------------- | :-------------- | :------- |
| **TC-5.1-01** | **Enable Mod**  | - `DISABLED Ayaka`.  | 1. Click Enable.                      | - Renamed to `Ayaka`.<br>- DB `status` → `ENABLED`.<br>- TanStack Query invalidated. | Enabled.        | High     |
| **TC-5.1-02** | **Disable Mod** | - `Ayaka` (enabled). | 1. Click Disable.                     | - Renamed to `DISABLED Ayaka`.<br>- DB `status` → `DISABLED`.                        | Disabled.       | High     |
| **TC-5.1-03** | **Undo Toggle** | - Just toggled mod.  | 1. Click "Undo" on toast (within 5s). | - Rename reverted.<br>- DB reverted.<br>- Toast: "Undo successful".                  | Original state. | High     |

### US-5.2: Smart Import

| ID            | Title                   | Pre-Condition     | Steps                    | Expected Result                                                                           | Post-Condition | Priority |
| :------------ | :---------------------- | :---------------- | :----------------------- | :---------------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-5.2-01** | **Drag-Drop Import**    | - Zip file ready. | 1. Drag `.zip` onto app. | - Extracted via `compress-tools`.<br>- Auto-categorized.<br>- Toast: "Import successful". | Mod listed.    | High     |
| **TC-5.2-02** | **Bulk Archive Import** | - 10 zip files.   | 1. Drag 10 zips.         | - Progress bar: "Importing 1/10...".<br>- Summary modal on complete.                      | All imported.  | High     |

### US-5.3: Enable Only This

| ID            | Title                | Pre-Condition                 | Steps                  | Expected Result                                                | Post-Condition | Priority |
| :------------ | :------------------- | :---------------------------- | :--------------------- | :------------------------------------------------------------- | :------------- | :------- |
| **TC-5.3-01** | **Isolation Toggle** | - ModA active, ModB disabled. | 1. "Enable Only" ModB. | - ModA disabled → `DISABLED ModA`.<br>- ModB enabled → `ModB`. | Swapped.       | High     |

---

## 2. Negative Test Cases (Error Handling)

### US-5.1: Toggle Errors

| ID            | Title                     | Pre-Condition                      | Steps                   | Expected Result                                                  | Post-Condition | Priority |
| :------------ | :------------------------ | :--------------------------------- | :---------------------- | :--------------------------------------------------------------- | :------------- | :------- |
| **NC-5.1-01** | **Source Missing**        | - Mod deleted externally.          | 1. Click Toggle.        | - Error: "Folder not found".<br>- UI refreshed.                  | Item removed.  | High     |
| **NC-5.1-02** | **Destination Exists**    | - `DISABLED A` and `A` both exist. | 1. Enable `DISABLED A`. | - Error: "Name collision".<br>- Abort. No data loss.             | Safe.          | High     |
| **NC-5.1-03** | **Permission Denied**     | - Folder read-only.                | 1. Toggle.              | - Error: "Permission Denied".<br>- State unchanged.              | Safe.          | Medium   |
| **NC-5.1-04** | **Operation Lock Active** | - Another operation running.       | 1. Click Toggle.        | - Toast: "Operation in progress".<br>- Action queued or blocked. | Queued.        | High     |

### US-5.2: Import Errors

| ID            | Title                   | Pre-Condition     | Steps              | Expected Result                                                       | Post-Condition    | Priority |
| :------------ | :---------------------- | :---------------- | :----------------- | :-------------------------------------------------------------------- | :---------------- | :------- |
| **NC-5.2-01** | **Corrupt Zip**         | - Bad header.     | 1. Drag-drop.      | - Error: "Extraction Failed".<br>- Skip corrupt file.                 | Others processed. | High     |
| **NC-5.2-02** | **Existing Mod Name**   | - Mod ID exists.  | 1. Import same.    | - Prompt: "Overwrite / Rename / Cancel".                              | User decides.     | Medium   |
| **NC-5.2-03** | **Duplicate Character** | - Ganyu A active. | 1. Enable Ganyu B. | - Alert: "Duplicate Character Active".<br>- User confirms or cancels. | Warned.           | High     |

---

## 3. Edge Cases & Stability

| ID          | Title                          | Simulation Step                                       | Expected Handling                                                                          | Priority |
| :---------- | :----------------------------- | :---------------------------------------------------- | :----------------------------------------------------------------------------------------- | :------- |
| **EC-5.01** | **Race Condition Toggle**      | 1. Select 10 mods.<br>2. Spam "Enable" fast.          | - Operation Lock (TRD §3.6) queues operations.<br>- Execute 1 by 1.<br>- No DB lock error. | High     |
| **EC-5.02** | **Partial "Enable Only" Fail** | 1. ModA disable OK.<br>2. ModB enable fails (locked). | - Transaction rollback.<br>- ModA re-enabled.<br>- Error toast.                            | High     |
| **EC-5.03** | **Long Filenames**             | 1. Mod name 250 chars.                                | - Enable/Disable preserves full name.<br>- No truncation.                                  | Medium   |
| **EC-5.04** | **Game Running**               | 1. Game process active.<br>2. Toggle mod.             | - Allowed (3DMigoto supports hot swap).<br>- If file locked → fail gracefully.             | Medium   |
| **EC-5.05** | **Watcher Suppression**        | 1. Toggle mod (app rename).<br>2. Watcher fires.      | - Watcher suppressed during app I/O (TRD §3.5).<br>- No double-processing.                 | High     |
| **EC-5.06** | **Bad Prefix Fix**             | 1. Folder named `disabled ayaka` (lowercase).         | - Regex detects non-standard prefix.<br>- Standardize to `DISABLED Ayaka`.                 | Medium   |

---

## 4. Technical Metrics

| ID          | Metric           | Threshold   | Method                       |
| :---------- | :--------------- | :---------- | :--------------------------- |
| **TM-5.01** | **Toggle Speed** | **< 50ms**  | `fs::rename` + DB update.    |
| **TM-5.02** | **Batch Speed**  | **< 2s**    | "Enable All" for 50 items.   |
| **TM-5.03** | **Undo Latency** | **< 100ms** | Undo action to state revert. |

---

## 5. Data Integrity

| ID          | Object                    | Logic                                                                                        |
| :---------- | :------------------------ | :------------------------------------------------------------------------------------------- |
| **DI-5.01** | **Atomic Rename**         | Rename operation must be atomic. No "half-renamed" states.                                   |
| **DI-5.02** | **Status ENUM**           | `status` in DB must be `ENABLED` or `DISABLED`. Must match `!name.starts_with("DISABLED ")`. |
| **DI-5.03** | **TanStack Invalidation** | After toggle, `['mods', gameId]` query cache invalidated.                                    |
