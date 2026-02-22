# Test Case Scenarios: Epic 11 - Settings & Infrastructure

**Objective:** Validate game CRUD (reusing E1's `validate_instance`), atomic config writes, PIN security (5-attempt lockout), error log viewer, maintenance/orphan cleanup, and Zod form validation.

**Ref:** [epic11-settings.md](file:///e:/Dev/EMMM2NEW/.docs/epic11-settings.md) | TRD §2.1, §3.6

---

## 1. Functional Test Cases (Positive)

### US-11.1: Game Management (CRUD)

| ID             | Title                | Pre-Condition         | Steps                                                     | Expected Result                                                                            | Post-Condition | Priority |
| :------------- | :------------------- | :-------------------- | :-------------------------------------------------------- | :----------------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-11.1-01** | **Add Game**         | - Settings page open. | 1. Click "Add Game".<br>2. Browse valid path.<br>3. Save. | - `validate_instance` (from E1) passes.<br>- Sidebar refreshes with new game.              | Game added.    | High     |
| **TC-11.1-02** | **Edit Game Config** | - Game configured.    | 1. Edit `launcher_path`.<br>2. Save.                      | - Atomic write (temp + rename).<br>- Config updated.                                       | Persisted.     | High     |
| **TC-11.1-03** | **Delete Game**      | - Game with mods.     | 1. Click Delete.<br>2. Confirm.                           | - Game removed from DB.<br>- Mod entries cascade-deleted.<br>- Physical folders untouched. | Removed.       | High     |
| **TC-11.1-04** | **Update Settings**  | - Dark mode active.   | 1. Switch to Light mode.<br>2. Save.                      | - `config.json` updated atomically.<br>- UI theme toggles instantly.                       | Updated.       | Medium   |

### US-11.2: Maintenance Tasks

| ID             | Title              | Pre-Condition                       | Steps                                      | Expected Result                                                                                    | Post-Condition | Priority |
| :------------- | :----------------- | :---------------------------------- | :----------------------------------------- | :------------------------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-11.2-01** | **Empty Trash**    | - Files in `./app_data/trash/`.     | 1. Settings → Maintenance → "Empty Trash". | - All files deleted permanently.<br>- Toast: "Trash emptied".                                      | Empty.         | Medium   |
| **TC-11.2-02** | **Cache Purge**    | - Stale thumbnails > 30 days.       | 1. Click "Clear Cache".                    | - Files > 30 days deleted.<br>- Recent files preserved.                                            | Cleaned.       | Medium   |
| **TC-11.2-03** | **Orphan Cleanup** | - Orphaned `collection_items` rows. | 1. Run maintenance (manual or scheduled).  | - Orphaned rows removed.<br>- Stale thumbnails cleaned.<br>- Empty trash entries > 30 days purged. | Clean.         | Medium   |

### US-11.3: Error Log Viewer

| ID             | Title         | Pre-Condition        | Steps                   | Expected Result                                                    | Post-Condition | Priority |
| :------------- | :------------ | :------------------- | :---------------------- | :----------------------------------------------------------------- | :------------- | :------- |
| **TC-11.3-01** | **View Logs** | - Log entries exist. | 1. Settings → Logs tab. | - Recent log entries shown.<br>- Filter by level: INFO/WARN/ERROR. | Displayed.     | Medium   |

### US-11.4: PIN Management

| ID             | Title       | Pre-Condition        | Steps                             | Expected Result                                                | Post-Condition | Priority |
| :------------- | :---------- | :------------------- | :-------------------------------- | :------------------------------------------------------------- | :------------- | :------- |
| **TC-11.4-01** | **Set PIN** | - No PIN configured. | 1. Settings → Security → Set PIN. | - Argon2 hash stored in config.<br>- Plain text never written. | Secured.       | High     |

---

## 2. Negative Test Cases (Error Handling)

### US-11.1: Config Errors

| ID             | Title                      | Pre-Condition                   | Steps                          | Expected Result                                                         | Post-Condition | Priority |
| :------------- | :------------------------- | :------------------------------ | :----------------------------- | :---------------------------------------------------------------------- | :------------- | :------- |
| **NC-11.1-01** | **Invalid Path (Zod)**     | - Form rendered.                | 1. Enter `?*<>` in path field. | - Zod validation: red border.<br>- Save button disabled.                | Blocked.       | High     |
| **NC-11.1-02** | **Save Permission Denied** | - `config.json` read-only.      | 1. Save settings.              | - Error: "Failed to write config".<br>- Old config retained.            | Safe.          | Medium   |
| **NC-11.1-03** | **Duplicate Game Path**    | - Game path already registered. | 1. Add same path.              | - Warning: "Already registered" (reuses E1 service).<br>- Save blocked. | Blocked.       | Medium   |

### US-11.4: PIN Errors

| ID             | Title                      | Pre-Condition | Steps                       | Expected Result                                     | Post-Condition | Priority |
| :------------- | :------------------------- | :------------ | :-------------------------- | :-------------------------------------------------- | :------------- | :------- |
| **NC-11.4-01** | **Wrong PIN (5 Attempts)** | - PIN set.    | 1. Enter wrong PIN 5 times. | - Lockout: 60s block.<br>- Countdown timer visible. | Hard lock.     | High     |

---

## 3. Edge Cases & Stability

| ID           | Title                      | Simulation Step                            | Expected Handling                                                                | Priority |
| :----------- | :------------------------- | :----------------------------------------- | :------------------------------------------------------------------------------- | :------- |
| **EC-11.01** | **Corrupt Config**         | 1. Write random bytes to `config.json`.    | - Serde parse fails.<br>- Reset to default.<br>- Notify user.                    | High     |
| **EC-11.02** | **Crash During Save**      | 1. Kill app mid-write (atomic).            | - `config.json` intact (old version).<br>- `config.tmp` discarded on next start. | High     |
| **EC-11.03** | **Huge Cache Clear**       | 1. 100k thumbnail files.                   | - Async delete in background.<br>- UI: "Cleaning..." progress.<br>- No freeze.   | Medium   |
| **EC-11.04** | **Maintenance During Use** | 1. Orphan cleanup runs while user browses. | - No visible disruption.<br>- Background task with low priority.                 | Medium   |

---

## 4. Technical Metrics

| ID           | Metric           | Threshold         | Method                                       |
| :----------- | :--------------- | :---------------- | :------------------------------------------- |
| **TM-11.01** | **Atomic Save**  | **100% reliable** | Temp file + rename strategy. No half-writes. |
| **TM-11.02** | **PIN Verify**   | **< 100ms**       | Argon2 hash verification.                    |
| **TM-11.03** | **Save Latency** | **< 30ms**        | Config write complete.                       |

---

## 5. Data Integrity

| ID           | Object                | Logic                                                                        |
| :----------- | :-------------------- | :--------------------------------------------------------------------------- |
| **DI-11.01** | **Schema Validation** | All inputs validated via React Hook Form + Zod before serialize.             |
| **DI-11.02** | **Atomic Write**      | Config writes use temp file + rename. NEVER write directly to `config.json`. |
| **DI-11.03** | **PIN Hash**          | Stored as Argon2 hash. `config.json` must never contain plain PIN.           |
| **DI-11.04** | **Lockout Standard**  | 5 failed attempts = 60s block (standardized with E7).                        |
