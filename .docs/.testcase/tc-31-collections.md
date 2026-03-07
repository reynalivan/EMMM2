# Test Cases: Virtual Collections (Epic 31)

## A. Requirement Summary

- **Feature Goal**: Allow users to save their currently enabled mods as a named "Collection" (loadout) and apply it instantly with a single button. Includes atomic apply machinery allowing snapshotting and reverting (undo).
- **User Roles**: Application User.
- **User Story**: As a user, I want to swap between entirely different mod setups (i.e. 'Streaming Mode' vs 'NSFW Mode') without manually clicking each folder.
- **Acceptance Criteria**:
 - Saved collections track currently enabled folders natively in the SQLite Database.
 - Apply runs under`OperationLock` +`WatcherSuppression`, completing 100 mods in ≤ 5s.
 - Snapshot logic rolls back changes automatically on a failure midway.
 - Undo restores back the previous snapshot on demand in ≤ 5s.
 - Missing mods trigger warnings instead of total failure.
 - Phase 5: Smart Conflict Resolution (auto-disables conflicting mods during application).
 - Phase 5: Double ID Tracing (folder_hash BLAKE3 auto-heal if folder was renamed).
 - Phase 5:`is_safe_context` Awareness (warns if applying a collection with NSFW mods while Safe Mode is ON).
 - Phase 5: Portable Persistence (collections exportable/importable).
- **Success Criteria**: Atomic operations never cause DB desyncs. Rollbacks strip back modified states.
- **Main Risks**: Edge cases involving externally deleted/locked folders crashing mid-renaming, forcing snapshot rollback reliability tests.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-31-collections.md`

| Acceptance Criteria | Covered by TC IDs |
| :------------------------------ | :---------------- |
| AC-31.1.1 (Save Creation) | TC-31-001 |
| AC-31.1.2 (Save Naming) | TC-31-002 |
| AC-31.1.3 (Empty Name Valid) | TC-31-003 |
| AC-31.1.4 (Empty Collection) | TC-31-004 |
| AC-31.2.1 (Atomic Apply) | TC-31-005 |
| AC-31.2.2 (React Query Refresh) | TC-31-006 |
| AC-31.2.3 (Mid-fail Rollback) | TC-31-007 |
| AC-31.2.4 (Missing Folder skip) | TC-31-008 |
| AC-31.3.1 (Undo Button) | TC-31-009 |
| AC-31.3.2 (Snapshot Overwrite) | TC-31-010 |
| Phase 5: Smart Conflict Res. | TC-31-011 |
| Phase 5: Double ID Tracing | TC-31-012 |
| Phase 5:`is_safe_context` | TC-31-013 |
| Phase 5: Portable Persistence | TC-31-014 |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :------------------------- | :------- | :------- | :--------------- | :------------------------------------------------------------------------------------------------- | :--------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------- |
| TC-31-001 | Save Active Loadout | Positive | High | S1 | 10 Mods are currently enabled in the grid. |`UserPresetA` | 1. Click "Save Current State" in the Collections UI.<br>2. Name it "UserPresetA".<br>3. Save. |`collections` DB is updated.`collection_items` records 10 paths precisely. Toast confirms save. | AC-31.1.1 |
| TC-31-002 | Name Bounds constraints | Edge | Medium | S3 | Valid enabled mods are present. | Name: 128 Char String | 1. Open Save Modal.<br>2. Type exactly 128 characters into the name field.<br>3. Click Save. | binds the label to the loadout inside`≤ 300ms`. The long name truncates visually with an ellipsis in the UI list. | AC-31.1.2 |
| TC-31-003 | Block Empty String names | Negative | High | S2 | Save Modal is open. | Empty Name Input | 1. Clear the Name Input entirely.<br>2. Attempt to click 'Save'. | Form validation blocks submit. Submit button is disabled. Helper text shows "Name required". | AC-31.1.3 |
| TC-31-004 | Empty Loadout saving | Positive | Medium | S3 | Exactly 0 Mods are enabled currently. |`VanillaState` | 1. Save Current State.<br>2. Name it "VanillaState". | Empty collection saved. Applying this later will disable all currently active mods (returning to Vanilla). | AC-31.1.4 |
| TC-31-005 | Apply Atomic execution | Positive | High | S1 | A layout of 100 mods exists as a saved Preset. The current grid has a mismatched setup. | 100 Mod Preset | 1. Select the 100 Mod Preset.<br>2. Click "Apply". | Old mods disable, preset mods enable. Handled using pure diff operations under`OperationLock` in < 5s. DB state matches FS state. | AC-31.2.1 |
| TC-31-006 | Apply trigger visual cache | Positive | Medium | S2 | A Preset exists. | N/A | 1. Apply a preset.<br>2. Observe the UI Object List active counts in the sidebar. | ObjectList active counts refresh accurately tracking the new preset values exclusively without requiring an app reload. | AC-31.2.2 |
| TC-31-007 | Mid-flight rename error | Negative | High | S1 | A preset of 50 mods exists. | Lock a preset mod folder using OS File handles | 1. Using another program, lock one of the target mod folders (so it cannot be renamed).<br>2. Apply the Preset in EMMM2. | Fails on the locked file. Triggers Rollback sequence reversing any previous renames from this batch. UI Toasts "Apply failed: file locked, changes reverted". | AC-31.2.3 |
| TC-31-008 | Missing folder skip logic | Edge | High | S2 | A mapped mod inside a saved Collection was deleted via Windows Explorer. | One mapped mod deleted externally | 1. Apply the known Collection.<br>2. Observe UI and Toasts. | Warns the user about 1 missing path ("1 mod could not be found"). The remaining 99 applied. DB ignores the missing one. | AC-31.2.4 |
| TC-31-009 | Success Undo action | Positive | High | S1 | User just applied a setup. The 10s interactive "Undo" Toast is visible. | Undo Toast Present | 1. Quickly click the "Undo" button on the Toast before it expires. | Returns the exact visual and physical folder enabled/disabled layout back to the original state right before the apply action. | AC-31.3.1 |
| TC-31-010 | Discard Old Undo State | Edge | Low | S3 | Two presets exist (Preset A, Preset B). | Multi-Apply | 1. Apply Preset A.<br>2. Apply Preset B before the toast expires.<br>3. Click Undo on the 2nd toast. | Only reverts back to Preset A's state (the state immediately prior to applying Preset B). Snapshot table keeps solely the latest event. | AC-31.3.2 |
| TC-31-011 | Smart Conflict Resolution | Positive | Medium | S2 | Preset tries to enable`Mod A` which conflicts with`Mod B` (already magically enabled). | Conflict preset | 1. Apply the collection.<br>2. Ensure the engine detects the runtime conflict. | The new incoming preset logic natively disables`Mod B` to make room for`Mod A` instead of throwing a generic DB error. | Phase 5 |
| TC-31-012 | Double ID Tracing Healing | Edge | High | S1 | Preset requires`ModFolder_123`. User renamed physical folder to`ModFolder_ABC`. | BLAKE3 tracked folder | 1. Rename physical folder.<br>2. Apply the collection that mapped to the old folder path. | The backend realizes`ModFolder_123` is missing, checks its BLAKE3`folder_hash` against known folders, finds`ModFolder_ABC`, auto-heals the DB entry, and applies it. | Phase 5 |
| TC-31-013 |`is_safe_context` Warning | Negative | High | S1 | Safe Mode is ON. Collection`NSFW Loadout` contains 50`is_safe=false` mods. | NSFW preset | 1. Attempt to apply the explicitly unsafe collection while Safe Mode is ON. | Backend denies the request entirely or prompts a giant warning text: "This collection contains hidden NSFW mods. You must disable Safe Mode first." | Phase 5 |
| TC-31-014 | Portable Persistence | Positive | Medium | S2 | Valid Collection exists. | Export JSON | 1. Click Export on the Collection.<br>2. Delete the Collection in DB.<br>3. Import the generated JSON file. | The Collection is restored fully matching the exported file. JSON schema validation succeeds. | Phase 5 |

## D. Missing / Implied Test Areas

- **[Implied] Watcher Conflict**: Does applying generate`fs-changed` events? No.`WatcherSuppression` guards it until the batch triggers complete cache revalidation.
- **[Implied] Safe Mode Exclusions**: Ensure that Collection apply does NOT leak Safe Mode`is_safe=false` thumbnails during rapid re-rendering grids (handled by Phase 5`is_safe_context` Warning).

## E. Open Questions / Gaps

- No specific questions remain.

## F. Automation Candidates

- **TC-31-005, TC-31-007**: Core Engine Integration Test utilizing mocked filesystem with dummy nodes mimicking atomic bulk swap patterns guaranteeing failure rollback assertions exactly returning boolean`true`.
- **TC-31-012**: Backend Unit Test to verify BLAKE3 auto-healing resolution paths when`find_by_path` returns`None` but`find_by_hash` succeeds.

## G. Test Environment Setup

- **Logical DB Presets**: Preload`collections` and`collection_items` tables with 5 distinct mock collections mapping to valid DB mod paths.
- **Physical Grid State**: Initialize a realistic nested physical folder structure across`Characters`,`Weapons`, and`UI` with at least 150 dummy valid`.ini` files to serve as load targets.

## H. Cross-Epic E2E Scenarios

- **E2E-31-001 (Preset Application Auto-Heal & Safe Mode)**: The user renames 3 physical folders via Windows Explorer outside of the app. The user opens the app, ensures Safe Mode is OFF, and attempts to apply "Main Streaming Preset" which referenced those old names. The "Double ID Tracing" kicks in, resolving the new physical paths instantly without failure. The DB updates mappings silently. The UI refreshes the FolderGrid (Epic 12) showing the correct enabled state across 60 items. The user then turns Safe Mode ON (Epic 30) and attempts to apply an NSFW collection, which throws a block warning preventing accidental stream leaks.
