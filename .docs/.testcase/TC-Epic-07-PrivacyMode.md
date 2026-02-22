# Test Case Scenarios: Epic 7 - Privacy Mode (Master Mode Switcher)

**Objective:** Verify SFW/NSFW switching security, dual guard (Frontend Zustand + Backend SQL), default SFW on startup, PIN lockout (5 attempts), operation lock, and atomicity.

**Ref:** [epic7-privacy-mode.md](file:///e:/Dev/EMMM2NEW/.docs/epic7-privacy-mode.md) | TRD §2.2, §3.5, §3.6

---

## 1. Functional Test Cases (Positive)

### US-7.1: Mode Switch

| ID            | Title                      | Pre-Condition           | Steps                                     | Expected Result                                                                              | Post-Condition   | Priority |
| :------------ | :------------------------- | :---------------------- | :---------------------------------------- | :------------------------------------------------------------------------------------------- | :--------------- | :------- |
| **TC-7.1-01** | **Enter Safe Mode**        | - NSFW mods active.     | 1. Click "Safe Mode" toggle.              | - NSFW mods physically disabled (renamed).<br>- UI filtered.<br>- Zustand `safeMode = true`. | Safe active.     | High     |
| **TC-7.1-02** | **Exit Safe Mode**         | - Safe Mode ON.         | 1. Click toggle.<br>2. Enter correct PIN. | - NSFW mods restored (re-enabled).<br>- UI unfiltered.<br>- Zustand `safeMode = false`.      | NSFW active.     | High     |
| **TC-7.1-03** | **Default SFW on Startup** | - Any config state.     | 1. Launch app.                            | - `appStore.safeMode = true` regardless of last session.<br>- NSFW content hidden.           | Safe by default. | High     |
| **TC-7.1-04** | **One-Click Swap**         | - Multiple NSFW mods.   | 1. Toggle Safe Mode.                      | - All NSFW mods toggled atomically.<br>- Transaction: all succeed or all rollback.           | Atomic.          | High     |
| **TC-7.1-05** | **State Memory**           | - ModA enabled in NSFW. | 1. Switch SFW.<br>2. Switch back NSFW.    | - ModA restored to Enabled.<br>- Per-context state preserved via `last_status_active`.       | Restored.        | High     |

### US-7.2: NSFW Tagging

| ID            | Title                 | Pre-Condition               | Steps                            | Expected Result                                         | Post-Condition | Priority |
| :------------ | :-------------------- | :-------------------------- | :------------------------------- | :------------------------------------------------------ | :------------- | :------- |
| **TC-7.2-01** | **Auto Tag by Name**  | - Mod name contains "Nude". | 1. Scan.                         | - Auto-tagged `is_safe = false`.                        | NSFW flagged.  | High     |
| **TC-7.2-02** | **Manual Tag Toggle** | - Any mod.                  | 1. Right-click → "Mark as NSFW". | - `is_safe` toggled in DB.<br>- UI updates immediately. | Tagged.        | Medium   |

### US-7.3: Dual Guard Verification

| ID            | Title                        | Pre-Condition   | Steps                                                     | Expected Result                                                                                                      | Post-Condition    | Priority |
| :------------ | :--------------------------- | :-------------- | :-------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------- | :---------------- | :------- |
| **TC-7.3-01** | **Frontend + Backend Guard** | - Safe Mode ON. | 1. Query `mods` from frontend.<br>2. Inspect backend SQL. | - Frontend: Zustand filters UI.<br>- Backend: SQL appends `AND is_safe = 1`.<br>- Both guards active simultaneously. | Double protected. | High     |

---

## 2. Negative Test Cases (Error Handling)

### US-7.3: Lock Security

| ID            | Title                    | Pre-Condition        | Steps                        | Expected Result                                                                       | Post-Condition | Priority |
| :------------ | :----------------------- | :------------------- | :--------------------------- | :------------------------------------------------------------------------------------ | :------------- | :------- |
| **NC-7.3-01** | **Wrong PIN**            | - PIN set.           | 1. Enter wrong PIN.          | - Error "Invalid PIN".<br>- Retries remaining shown.                                  | Locked.        | High     |
| **NC-7.3-02** | **Lockout (5 Attempts)** | - 4 failed attempts. | 1. Enter wrong PIN 5th time. | - UI blocks input for 60s.<br>- Countdown visible.<br>- Log: "PIN lockout triggered". | Hard lock.     | High     |

### US-7.1: Switch Failures

| ID            | Title                     | Pre-Condition                        | Steps                      | Expected Result                                                            | Post-Condition    | Priority |
| :------------ | :------------------------ | :----------------------------------- | :------------------------- | :------------------------------------------------------------------------- | :---------------- | :------- |
| **NC-7.1-01** | **File Permission Error** | - Mod folder locked.                 | 1. Switch mode.            | - Error: "Transaction Failed".<br>- Full rollback: all mods reverted.      | State consistent. | High     |
| **NC-7.1-02** | **Operation Lock Active** | - Another operation running.         | 1. Click Safe Mode toggle. | - Toast: "Operation in progress".<br>- Action blocked until lock released. | Queued.           | High     |
| **NC-7.1-03** | **Missing Target Folder** | - DB has "Mod A" but folder deleted. | 1. Switch mode.            | - Log warning, skip Mod A.<br>- Continue with rest (Soft Fail).            | Partial complete. | Medium   |

---

## 3. Edge Cases & Stability

| ID          | Title                                 | Simulation Step                                           | Expected Handling                                                                                    | Priority |
| :---------- | :------------------------------------ | :-------------------------------------------------------- | :--------------------------------------------------------------------------------------------------- | :------- |
| **EC-7.01** | **Kill Process Mid-Switch**           | 1. Click switch.<br>2. Kill app at 50%.                   | - Startup detects mismatch (DB vs filesystem).<br>- Auto-repair: force Safe Mode (disable all NSFW). | High     |
| **EC-7.02** | **External NSFW Enable**              | 1. Safe Mode ON.<br>2. User enables NSFW mod in Explorer. | - Watcher detects rename.<br>- Auto-disable OR hide from UI.<br>- Enforce Safe Mode policy.          | High     |
| **EC-7.03** | **Forgot PIN**                        | 1. User stuck, no PIN memory.                             | - Reset: manual `config.json` edit (requires OS access).<br>- Documentation in app Help.             | Medium   |
| **EC-7.04** | **New NSFW Mod in Safe Mode**         | 1. Safe Mode ON.<br>2. Import mod tagged NSFW.            | - Auto-disable immediately.<br>- Hidden from grid.<br>- Log event.                                   | High     |
| **EC-7.05** | **Dashboard Filter Dependency**       | 1. Safe Mode ON.<br>2. Open Dashboard (E13).              | - All dashboard queries filter `is_safe = 1`.<br>- Chart data excludes NSFW.                         | High     |
| **EC-7.06** | **Watcher Suppression During Switch** | 1. Toggle triggers mass renames.                          | - Watcher suppressed (TRD §3.5).<br>- No infinite loop of events.                                    | High     |

---

## 4. Technical Metrics

| ID          | Metric             | Threshold   | Method                                  |
| :---------- | :----------------- | :---------- | :-------------------------------------- |
| **TM-7.01** | **Switch Latency** | **< 2s**    | Rename 500 NSFW items.                  |
| **TM-7.02** | **Leak Check**     | **0 items** | Search "Nude" in Safe Mode → 0 results. |
| **TM-7.03** | **PIN Verify**     | **< 100ms** | Argon2 hash verification time.          |

---

## 5. Data Integrity

| ID          | Object               | Logic                                                                           |
| :---------- | :------------------- | :------------------------------------------------------------------------------ |
| **DI-7.01** | **PIN Hash**         | Argon2 hash in config. NEVER plain text.                                        |
| **DI-7.02** | **`is_safe` Column** | Defaults to `false` for new untagged mods. Only `true` after explicit mark.     |
| **DI-7.03** | **Dual Guard**       | Frontend Zustand AND backend SQL MUST both filter. Neither alone is sufficient. |
| **DI-7.04** | **Startup Default**  | `appStore.safeMode` MUST be `true` on every app launch.                         |
