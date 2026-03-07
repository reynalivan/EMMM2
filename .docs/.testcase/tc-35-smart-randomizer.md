# Test Cases: Smart Randomizer & Integrated Game Launcher (Epic 35)

## A. Requirement Summary

- **Feature Goal**: Allow users to instantly generate an automatic non-conflicting mod preset loadout (selecting maximum 1 mod uniquely per target Object ID slot matching criteria filter). Includes an Integrated Game Launcher feature using`sysinfo` process checking to open both 3DMigoto Loader and underlying Game EXE via Admin.
- **User Roles**: Application User.
- **Acceptance Criteria**:
 -`suggest_random_mods` returns instantly`<200ms` avoiding conflicting selections (2 mods selected simultaneously over 1 character).
 - Skips system folders (dot-prefixes).
 - Respects Safe Mode implicitly removing NSFW pools entirely.
 - Generates statistically varied outputs per sequential "Re-roll" execution.
 - "Play" button triggers Admin execution wrapped via`sudo` bounds loading Game Executables independently.
 - Phase 5: Randomizer Memory (Cache previous 3 rolls to avoid immediate repeats).
 - Phase 5: Seed Logging (Output generation seed to logs for reproducible debugging).
 - Phase 5: Category Weighting (Allow UI to specify "70% Characters, 30% Weapons").
- **Success Criteria**: Generated loadouts are immediately playable with zero conflicts. The game launcher reliably starts both components (loader + game) in the correct order.
- **Main Risks**: 3DMigoto Loader hanging if launched twice. UAC prompts interrupting the automated launch flow maliciously. Randomizer ignoring Safe Mode.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-35-smart-randomizer.md`

| Acceptance Criteria | Covered by TC IDs |
| :-------------------------------------- | :---------------- |
| AC-35.1.1 (Launch Sequence & Admin) | TC-35-001 |
| AC-35.1.2 (Auto-Close Post-Launch) | TC-35-002 |
| AC-35.1.3 (UAC Deny Handled) | TC-35-003 |
| AC-35.1.4 (Missing Launcher Warning) | TC-35-004 |
| AC-35.1.5 (Skip Pre-started Loader) | TC-35-005 |
| AC-35.2.1 (Pure Non-Conflict Selection) | TC-35-006 |
| AC-35.2.2 (Safe Mode Explicit Filter) | TC-35-007 |
| AC-35.2.3 (Preview Re-Roll Logic) | TC-35-008 |
| AC-35.2.4 (Bulk Apply Sequence) | TC-35-009 |
| AC-35.2.5 (Fully Excluded Object Skip) | TC-35-010 |
| AC-35.2.6 (Operation Lock Guard) | TC-35-011 |
| AC-35.2.8 (Full Library Exhaustion) | TC-35-012 |
| AC-35.3.1 (Dot Prefix Exclusion) | TC-35-013 |
| Phase 5: Randomizer Memory | TC-35-014 |
| Phase 5: Seed Logging | TC-35-015 |
| Phase 5: Category Weighting | TC-35-016 |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :------------------------------- | :------- | :------- | :--------------- | :----------------------------------------------------------------------- | :----------------------------- | :--------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------- |
| TC-35-001 | One-Click Execution Launcher | Positive | High | S1 | Valid Executable settings initialized in`settings.json`. | Game EXE and Loader EXE paths. | 1. Click "Play" on TopBar. |`sysinfo` determines loader is unopen. PowerShell invokes UAC triggering executable sequence perfectly`<100ms`. | AC-35.1.1 |
| TC-35-002 | Auto-Close EMMM2 Logic | Positive | Medium | S2 | "Auto-Close after launch" Setting is enabled. | Settings flag`auto_close`. | 1. Click "Play". | Application invokes external exe processes and calls Tauri`app.exit(0)` securely closing the mod manager entirely. | AC-35.1.2 |
| TC-35-003 | UAC Admin Denied Guard | Negative | High | S1 | Valid Executable mapped, but requires Admin. | Standard game executable. | 1. Click "Play".<br>2. Prompt window appears, click "No" / Deny. | Core detects permission rejection strictly outputting localized Toast string instructing "Admin rights required" without panicking Rust Core. | AC-35.1.3 |
| TC-35-004 | Missing Launcher File Target | Negative | Medium | S2 | Settings contain an invalid string targeting a deleted binary target. | Invalid path string. | 1. Click "Play". | Captures missing target file instantly routing UX into "Settings > Games" preventing silent unfunctional buttons. | AC-35.1.4 |
| TC-35-005 | Loader Re-Initialization Block | Edge | High | S1 | 3DMigoto Loader is actively already running on System Threading. |`3DMigoto Loader.exe` | 1. Ensure`Loader.exe` runs in Task Manager.<br>2. Click "Play". |`sysinfo` verifies active process skipping re-calling the Loader, executing primary target game exclusively. | AC-35.1.5 |
| TC-35-006 | Non-Conflicting Random Generator | Positive | High | S1 | Target "Albedo" has 5 unique Mod Variations physically mapped in DB. | 5 Valid Mods for Albedo. | 1. Open "Generate New Setup". | Modal evaluates choices extracting exactly 1 specific variant directly resolving Object assignment`<200ms`. No duplicate characters are selected. | AC-35.2.1 |
| TC-35-007 | Safe Mode Boundary Bypass | Positive | High | S1 | Safe Mode`ON`. "Raiden" possesses exactly 2 NSFW mods. | NSFW tagged mods. | 1. Run Generator instance. | Algorithm purges "Raiden" Object rendering safely resulting 0 explicit NSFW outputs globally. | AC-35.2.2 |
| TC-35-008 | Reroll Variable Distinction | Positive | Medium | S2 | Base array contains a large pool of valid mods. | DB of 50+ mods. | 1. Click "Re-roll" consecutive sequences. | Native`SliceRandom` invokes dynamic outputs preventing exact duplicated patterns resolving physically differing variants upon multiple clicks. | AC-35.2.3 |
| TC-35-009 | Bulk Apply Sync Binding | Positive | High | S1 | Modal mapping confirmed in UI via Preview. | Preview State Array. | 1. Tap "Apply This Setup". | Target executes bulk transaction using unified Epic 31 Apply parameters, handling`SuppressionGuards`, rendering the new physical layout. | AC-35.2.4 |
| TC-35-010 | Excluded Object Skip | Positive | Medium | S2 | All mods for "Zhongli" are marked as "Exclude from Randomizer". | DB flags`exclude_rng=true`. | 1. Generate Loadout. | "Zhongli" slot is skipped. No mods are magically flipped on for that category. | AC-35.2.5 |
| TC-35-011 | Current Operation Locking | Edge | High | S1 | Global Lock held by a background dedup scan. | Active`OperationLock`. | 1. Attempt Apply execution from Randomizer. | Click drops payload rendering Toast String precisely indicating Background processes operating strictly preventing DB mutations mid-action. | AC-35.2.6 |
| TC-35-012 | Complete Safe Mode Exhaustion | Edge | Low | S3 | Safe Mode active. Total local library contains exclusively NSFW content. | 100% NSFW Library. | 1. Open Generator modal payload. | UI gracefully displays generic text "No safe mods available to generate", avoiding infinite loop generation freezes. | AC-35.2.8 |
| TC-35-013 | System Folder Dot Exclusion | Positive | Medium | S2 |`.EMMM2_System` physical folder exists mapping system mods. | Dot-prefixed folder. | 1. Check generated pools. | Filter ignores strict dot-folders. | AC-35.3.1 |
| TC-35-014 | Phase 5: Randomizer Memory | Edge | Medium | S2 | User clicks Re-Roll 5 times rapidly. | 50 available mods. | 1. Click Re-Roll 5x.<br>2. Inspect variants per roll. | Cache stores the last 3 rolls in memory. The generation algorithm actively weights against selecting the exact same mod variant shown in the previous 3 rolls. | Phase 5 |
| TC-35-015 | Phase 5: Seed Logging | Positive | Low | S3 | User generates a loadout. | Normal generation. | 1. Generate.<br>2. Open`app.log`. | Backend outputs`Tracing RNG Seed: XXXXXXXX-XXXX` enabling developers to reproduce specific RNG layout grids for debugging purposes. | Phase 5 |
| TC-35-016 | Phase 5: Category Weighting | Positive | Medium | S2 | User configures RNG Settings: "Characters 100%, Weapons 0%". | Slider configs. | 1. Apply Settings.<br>2. Generate Setup. | Output entirely consists of Character mods. Zero weapon objects are mutated or explicitly assigned in the resulting payload layout (Phase 5). | Phase 5 |

## D. Missing / Implied Test Areas

- **[Implied] RNG Initialization**: Ensure`thread_rng()` seeds itself natively bypassing deterministic cache repetition errors (random array elements stuck generating duplicate patterns persistently based off fixed identical clock blocks). This is resolved via Phase 5 Seed Logging.

## E. Open Questions / Gaps

- No specific questions. The Phase 5 requirements address the "re-roll fatigue" where it would roll the exact same mod twice in a row.

## F. Automation Candidates

- **TC-35-006, TC-35-007, TC-35-016**: Pure database logic unit testing inserting structured row sets internally matching parameters confirming exact boundary filtering logic accurately extracting non-NSFW random slice elements.
- **TC-35-001**: Unit tests invoking isolated`sysinfo` mocks checking boolean`is_running` process checks bypass shell invocations.

## G. Test Environment Setup

- **Mock Launcher**: Configure`game.launcher_path` pointing to a local Windows batch script that does effectively nothing but exits simulating valid EXE location tracking.
- **Database Objects**: Populate`folders` with Characters: "Albedo" (5 Mods), "Raiden" (2 Mods - Both NSFW), "Hu Tao" (1 Mod). Include`.EMMM2_System` as a simulated dot-prefix locked object mapping.
- **Permissions**: Prepare to manually reject a UAC prompt when invoking "Play" testing Admin Elevation exceptions.

## H. Cross-Epic E2E Scenarios

- **E2E-35-001 (Randomizer to Game Launch)**: The user is bored with their current setup and navigates to the Smart Randomizer (Epic 35). They configure Category Weighting to 80% Characters and 20% UI (Phase 5). They ensure Safe Mode is securely engaged globally (Epic 30). They click "Generate". The Rust backend uses`thread_rng`, caches the memory (Phase 5), logs the seed, and returns a non-conflicting layout containing zero NSFW files. The user previews the layout in the UI, clicks "Re-Roll" once to swap an unwanted variant (memory caching ensures a new one appears), and clicks "Apply". The backend executes the Database Bulk Txn and physical folder renaming (Epic 11). Once complete, the user clicks the global "Quick Play" button on the TopBar (Epic 33). The application detects`3DMigoto Loader` is closed, triggers a UAC prompt via PowerShell, launches the Loader, starts`GenshinImpact.exe`, and then gracefully auto-closes EMMM2 natively releasing system resources.
