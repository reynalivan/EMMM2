# Test Cases: App Updater & Asset Sync (Epic 34)

## A. Requirement Summary

- **Feature Goal**: Provide two discrete update pipelines: (1) App Binary Updates (via GitHub Releases + Tauri Plugin Updater v2). (2) Dynamic Asset Sync (via`reqwest` fetching`schema.json` payloads without requiring app patches).
- **User Roles**: Application User.
- **Acceptance Criteria**:
 - Binary update checks`<5s` non-blocking at startup.
 - Download streamed above 1 update/sec rate with size > 50MB prompting.
 - Ed25519 hash validation aborts corrupted/tampered payloads.
 - Asset Sync (`schema.json`) downloads quickly`<3s` using ETag caching.
 - Graceful disconnection backoffs up to 3 tries, falling back natively to Bundled Schemas.
 - Phase 5: Tauri v2 Updater implementation fully validated.
 - Phase 5: Release Channels (Stable vs Beta) toggle functional.
 - Phase 5: Changelog Injection renders markdown in UI.
- **Success Criteria**: Updates are seamless and never corrupt the application. Asset syncs keep game schemas up to date automatically without needing full app updates.
- **Main Risks**: Updating the binary while the DB is being actively written to could cause corruption. Updating fails due to missing Admin privileges.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-34-app-updater.md`

| Acceptance Criteria | Covered by TC IDs |
| :---------------------------------- | :---------------- |
| AC-34.1.1 (Background Checking) | TC-34-001 |
| Phase 5: Changelog Injection | TC-34-002 |
| AC-34.1.3 (Stream Payload Install) | TC-34-003 |
| AC-34.1.4 (Explicit Size Prompt) | TC-34-004 |
| AC-34.1.5 (Network Disconnect) | TC-34-005 |
| AC-34.1.6 (Tampered Signature Deny) | TC-34-006 |
| AC-34.1.7 (Closure Mid-Stream) | TC-34-007 |
| AC-34.1.8 (Debug Guard Shield) | TC-34-008 |
| AC-34.2.1 (ETag Startup Check) | TC-34-009 |
| AC-34.2.2 (Missing Schema Pull) | TC-34-010 |
| AC-34.2.4 (Parallel Sync Action) | TC-34-011 |
| AC-34.2.5 (Rate Limit Backoff) | TC-34-012 |
| AC-34.2.6 (Offline Bundled Grace) | TC-34-013 |
| AC-34.2.7 (JSON Corrupt Deny) | TC-34-014 |
| Phase 5: Tauri v2 Updater Validate | TC-34-015 |
| Phase 5: Release Channels (Beta) | TC-34-016 |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :----------------------------- | :------- | :------- | :--------------- | :----------------------------------------------------------------------------------------- | :----------------------------------------------------------------- | :-------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------ | :-------- |
| TC-34-001 | Background Version Check | Positive | High | S1 | App is connected to the internet. Remote endpoint points to`v9.9.9`. Current is`v1.0.0`. |`v9.9.9` release manifest. | 1. Launch EMMM2. | Background process checks payload`<5s`. Sends UI event displaying "Update Available" TopBar toast without freezing app interaction. | AC-34.1.1 |
| TC-34-002 | Phase 5: Changelog Injection | Positive | Medium | S2 | Update contains a detailed Markdown changelog release note. | Navigate into "Settings > Maintenance". | 1. Click "Check for Updates". | Fetch resolves GitHub releases, injecting the localized Markdown notes safely into the DOM UI`<5s` (Phase 5). | Phase 5 |
| TC-34-003 | Install Relaunch Binary Flow | Positive | High | S1 | A valid update is available (e.g.,`15MB`). | Valid`.msi` or`.exe` updater. | 1. Proceed "Download and Install".<br>2. Watch Progress.<br>3. Hit Restart. | Modal outputs streaming integers > 1/sec. Concludes, invoking Tauri's`process.relaunch()`. | AC-34.1.3 |
| TC-34-004 | Giant Size Prompt Wall | Edge | Medium | S2 | Update tagged size is significantly large (`> 50MB`). | Update size`60MB`. | 1. Click "Download and Install". | Catch logic triggers explicit warning modal "Update is 60MB. Download now?" gating bandwidth. | AC-34.1.4 |
| TC-34-005 | Offline Network Graceful Catch | Negative | High | S1 | PC Wi-Fi forced OFF / Firewall blocking port. | Offline Environment. | 1. Open App. | Check gracefully times out fetching payloads, sending a harmless string "check your connection" without panicking the Rust core. | AC-34.1.5 |
| TC-34-006 | Signature Fails Validation | Negative | Critical | S1 | Server supplies Corrupted`.zip`/`.msi` that breaks expected`Ed25519` pubkey. | Tampered binary payload. | 1. Download Corrupt update.<br>2. Wait for verification logic. | Target validation absolutely rejects the binary replacing it with an`Update Failed: Invalid Signature` toast. Temporary partial bytes are purged. | AC-34.1.6 |
| TC-34-007 | Closure Interruption State | Edge | Low | S3 | User closes the application midway through an update download (50%). | Half-downloaded temp file. | 1. Close Window actively during download phase. | Gracefully nullifies temp download`.tmp` chunk. Original binary explicitly reserved fully. | AC-34.1.7 |
| TC-34-008 | Dev Tool`v0.0.0` Guarding | Edge | Medium | S2 | Running a compiled dev build tagged as`v0.0.0`. | Dev build. | 1. Run Check Updates. | Logic fully aborts verification directly saving Dev directories protecting builds from auto-replacing themselves via Production payloads. | AC-34.1.8 |
| TC-34-009 | ETag Conditional Fetch | Positive | High | S1 | Latest Schema is already downloaded. App is restarted. | Identical ETags. | 1. Boot 1st time.<br>2. Boot 2nd time targeting`check_manifest()`. | ETag verifies exact HTTP header`<500ms`. 0 bytes are downloaded internally, saving Github rate limits entirely. | AC-34.2.1 |
| TC-34-010 | Fast Target Schema Fetch | Positive | High | S1 | The active game is missing its local`schema.json`. | Unconfigured target game. | 1. Select Game identifier. | Fetches raw payload directly hitting cache`app_data_dir/assets/{id}` in`< 3s`. Game functionality unlocks. | AC-34.2.2 |
| TC-34-011 | Parallel "Sync Assets" Action | Positive | Medium | S2 | 4 Games are actively registered in the App. | Multiple schema targets. | 1. Click "Sync Assets" manually in settings. |`reqwest` threadpool dispatches parallel requests checking ETags instantly reporting`{Ok, Cached, Failed}` array response. | AC-34.2.4 |
| TC-34-012 | HTTP 429 Exponential Retry | Edge | High | S1 | GitHub API rate limit is reached (HTTP 429). | HTTP 429 Response. | 1. Request Schema update. | Backoff natively initiates`1s -> 2s -> 4s` maximum. Bails dropping error string safely without a hard panic. | AC-34.2.5 |
| TC-34-013 | Bundled Offline Fallback | Edge | High | S1 | PC is disconnected. No downloaded Cache found. | Offline + Clean Slate. | 1. Click Unconfigured Target Game without internet. | Attempts internet, catches fail, instantly deploys the physical internal rust-embedded binary`resources/schemas/{id}_fallback.json` yielding full operation. | AC-34.2.6 |
| TC-34-014 | Json Syntax Corruption Denied | Negative | Critical | S1 | CDN pushed a bad schema`<HTML>` String text payload randomly instead of JSON. | Bad HTML payload. | 1. Trigger Schema Sync. |`serde_json` strongly catches invalid typing securely rejecting bytes entirely, leaving prior cached valid`.json` functional unharmed. | AC-34.2.7 |
| TC-34-015 | Phase 5: Tauri v2 Updater | Positive | High | S1 | Valid remote update exists. | Standard Tauri v2 manifest (`emmm2-v1.1.0-x86_64-setup.nsis.zip`). | 1. Trigger download and watch network traffic. | Implementation strictly conforms to the Tauri v2`updater` plugin specification. Download completes and extracts. | Phase 5 |
| TC-34-016 | Phase 5: Release Channels | Positive | Medium | S2 | App is configured for "Beta" channel. Server has`v1.1.0-beta.1` available. | Channel config`beta`. | 1. Change Update Channel to "Beta" in Settings.<br>2. Run Check. | App fetches the beta release manifest instead of the stable one. UI prompts to install the Beta build. | Phase 5 |

## D. Missing / Implied Test Areas

- **[Implied] Asset File Safety Permissions**: Does application path`%AppData%` possess writing permissions outside standard execution directory rights? (Tauri handles`app_data_dir()`).
- **[Implied] Update Version Semver Format Check**: Validate handling`1.0.10` >`1.0.9`, verifying exact Semver comparator mechanics structurally works.

## E. Open Questions / Gaps

- **Installer Permissions**: On Windows, does the`.nsis` string require UAC elevation prompt blindly, and if the user hits "No", does the Tauri app catch the failure correctly?

## F. Automation Candidates

- **TC-34-012 & TC-34-014**: Unit Testing mocking`Reqwest` injecting simulated rate limit`429` responses measuring time-bound looping asserts ensuring Threading operates bounds.
- **TC-34-006**: Rust level invocation testing manipulating physical Public Key strings injecting`tauri` updater payloads ensuring mathematical failures always strictly abort writes.

## G. Test Environment Setup

- **Mock Updater Endpoint**: Spin up a local static server serving a dummy GitHub Release JSON payload indicating version`v9.9.9`. Serve a binary payload that deliberately fails the Ed25519 signature validation test.
- **CDN Target**: Configure hosts file or internal Rust logic mapping`raw.githubusercontent` CDN routes to a controlled test endpoint to simulate HTTP 429 Rate Limits and Malformed JSON payloads.
- **Release Channels**: Setup endpoints for both`/stable/latest.json` and`/beta/latest.json` to verify channel swapping.

## H. Cross-Epic E2E Scenarios

- **E2E-34-001 (Seamless Schema Update Pipeline)**: A user launches EMMM2 with active internet. The Rust backend silently pings the schema endpoint using`reqwest` and sends the local ETag. The server responds with`200 OK` indicating a new`Genshin` schema update (`req-34`). The background task downloads the`<1MB` payload, validates it using`serde_json`, and hot-swaps the underlying schema cache. Simultaneously, the App Updater pings the GitHub Releases API (Tauri v2 Plugin Updater) and discovers`v1.1.0` is available. A clean TopBar Toast fires indicating "Update Available". The user clicks the toast, opening the settings modal. They read the injected Markdown changelog. They click "Install". The binary streams, validates the Ed25519 signature, and prompts to restart. The app closes and re-opens on the new version, with the new game schema automatically applied, entirely preserving user data.
