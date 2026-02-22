# Test Case Scenarios: Epic 10 - Automation & QoL

**Objective:** Verify integrated launcher (`launcher_path`), mod randomizer (Safe Mode aware), content curation (Pin/Favorite), operation lock, and empty pool handling.

**Ref:** [epic10-qol.md](file:///e:/Dev/EMMM2NEW/.docs/epic10-qol.md) | TRD §2.1, §3.6

---

## 1. Functional Test Cases (Positive)

### US-10.1: Integrated Launcher

| ID             | Title                  | Pre-Condition                       | Steps            | Expected Result                                                                                  | Post-Condition | Priority |
| :------------- | :--------------------- | :---------------------------------- | :--------------- | :----------------------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-10.1-01** | **Play Game**          | - Valid `launcher_path` configured. | 1. Click "Play". | - 3DMigoto Loader starts (Admin via UAC).<br>- Game EXE starts.<br>- App auto-closes (optional). | Running.       | High     |
| **TC-10.1-02** | **Custom Launch Args** | - `launch_args`: `-popupwindow`.    | 1. Click "Play". | - Game opens in windowed mode.<br>- Args passed verbatim to `Command::args`.                     | Correct mode.  | Medium   |

### US-10.2: Mod Randomizer

| ID             | Title                    | Pre-Condition                              | Steps                 | Expected Result                                                                                      | Post-Condition | Priority |
| :------------- | :----------------------- | :----------------------------------------- | :-------------------- | :--------------------------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-10.2-01** | **Gacha Roll**           | - Multiple mods for same object.           | 1. Click "Gacha Mod". | - Random mod selected.<br>- Preview dialog shown.<br>- Click "Apply" → mod enabled, others disabled. | Applied.       | Medium   |
| **TC-10.2-02** | **Safe Mode Randomizer** | - Safe Mode ON.<br>- Mix of SFW/NSFW mods. | 1. Click Gacha.       | - Pool excludes `is_safe = false` mods.<br>- Only SFW mods selectable.                               | Filtered.      | High     |

### US-10.3: Content Curation (Pin/Favorite)

| ID             | Title            | Pre-Condition    | Steps                     | Expected Result                                                      | Post-Condition | Priority |
| :------------- | :--------------- | :--------------- | :------------------------ | :------------------------------------------------------------------- | :------------- | :------- |
| **TC-10.3-01** | **Pin Mod**      | - Mod in grid.   | 1. Click "Pin" icon.      | - `is_pinned = true` in DB.<br>- Mod moves to top of grid instantly. | Pinned.        | High     |
| **TC-10.3-02** | **Favorite Mod** | - Mod in grid.   | 1. Click "Favorite" icon. | - `is_favorite = true` in DB.<br>- Star icon shown on card.          | Favorited.     | Medium   |
| **TC-10.3-03** | **Unpin Mod**    | - Mod is pinned. | 1. Click "Unpin".         | - `is_pinned = false`.<br>- Mod returns to normal sort position.     | Unpinned.      | Medium   |

---

## 2. Negative Test Cases (Error Handling)

### US-10.1: Launch Errors

| ID             | Title                  | Pre-Condition              | Steps                              | Expected Result                                                                              | Post-Condition   | Priority |
| :------------- | :--------------------- | :------------------------- | :--------------------------------- | :------------------------------------------------------------------------------------------- | :--------------- | :------- |
| **NC-10.1-01** | **Admin Denied**       | - UAC prompt shown.        | 1. Click Play → Click "No" on UAC. | - Log: "Launch Cancelled".<br>- Toast: "Please allow Admin access".<br>- App remains active. | Stable.          | High     |
| **NC-10.1-02** | **Launcher Not Found** | - `launcher_path` invalid. | 1. Click Play.                     | - Toast: "Launcher Not Found".<br>- Redirect to Settings (E11).                              | Settings opened. | High     |
| **NC-10.1-03** | **Loader Crash**       | - Corrupt loader EXE.      | 1. Click Play.                     | - Error: "Loader exited with code X".<br>- Details logged.                                   | Logged.          | Medium   |

### US-10.2: Randomizer Errors

| ID             | Title                     | Pre-Condition                     | Steps                   | Expected Result                                 | Post-Condition | Priority |
| :------------- | :------------------------ | :-------------------------------- | :---------------------- | :---------------------------------------------- | :------------- | :------- |
| **NC-10.2-01** | **Empty Pool**            | - All mods filtered (0 eligible). | 1. Click Gacha.         | - Toast: "No mods available for randomization". | No action.     | Medium   |
| **NC-10.2-02** | **Operation Lock Active** | - Toggle operation running.       | 1. Click Gacha "Apply". | - Toast: "Operation in progress".<br>- Blocked. | Queued.        | High     |

---

## 3. Edge Cases & Stability

| ID           | Title                    | Simulation Step                    | Expected                                                                   | Handling                                         | Priority |
| ------------ | ------------------------ | ---------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------ | -------- |
| **EC-10.01** | **Spam Play Button**     | 1\\. Click Play 10x.               | \\- Button disabled after 1st click. \\- Toast: "Game is launching...".    | —                                                | High     |
| **EC-10.02** | **Game Already Running** | 1\\. Game active. 2\\. Click Play. | \\- Warn: "Game already running". \\- OR Focus existing window.            | —                                                | Medium   |
| **EC-10.03** | **System Mods Filter**   | 1\\. Mod `.Fixes` (hidden/system). | 1\\. Randomize.                                                            | \\- Never selects `.Fixes` or `.` prefixed mods. | High     |
| **EC-10.04** | **Mandatory Mods**       | 1\\. Randomizer picks mod.         | \\- Does not disable `.` prefix (system) mods. \\- Only toggles user mods. | —                                                | High     |

---

## 4. Technical Metrics

| ID           | Metric               | Threshold   | Method                              |
| :----------- | :------------------- | :---------- | :---------------------------------- |
| **TM-10.01** | **Launch Latency**   | **< 100ms** | `Command::new()` execution time.    |
| **TM-10.02** | **Randomizer Logic** | **< 10ms**  | Selection algorithm with 10k items. |

---

## 5. Data Integrity

| ID           | Object                   | Logic                                                                                 |
| :----------- | :----------------------- | :------------------------------------------------------------------------------------ |
| **DI-10.01** | **Launch Args**          | `launch_args` from `games` table passed verbatim to `Command::args`. No modification. |
| **DI-10.02** | **`launcher_path`**      | Uses `launcher_path` field (not `loader_path`). Validated absolute path.              |
| **DI-10.03** | **Pin Sort Order**       | Pinned items (`is_pinned = true`) MUST sort before unpinned in all views.             |
| **DI-10.04** | **Randomizer Exclusion** | Mods with `.` prefix (system mods) excluded from randomizer pool.                     |
