# Test Case Scenarios: Epic 12 - System Maintenance & Dynamic Updates

**Objective:** Validate app binary updates (`tauri-plugin-updater` v2), metadata/asset syncing via `reqwest`, download progress, rate limiting, ETag caching, and signature verification.

**Ref:** [epic12-system-update.md](file:///e:/Dev/EMMM2NEW/.docs/epic12-system-update.md) | TRD §1.2

---

## 1. Functional Test Cases (Positive)

### US-12.1: Dynamic Metadata Sync

| ID             | Title                    | Pre-Condition                         | Steps                      | Expected Result                                                                                     | Post-Condition      | Priority |
| :------------- | :----------------------- | :------------------------------------ | :------------------------- | :-------------------------------------------------------------------------------------------------- | :------------------ | :------- |
| **TC-12.1-01** | **DB Update on Startup** | - Remote has new character "Mavuika". | 1. Launch app.             | - `reqwest` fetches manifest.<br>- New character downloaded.<br>- "Mavuika" appears in filter list. | DB version matched. | High     |
| **TC-12.1-02** | **Asset Fetch (CDN)**    | - "Geo Icon" missing locally.         | 1. Open menu needing icon. | - App fetches from GitHub CDN.<br>- Icon appears after download.                                    | Cached.             | Medium   |
| **TC-12.1-03** | **ETag Caching**         | - Data already up-to-date.            | 1. Launch app.             | - `If-Modified-Since` / `ETag` sent.<br>- Server: 304 Not Modified.<br>- No download.               | Efficient.          | Medium   |

### US-12.2: App Binary Update

| ID             | Title                 | Pre-Condition                   | Steps                         | Expected Result                                                                     | Post-Condition | Priority |
| :------------- | :-------------------- | :------------------------------ | :---------------------------- | :---------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-12.2-01** | **Check & Install**   | - New version on update server. | 1. Click "Check for Updates". | - New version found.<br>- Download progress bar shown.<br>- Install → App restarts. | New version.   | High     |
| **TC-12.2-02** | **Download Progress** | - Large update (50MB+).         | 1. Start update.              | - Progress: "Downloading... X/Y MB".<br>- Percentage and speed shown.               | Completed.     | Medium   |

---

## 2. Negative Test Cases (Error Handling)

### US-12.1: Network Errors

| ID             | Title                 | Pre-Condition                | Steps            | Expected Result                                                                        | Post-Condition | Priority |
| :------------- | :-------------------- | :--------------------------- | :--------------- | :------------------------------------------------------------------------------------- | :------------- | :------- |
| **NC-12.1-01** | **Offline**           | - No internet.               | 1. Launch app.   | - Update check fails silently.<br>- Log: "Network Error".<br>- App continues normally. | Functional.    | High     |
| **NC-12.1-02** | **404 / Server Down** | - Bad URL or server offline. | 1. Check update. | - Log: "Endpoint Unreachable".<br>- App continues.                                     | Functional.    | Medium   |
| **NC-12.1-03** | **Rate Limited**      | - GitHub API rate limit hit. | 1. Check update. | - Retry with exponential backoff.<br>- Max 3 retries.<br>- Log details.                | Handled.       | Medium   |

### US-12.2: Update Errors

| ID             | Title                     | Pre-Condition      | Steps              | Expected Result                                                                      | Post-Condition | Priority |
| :------------- | :------------------------ | :----------------- | :----------------- | :----------------------------------------------------------------------------------- | :------------- | :------- |
| **NC-12.2-01** | **Signature Verify Fail** | - Tampered binary. | 1. Install update. | - Error: "Security: Invalid Signature".<br>- Delete temp files.<br>- Update aborted. | Abort.         | High     |

---

## 3. Edge Cases & Stability

| ID           | Title                       | Simulation Step                       | Expected Handling                                                          | Priority |
| :----------- | :-------------------------- | :------------------------------------ | :------------------------------------------------------------------------- | :------- |
| **EC-12.01** | **Partial Download**        | 1. Cut network at 50%.                | - Checksum fails.<br>- Retry on next launch.<br>- Temp files cleaned.      | High     |
| **EC-12.02** | **Update Loop**             | 1. Version 1.0 → Update to 1.0 (bug). | - Version compare: `remote > local` only.<br>- Prevent infinite loop.      | High     |
| **EC-12.03** | **Disk Full During Update** | 1. 0 space available.                 | - Error: "IO Error".<br>- Clean partial download.                          | Medium   |
| **EC-12.04** | **Debug Build Skip**        | 1. Running v0.0.0 (dev).              | - Update logic ignores dev builds.<br>- Never overwrite dev features.      | Medium   |
| **EC-12.05** | **Bandwidth Limit**         | 1. Update > 50MB.                     | - User prompted to confirm before download.<br>- Cancel returns to normal. | Medium   |

---

## 4. Technical Metrics

| ID           | Metric                  | Threshold                 | Method                                             |
| :----------- | :---------------------- | :------------------------ | :------------------------------------------------- |
| **TM-12.01** | **Startup Check**       | **< 500ms**               | Metadata check added to startup.                   |
| **TM-12.02** | **Download Efficiency** | **0 redundant downloads** | `ETag` / `If-Modified-Since` prevents re-download. |

---

## 5. Data Integrity

| ID           | Object                | Logic                                                                              |
| :----------- | :-------------------- | :--------------------------------------------------------------------------------- |
| **DI-12.01** | **Ed25519 Signature** | Public key pinned in app. Only signed binaries accepted by `tauri-plugin-updater`. |
| **DI-12.02** | **HTTP Client**       | All network requests use `reqwest` crate (per TRD). No other HTTP clients.         |
| **DI-12.03** | **Background Fetch**  | All network I/O runs in `tokio` async tasks. Never blocks main thread.             |
