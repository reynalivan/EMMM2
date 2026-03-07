# Test Cases: Privacy & Safe Mode (Epic 30)

## A. Requirement Summary

- **Feature Goal**: Protect user library visibility containing sensitive/NSFW graphics from public viewing via a global Safe Mode CSS-filter screen. Restrict un-toggling Safe Mode with a Backend-verified PIN layer.
- **User Roles**: Application User.
- **User Story**: As a user, I want a quick, instant Safe Mode so I can use the App on stream, and lock it so others cannot pry into my hidden mods.
- **Acceptance Criteria**:
 - Global`safeMode` Zustand store triggers visual mask CSS`blur(12px)` + Name string`[Hidden Mod]`.
 - Object list counts automatically subtract`is_safe=false` mod count from UI when active.
 - Rate limited PIN gate backend with secure storage (Phase 5 requires Argon2 implementation lockout).
 - Toggling`is_safe` on mod applies globally in ≤ 200ms instantly.
 - Mode persistence via`store.json`.
 - Phase 5: Auto-classification on import (inherits object safety).
 - Phase 5: Dual Guard (`is_safe = true` explicit queries).
 - Phase 5: Portable Consistency (saving to`info.json`).
- **Success Criteria**: No DB query leakage, visual blur immediately applied without network calls. PIN rate limiting prevents automated backend guessing.
- **Main Risks**: Accidental count leakages through background queries caching. Forgetting PIN entirely wiping user access to Safe Mode disable.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-30-privacy-safe-mode.md`

| Acceptance Criteria | Covered by TC IDs |
| :----------------------------------- | :---------------- |
| AC-30.1.1 (Store Toggle Persistence) | TC-30-001 |
| AC-30.1.2 (Instant UI Masking) | TC-30-002 |
| AC-30.1.3 (Object List Subtraction) | TC-30-003 |
| AC-30.1.4 (PIN Dialog Catch) | TC-30-004 |
| AC-30.1.5 (App Boot Persistence) | TC-30-005 |
| AC-30.2.1 (Context Mod toggle) | TC-30-006 |
| AC-30.2.2 (Instant Mod toggle) | TC-30-007 |
| AC-30.2.3 (Mode priority) | TC-30-008 |
| AC-30.3.1 (Valid PIN DB Write) | TC-30-009 |
| AC-30.3.2 (Valid unlock) | TC-30-010 |
| AC-30.3.3 (Rate limit throttle) | TC-30-011 |
| AC-30.3.4 (Invalid visual feedback) | TC-30-012 |
| AC-30.3.5 (No-PIN Confirmation) | TC-30-013 |
| Phase 5: Auto-classification | TC-30-014 |
| Phase 5: Portable Consistency | TC-30-015 |
| Phase 5: Argon2 Lockout | TC-30-016 |
| Phase 5: Dual Guard Explicit Query | TC-30-017 |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :----------------------------------- | :------- | :------- | :--------------- | :--------------------------------------------------------------------------------------------------- | :-------------------------------------- | :------------------------------------------------------------------------------------------------------ | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------- |
| TC-30-001 | Toggle Safe Mode Off to On | Positive | High | S1 | Safe Mode is Off, App is loaded, no PIN configured. |`safeMode=false` | 1. Click the Shield token in the TopBar. | Instantly applies`boolean -> true`. Saved via`@tauri-apps/plugin-store`. Shield icon glows active. | AC-30.1.1 |
| TC-30-002 | Visual Mask logic | Positive | High | S1 | Grid showing a mix of`is_safe` true and false mods. | Mix of safe/NSFW mods. | 1. Ensure FolderGrid is visible.<br>2. Toggle Safe Mode ON. | For NSFW mods: Thumbnail filters active (`blur(12px)`). Mod name string is replaced with`[Hidden Mod]`. The actual name and image are totally unreadable. | AC-30.1.2 |
| TC-30-003 | ObjectList Count Subtraction | Positive | Medium | S2 | Object has 3 Total Enabled mods, 2 of them are marked`is_safe=false`. | Object counts (1/3) | 1. Enable Safe Mode.<br>2. Observe the Object Sidebar counts. | ObjectList intelligently drops its displayed count from 3 to 1 to conceal the existence of the hidden ones. | AC-30.1.3 |
| TC-30-004 | Shield Token Blocked by Pin | Negative | High | S1 | Security PIN set in Settings. Safe Mode is ON. |`PIN matches 8828` | 1. Click the Shield token to disable Safe Mode. |`PinEntryModal` captures the UI. Safe Mode stays Active. Blur is preserved. | AC-30.1.4 |
| TC-30-005 | Persistence App Restart | Edge | Medium | S2 | Safe Mode is currently active in Zustand state. | App Config | 1. Close the App entirely.<br>2. Restart the App. | App launches reading`store.json`. Blur is applied immediately upon first render of the Grid, preventing any 1-frame flashes of NSFW content. | AC-30.1.5 |
| TC-30-006 | Target mod DB Safety toggling | Positive | Medium | S1 | Mod card visible on Grid. | Safe mode currently OFF. | 1. Right-click card to open Context Menu.<br>2. Click "Mark as NSFW". | Mod UPDATE query applies inside < 200ms. Grid caches invalidate, adding the NSFW flag. | AC-30.2.1 |
| TC-30-007 | Instancing target during Active Mode | Positive | Medium | S2 | Safe Mode currently ON globally. User is looking at a safe mod. | Safe mode ON. | 1. Right-click a "Safe" mod.<br>2. Click "Mark as NSFW". | The mod's card instantly goes blurry over the live UI, replacing the name to`[Hidden Mod]`. | AC-30.2.2 |
| TC-30-008 | Marking Safe under Active state | Edge | Low | S3 | Safe Mode is ON. User selects a`[Hidden Mod]`. | NSFW Mod. | 1. Right-click the blurred card.<br>2. Reveal "Mark Safe". | Does _not_ automatically un-blur the mod thumbnail immediately if global lock is evaluating strict safety constraints. Or if it does unblur, it does so. | AC-30.2.3 |
| TC-30-009 | Set PIN via Interface | Positive | High | S1 | User is inside Settings -> Privacy. | String: "8828" | 1. Enter a valid 4-8 digit numeric PIN.<br>2. Click Apply/Save. | PIN is hashed via Argon2 (Phase 5) and saved to`store.json`. Success toast appears. | AC-30.3.1 |
| TC-30-010 | Shield Unlock verified DB | Positive | High | S1 | Shield Blocked Modal is open. | Correct Input`8828` | 1. Enter the correct PIN.<br>2. Push 'Submit'. | Prompt closes. Global Safe Mode boolean transitions to`false`. Filter masks drop revealing real names and images. | AC-30.3.2 |
| TC-30-011 | Throttle brute force | Negative | High | S1 | Shield Blocked Modal is open. | Random wrong inputs. | 1. Enter the WRONG PIN intentionally.<br>2. Repeat 3 times. | Application locks out the entry logic. Displays "Try again in Xs" countdown. Argon2 implementation enforces mandatory minimum computational delay preventing floods. | AC-30.3.3 |
| TC-30-012 | Shake feedback | Negative | Medium | S3 | Shield Blocked Modal is open. | Incorrect String`1111` | 1. Enter wrong PIN once. | Modal reacts via UI Animation visually (`keyframes shake`) + error text appears. | AC-30.3.4 |
| TC-30-013 | PIN deleted confirmation gate | Edge | Low | S3 | PIN is deleted in settings. Shield clicked while active. | Empty Auth. | 1. Delete PIN string in Settings.<br>2. Return to Dashboard.<br>3. Click Shield. | Requests simple validation: "Are you sure you want to disable Safe Mode?" allowing bypass logically since no PIN exists. | AC-30.3.5 |
| TC-30-014 | Auto-classification on Import | Edge | Medium | S2 | An Object Category (e.g.,`Characters/HentaiList`) is centrally flagged as`is_safe=false` in Setup. | New Mod Archive targeting`HentaiList`. | 1. Drag and drop import the archive.<br>2. Ensure classification maps it to the unsafe Object Category. | Mod is automatically assigned`is_safe = false` upon ingestion in the DB because it matches an unsafe Object Category. | Phase 5 |
| TC-30-015 | Portable Consistency (info.json) | Positive | Medium | S2 | Mod is marked NSFW via UI. | Valid Mod on Disk. | 1. Mark Mod as NSFW.<br>2. Open its physical`info.json` in VSCode. | The Database UPDATE query is accompanied by a physical FS write setting`"is_safe": false` inside the physical metadata, ensuring portability if the DB is destroyed. | Phase 5 |
| TC-30-016 | Argon2 Lockout Implementation | Edge | High | S1 | Malicious script attempts to call`verify_pin` via Tauri IPC. | Script calling backend 100x/sec. | 1. Execute rapid IPC loop guessing PINs. | The backend Argon2 hashing naturally caps verifications based on computational time (e.g., ~200ms per check), hard-blocking brute force IPC spam attempts. | Phase 5 |
| TC-30-017 | Dual Guard Explicit Query | Positive | High | S1 | Custom Object API query fetches list of mods. | Safe Mode ON. | 1. Trigger`get_mods_for_object` while Safe Mode is ON.<br>2. Inspect network payload. | Rust Backend explicitly appends`AND is_safe = true` to the SQL query. The client never even receives the JSON payload containing the NSFW mod metadata, ensuring zero network leakage. | Phase 5 |

## D. Missing / Implied Test Areas

- **[Implied] Bulk Edit Safe Toggle**: Ensure if a user selects 5 mods using Shift-Click + "Mark as NSFW", the query batches updates without issue, blurring them.
- **[Implied] Randomization Integrity**: Ensure Epic 35 ignores pulling from pool mapping if a hidden`is_safe=false` object is drawn during safe mode randomization.

## E. Open Questions / Gaps

- No specific questions remain. The Safe Mode filter works.

## F. Automation Candidates

- **TC-30-001, TC-30-005**: Load E2E suite via Playwright checking exact CSS`filter:` values on rendered nodes against`FolderCard` classes when Zustand mocks`true`.
- **TC-30-009**: Server rust tests asserting`argon2` matches output format precisely without text collision. Ensure computation bounds are respected.

## G. Test Environment Setup

- **Security Mocks**: Establish an Argon2 hashed representation of`8828` inside`store.json` for backend DB retrieval.
- **Mixed Content Grid**: Generate a mock database table containing 10 objects. Flag 4 of them explicitly to`is_safe=false`.

## H. Cross-Epic E2E Scenarios

- **E2E-30-001 (Safe Mode Stream Layout Pipeline)**: The user opens the App with Safe Mode enabled (Epic 30) and a PIN configured. The user navigates across the Dashboard (Epic 33), generating no leakages of un-safe content. The user opens the "NSFW Characters" Object List (Epic 06), and the Folder Grid renders 0 items because the Backend explicitly blocks loading them under Dual Guard. The user clicks Top Bar shield and inputs their 4-digit PIN. The UI organically unblurs active cards and triggers a data refetch to populate the missing NSFW cards directly from SQLite.
