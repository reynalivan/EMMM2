# Test Case Scenarios: Epic 1 - Onboarding & System Configuration

**Objective:** Validate app initialization, 3DMigoto auto-discovery, secure config management, multi-instance prevention, duplicate game guard, and default Safe Mode enforcement.

**Ref:** [epic1-onboarding-config.md](file:///e:/Dev/EMMM2NEW/.docs/epic1-onboarding-config.md) | TRD §2.1, §3.5, §3.6

---

## 1. Functional Test Cases (Positive)

### US-1.1: Setup Mode Selection & Welcome Screen

| ID            | Title                              | Pre-Condition                              | Steps          | Expected Result                                                                   | Post-Condition      | Priority |
| :------------ | :--------------------------------- | :----------------------------------------- | :------------- | :-------------------------------------------------------------------------------- | :------------------ | :------- |
| **TC-1.1-01** | **Fresh Install - Welcome Screen** | - App data cleared.<br>- No `config.json`. | 1. Launch App. | - Shows "Welcome Screen".<br>- Buttons Enabled.<br>- Loading shimmer during init. | Route `/welcome`.   | High     |
| **TC-1.1-02** | **Existing Config - Bypass**       | - Valid `config.json`.                     | 1. Launch App. | - Skips Welcome.<br>- Navigates to Dashboard.                                     | Route `/dashboard`. | High     |
| **TC-1.1-03** | **Default SFW on Startup**         | - Any valid config.                        | 1. Launch App. | - `appStore.safeMode = true`.<br>- NSFW mods hidden from UI.                      | Safe Mode active.   | High     |

### US-1.2: XXMI Auto-Discovery

| ID            | Title                    | Pre-Condition                      | Steps                   | Expected Result                                                               | Post-Condition              | Priority |
| :------------ | :----------------------- | :--------------------------------- | :---------------------- | :---------------------------------------------------------------------------- | :-------------------------- | :------- |
| **TC-1.2-01** | **Auto-Detect Success**  | - Valid GIMI folder at `C:\Games`. | 1. Click "Auto-Detect". | - Found "Genshin Impact".<br>- Toast Success.<br>- `launcher_path` populated. | Config + DB Updated.        | High     |
| **TC-1.2-02** | **Multi-Game Discovery** | - GIMI + SRMI folders.             | 1. Auto-Detect root.    | - Both games found.<br>- Listed with correct `game_type`.                     | 2 entries in `games` table. | High     |

### US-1.3: Manual Path Setup

| ID            | Title                     | Pre-Condition        | Steps                                        | Expected Result                                         | Post-Condition          | Priority |
| :------------ | :------------------------ | :------------------- | :------------------------------------------- | :------------------------------------------------------ | :---------------------- | :------- |
| **TC-1.3-01** | **Manual Add Success**    | - Valid GIMI folder. | 1. Browse `D:\Games\GIMI`.<br>2. Click Save. | - `validate_instance` passes.<br>- Saved to DB.         | Game listed in sidebar. | High     |
| **TC-1.3-02** | **Form Validation (Zod)** | - Form rendered.     | 1. Fill all fields valid.<br>2. Submit.      | - React Hook Form + Zod validates.<br>- No red borders. | Saved.                  | Medium   |

### US-1.4: Multi-Instance Prevention

| ID            | Title                        | Pre-Condition               | Steps                             | Expected Result                                                 | Post-Condition  | Priority |
| :------------ | :--------------------------- | :-------------------------- | :-------------------------------- | :-------------------------------------------------------------- | :-------------- | :------- |
| **TC-1.4-01** | **Single Instance Enforced** | - App running (Instance A). | 1. Launch App again (Instance B). | - Instance B blocked.<br>- Instance A focused/brought to front. | Only 1 process. | High     |

### US-1.5: Duplicate Game Guard

| ID            | Title                      | Pre-Condition                   | Steps                   | Expected Result                                        | Post-Condition      | Priority |
| :------------ | :------------------------- | :------------------------------ | :---------------------- | :----------------------------------------------------- | :------------------ | :------- |
| **TC-1.5-01** | **Duplicate Path Blocked** | - "Genshin" already configured. | 1. Add same path again. | - Toast: "Game already registered".<br>- Save blocked. | No duplicate in DB. | High     |

---

## 2. Negative Test Cases (Error Handling)

### US-1.2: Auto-Discovery Failures

| ID            | Title                    | Pre-Condition                      | Steps                        | Expected Result                                                 | Post-Condition     | Priority |
| :------------ | :----------------------- | :--------------------------------- | :--------------------------- | :-------------------------------------------------------------- | :----------------- | :------- |
| **NC-1.2-01** | **No Valid Games Found** | - Empty root folder.               | 1. Auto-Detect `C:\Empty`.   | - Modal: "No valid instances found".<br>- Suggest Manual setup. | Stay on Welcome.   | High     |
| **NC-1.2-02** | **Missing Core Files**   | - Has `/Mods`, missing `d3dx.ini`. | 1. Scan Folder.              | - Silent skip.<br>- Log: "Missing d3dx.ini".                    | No invalid config. | Medium   |
| **NC-1.2-03** | **Access Denied**        | - Root requires Admin.             | 1. Auto-Detect `C:\Windows`. | - Error Toast: "Permission Denied".<br>- Log `EACCES`.          | App stable.        | Medium   |

### US-1.3: Manual Entry Failures

| ID            | Title                   | Pre-Condition                  | Steps                  | Expected Result                                               | Post-Condition | Priority |
| :------------ | :---------------------- | :----------------------------- | :--------------------- | :------------------------------------------------------------ | :------------- | :------- |
| **NC-1.3-01** | **Missing Mods Folder** | - Folder missing `/Mods`.      | 1. Add Folder.         | - Toast: "Invalid: Missing /Mods".<br>- Zod validation error. | Block Save.    | High     |
| **NC-1.3-02** | **Duplicate Game Path** | - Game already in config.      | 1. Add same path.      | - Toast: "Game already registered".                           | Block Save.    | Medium   |
| **NC-1.3-03** | **Non-Existent Path**   | - `Z:\Fake`.                   | 1. Type path manually. | - Zod validation fails immediately.<br>- Input border red.    | Block Save.    | High     |
| **NC-1.3-04** | **Missing DLL**         | - Has `/Mods`, no `d3d11.dll`. | 1. Add folder.         | - Error: "Critical file missing".<br>- Save blocked.          | Block Save.    | Medium   |

---

## 3. Edge Cases & Stability

| ID          | Title                     | Simulation Step                                         | Expected Handling                                                                                             | Priority |
| :---------- | :------------------------ | :------------------------------------------------------ | :------------------------------------------------------------------------------------------------------------ | :------- |
| **EC-1.01** | **Corrupt Config JSON**   | 1. Delete closing `}` from `config.json`.<br>2. Launch. | - Serde parse error caught.<br>- Backup as `config.json.bak`.<br>- Create default.<br>- Toast "Config Reset". | High     |
| **EC-1.02** | **Mixed Path Separators** | 1. Config: `D:/Games//GIMI\\Mods`.                      | - `PathBuf` canonicalizes.<br>- App works normally.                                                           | Medium   |
| **EC-1.03** | **Unusual Characters**    | 1. Path: `D:\G@mes\Genshin❤\`.                          | - Rust UTF-8 handling works.<br>- No crashes.                                                                 | Medium   |
| **EC-1.04** | **Read-Only Config**      | 1. Set `config.json` Read-Only.<br>2. Change setting.   | - Error Toast: "Cannot save config".<br>- Reverts in-memory.                                                  | Low      |
| **EC-1.05** | **Zero Byte Config**      | 1. `config.json` is 0 bytes.                            | - Treated as corrupt.<br>- Reset to default.                                                                  | High     |
| **EC-1.06** | **Multi-Instance Race**   | 1. Launch 2 instances simultaneously.                   | - `tauri-plugin-single-instance` blocks 2nd.<br>- No DB lock errors.                                          | High     |
| **EC-1.07** | **Loading State Shimmer** | 1. Trigger Auto-Detect on slow HDD.                     | - Shimmer overlay with "Scanning..." text.<br>- Cancel button available.                                      | Medium   |

---

## 4. Technical Metrics

| ID          | Metric               | Threshold   | Method                               |
| :---------- | :------------------- | :---------- | :----------------------------------- |
| **TM-1.01** | **Startup Latency**  | **< 800ms** | `app.run()` to `Dashboard.onMount`.  |
| **TM-1.02** | **Validation Speed** | **< 10ms**  | `validate_instance` execution time.  |
| **TM-1.03** | **Scan Latency**     | **< 2s**    | Auto-Discovery of 10 nested folders. |

---

## 5. Data Integrity

| ID          | Object              | Logic                                                    |
| :---------- | :------------------ | :------------------------------------------------------- |
| **DI-1.01** | **Config Schema**   | Must match `AppConfig` struct. No extra fields.          |
| **DI-1.02** | **Game ID**         | UUID v4 generated is unique and persistent.              |
| **DI-1.03** | **`launcher_path`** | Stored as validated absolute path (not `loader_path`).   |
| **DI-1.04** | **Default Safety**  | `appStore.safeMode` defaults to `true` on every startup. |
