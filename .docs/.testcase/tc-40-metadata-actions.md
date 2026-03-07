# Test Cases: Metadata Actions — Pin, Favorite, Move (Epic 40)

## A. Requirement Summary

- **Feature Goal**: Three quick organizational actions on Objects and mod folders:`pin_object` (DB-only, objectlist ordering),`toggle_favorite` (DB-only, gold star + filter), and`move_mod_to_object` (filesystem rename +`info.json` patch + DB update under`OperationLock` +`WatcherSuppression`).
- **User Roles**: End User.
- **User Stories**:
 - US-40.1: Pin Object to top of objectlist.
 - US-40.2: Favorite a Mod Card for quick filtering.
 - US-40.3: Move Mod to Different Object.
- **Acceptance Criteria**:
 - AC-40.1.1: Pin sets`is_pinned = true`; Object moves to "Pinned" section within ≤ 150ms.
 - AC-40.1.2: Second pin click sets`is_pinned = false`; Object returns to default sort.
 - AC-40.1.3: All Objects pinned → layout renders without breaking.
 - AC-40.2.1: Favorite sets`is_favorite = true`; card shows gold star within ≤ 150ms.
 - AC-40.2.2: Grid "Favorites Only" filter shows only`is_favorite = true` cards.
 - AC-40.2.3: Un-favoriting while in Favorites filter removes card from view.
 - AC-40.3.1: Move physically relocates folder to`mods_path/{category}/{new_object}/{basename}`; updates`info.json` + DB atomically.
 - AC-40.3.2: Post-move grid updates within ≤ 200ms via`invalidateQueries`.
 - AC-40.3.3: Target path collision → abort before any FS operation; toast error.
 - AC-40.3.4: DB update fails after rename → rollback DB rollback + manual rename back; no orphan rows.
 - AC-40.3.5: Enabled mod moved → enabled state preserved (folder name prefix intact).
- **Success Criteria**: Pin/Favorite ≤ 150ms; Move ≤ 1s SSD / ≤ 5s network drive; 0 orphan DB rows on failure.
- **Main Risks**:`info.json` write fails mid-move leaving inconsistent mod metadata; DB+FS state diverge if rollback also fails; simultaneous pin of many objects in rapid succession causing UI reorder flicker.
---

## B. Coverage Matrix

| Acceptance Criteria | Covered by TC IDs | Requirement File |
| :------------------------------------ | :---------------- | :-------------------------------------------------------------- |
| AC-40.1.1 (Pin Object) | TC-40-001 |`e:\Dev\EMMM2NEW\.docs\requirements\req-40-metadata-actions.md` |
| AC-40.1.2 (Unpin Object) | TC-40-002 |`e:\Dev\EMMM2NEW\.docs\requirements\req-40-metadata-actions.md` |
| AC-40.1.3 (All pinned layout) | TC-40-003 |`e:\Dev\EMMM2NEW\.docs\requirements\req-40-metadata-actions.md` |
| AC-40.2.1 (Favorite card) | TC-40-004 |`e:\Dev\EMMM2NEW\.docs\requirements\req-40-metadata-actions.md` |
| AC-40.2.2 (Favorites filter) | TC-40-005 |`e:\Dev\EMMM2NEW\.docs\requirements\req-40-metadata-actions.md` |
| AC-40.2.3 (Unfavorite while filtered) | TC-40-006 |`e:\Dev\EMMM2NEW\.docs\requirements\req-40-metadata-actions.md` |
| AC-40.3.1 (Move to new object) | TC-40-007 |`e:\Dev\EMMM2NEW\.docs\requirements\req-40-metadata-actions.md` |
| AC-40.3.2 (Grid update post-move) | TC-40-008 |`e:\Dev\EMMM2NEW\.docs\requirements\req-40-metadata-actions.md` |
| AC-40.3.3 (Collision abort) | TC-40-009 |`e:\Dev\EMMM2NEW\.docs\requirements\req-40-metadata-actions.md` |
| AC-40.3.4 (DB fail + rollback) | TC-40-010 |`e:\Dev\EMMM2NEW\.docs\requirements\req-40-metadata-actions.md` |
| AC-40.3.5 (Enabled mod move) | TC-40-011 |`e:\Dev\EMMM2NEW\.docs\requirements\req-40-metadata-actions.md` |

---

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| :-------- | :-------------------------------------------- | :------- | :------- | :------------------------------------------------------ | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------- | :-------- |
| TC-40-001 | Pin Object to objectlist top | Positive | High | Object: "Keqing",`is_pinned = false` | 1. Ensure Genshin configured. Objects: "Keqing" (unpinned), "Amber", "Kaeya". ObjectList visible.<br>2. Open objectlist.<br>3. Find "Keqing" in the unpinned Object list.<br>4. Click the pin icon (📌) next to "Keqing".<br>5. Start stopwatch.<br>6. Observe objectlist reorganization.<br>7. Query DB:`SELECT is_pinned FROM objects WHERE name = 'Keqing'`. | Within ≤ 150ms, "Keqing" moves to a "Pinned" section at the top of the objectlist. DB`is_pinned = true`. This section is visually distinct from the unpinned list below. | S2 | AC-40.1.1 |
| TC-40-002 | Unpin already-pinned Object | Positive | High | Object: "Keqing",`is_pinned = true` | 1. Ensure "Keqing" is currently pinned (`is_pinned = true`).<br>2. In objectlist, locate "Keqing" in the Pinned section.<br>3. Click the pin icon to unpin.<br>4. Observe objectlist within 150ms.<br>5. Verify DB`is_pinned` value. | "Keqing" moves out of the Pinned section and appears in its default alphabetical sort position in the main list. DB`is_pinned = false`. | S2 | AC-40.1.2 |
| TC-40-003 | All Objects pinned layout integrity | Edge | Low | 5 Objects | 1. Ensure Genshin configured with 5 Objects: Keqing, Amber, Kaeya, Fischl, Xiao — all`is_pinned = false`.<br>2. Pin Keqing → confirm objectlist.<br>3. Pin Amber → confirm objectlist.<br>4. Pin Kaeya → confirm objectlist.<br>5. Pin Fischl → confirm objectlist.<br>6. Pin Xiao → confirm objectlist.<br>7. Observe final objectlist state. | Pinned section contains all 5 Objects. The unpinned/main list below is empty. ObjectList does not crash, overflow, or show duplicate entries. Scroll if needed (no layout breakage). | S3 | AC-40.1.3 |
| TC-40-004 | Favorite a mod card | Positive | High |`folder_path: "mods_path/Characters/Kaeya/KaeyaMod_v2"` | 1. Ensure Folder`KaeyaMod_v2` visible in grid.`is_favorite = false`.<br>2. Right-click`KaeyaMod_v2` card in Explorer grid.<br>3. Click "Favorite" in context menu.<br>4. Start stopwatch.<br>5. Observe card in grid.<br>6. Query DB:`SELECT is_favorite FROM folders WHERE folder_path = ?`. | Within ≤ 150ms, a gold star ⭐ icon appears on the`KaeyaMod_v2` card. DB`is_favorite = true`. No page reload — optimistic or cache-invalidated. | S2 | AC-40.2.1 |
| TC-40-005 | Favorites-only filter shows starred mods | Positive | Medium | Favorites filter state | 1. Ensure Grid shows 5 mod cards. 2 have`is_favorite = true` (KaeyaMod_v2, AmbroseMod), 3 have`is_favorite = false`.<br>2. In Explorer grid toolbar, click "Favorites" filter button.<br>3. Observe the number of visible cards in the grid. | Grid shows exactly 2 cards:`KaeyaMod_v2` and`AmbroseMod`. The other 3 non-favorite cards are hidden. No error state. Filter toggle is visually highlighted as active. | S2 | AC-40.2.2 |
| TC-40-006 | Unfavorite removes card in filtered view | Edge | Medium | Favorites filter active | 1. Ensure Favorites filter active. Grid shows 2 favorites: KaeyaMod_v2 and AmbroseMod.<br>2. Favorites filter is ON. Grid shows KaeyaMod_v2 and AmbroseMod.<br>3. Right-click`KaeyaMod_v2` → "Unfavorite".<br>4. Observe grid immediately. |`KaeyaMod_v2` card is immediately removed from the filtered view. Only`AmbroseMod` remains. DB`is_favorite = false` for KaeyaMod_v2. No crash or blank screen. | S3 | AC-40.2.3 |
| TC-40-007 | Move mod to different Object | Positive | High | Source: Keqing object, Target: Amber object | 1. Ensure`KaeqingMod_v2` exists at`mods_path/Characters/Keqing/KaeqingMod_v2/`. Target Object: "Amber" (category: Characters).`mods_path/Characters/Amber/KaeqingMod_v2/` does NOT exist.<br>2. Right-click`KaeqingMod_v2` → "Move to..."<br>3. Object picker modal opens.<br>4. Select "Amber" from picker.<br>5. Confirm move.<br>6. Check filesystem:`mods_path/Characters/Amber/KaeqingMod_v2/`.<br>7. Open`mods_path/Characters/Amber/KaeqingMod_v2/info.json` and read object field.<br>8. Query DB:`SELECT folder_path, object_id FROM folders WHERE name = 'KaeqingMod_v2'`. | Folder exists at new path.`info.json``object` field equals`"Amber"`. DB`folder_path` = new path,`object_id` = Amber's ID. Source path`mods_path/Characters/Keqing/KaeqingMod_v2/` no longer exists. | S1 | AC-40.3.1 |
| TC-40-008 | Grid updates after move (≤ 200ms) | Positive | High | Same as TC-40-007 | 1. Ensure Follow TC-40-007 setup. Both Keqing and Amber Object views accessible in objectlist.<br>2. Navigate to Keqing Object in objectlist — observe grid (mod card visible).<br>3. Execute Move to Amber (from TC-40-007).<br>4. Start timer at move confirmation.<br>5. Navigate to Amber Object in objectlist.<br>6. Stop timer when card appears in Amber's grid. | Within ≤ 200ms of the move completing, navigating to Amber's grid shows`KaeqingMod_v2`. Keqing's grid no longer shows the card. Exactly one`invalidateQueries(['folders', gameId])` call fired. | S2 | AC-40.3.2 |
| TC-40-009 | Move aborted on collision | Negative | High | Conflicting target folder | 1. Ensure`KaeqingMod_v2` at Keqing path. AND`mods_path/Characters/Amber/KaeqingMod_v2/` already exists.<br>2. Right-click`KaeqingMod_v2` → "Move to..." → select "Amber".<br>3. Confirm move.<br>4. Observe response.<br>5. Check source path remains.<br>6. Check DB unchanged. | Move aborted BEFORE any file operation. Toast: "Move failed: a folder named 'KaeqingMod_v2' already exists in 'Amber'." Source folder at Keqing path untouched. DB shows original`folder_path` and`object_id`. | S2 | AC-40.3.3 |
| TC-40-010 | DB rollback when rename succeeds but DB fails | Negative | High | Mocked DB failure | 1. Ensure`KaeqingMod_v2` ready to move to Amber. Inject mock DB failure on UPDATE (SQLite error).<br>2. Set up DB UPDATE failure mock (test hook).<br>3. Trigger "Move to Amber".<br>4. After operation fails, check DB`folder_path`.<br>5. Check source path on disk.<br>6. Check target path on disk. | DB transaction rolls back:`folder_path` still shows original Keqing path.`fs::rename` (if already executed) is reversed — file moved back to original path. No orphan DB row. Toast shows move error. | S2 | AC-40.3.4 |
| TC-40-011 | Move enabled mod preserves enabled state | Edge | High | Enabled folder (no prefix) | 1. Ensure`KaeyaMod` (no`DISABLED` prefix,`is_enabled = true`) at`mods_path/Characters/Kaeya/KaeyaMod/`. Moving to Amber.<br>2. Right-click`KaeyaMod` → "Move to..." Amber.<br>3. Confirm move.<br>4. After completion, inspect filesystem path:`mods_path/Characters/Amber/`.<br>5. Check DB`is_enabled` for the moved folder. | Folder exists at`mods_path/Characters/Amber/KaeyaMod/` — still WITHOUT`DISABLED` prefix. DB`is_enabled = true` unchanged. 3DMigoto would still load this mod (no unexpected disable). | S2 | AC-40.3.5 |

---

## D. Missing / Implied Test Areas

- **Move Disabled Mod**:`DISABLED KaeyaMod` moved to Amber → should land at`mods_path/Characters/Amber/DISABLED KaeyaMod/` (prefix preserved). Not explicitly stated in AC but follows naming convention.
- **`info.json` Non-Destructive Patch**: Move patches only the`object` field in`info.json`. Any other keys (e.g.,`author`,`version`,`description`) must be preserved unchanged.
- **Cross-Category Move**: Moving from`Characters > Keqing` to`Weapons > Aquila Favonia` — target path changes both category and object folders. Confirm`create_dir_all` creates the full hierarchy if needed.
- **Object Picker Modal Accessibility**: The "Move to..." picker modal should filter out the current Object (can't move to same object), and should show all available Objects in the current game.
- **Concurrent Pin Spam**: Clicking pin/unpin rapidly 5 times — final DB state should match last click intent.

---

## E. Open Questions / Gaps

- When moving a mod, if`info.json` does not exist (optional file), does the move still proceed without error, just skipping the info.json patch?
- Does "Move to..." support selecting an Object from a different Category (e.g., moving from Characters to Weapons)? The req mentions`mods_path/{category}/{object_name}` so it should naturally support it via category derivation from the target object.

---

## F. Automation Candidates

- **TC-40-001 (Pin)**: Rust unit test — call`pin_object(game_id, object_id, true)`, assert`SELECT is_pinned = 1`.
- **TC-40-007 (Move)**: Rust integration test — temp dir setup, call`move_mod_to_object`, assert new path exists, old path gone, DB row updated, info.json patched.
- **TC-40-009 (Collision abort)**: Rust unit test — pre-create target, assert`Err(CommandError::PathCollision)` returned before any FS modification.
- **TC-40-005 (Favorites filter)**: Vitest — render grid with mock data, toggle filter button, assert rendered card count equals expected.

---

## G. Test Environment Setup

- **OS**: Windows 10/11
- **App**: EMMM2 dev build (`cargo tauri dev`)
- **Game**: Genshin Impact configured
- **DB State**:`emmm2.db` with Objects: Keqing, Amber, Kaeya, Fischl, Xiao; folders indexed.
- **Filesystem State** (create before each TC group):
 -`mods_path/Characters/Keqing/KaeqingMod_v2/` (with`info.json`, dummy`.ini`)
 -`mods_path/Characters/Kaeya/KaeyaMod_v2/` (`is_favorite = false` in DB)
 -`mods_path/Characters/Kaeya/AmbroseMod/` (`is_favorite = false` in DB)
 - Do NOT create:`mods_path/Characters/Amber/KaeqingMod_v2/` (for positive move tests)
- **OperationLock**: Released before each TC
- **File Watcher**: Running
-`info.json` in each mod folder:`{ "name": "...", "object": "Keqing", "author": "test" }`

## H. Cross-Epic E2E Scenarios

- **E2E-40-01 (Safe Mode Pin/Favorite Metadata Visibility)**: Enter Safe Mode visually (Epic 30) thereby hiding all designated NSFW Objects precisely. Verify mechanically that any explicitly "Pinned" status previously applied to such NSFW Objects remains suppressed from the UI Sidebar preventing ANY metadata-derived logic from breaking the Epic 30 Security constraints consistently`S1`. Ensure exiting Safe Mode reveals the original explicitly maintained "Pinned" ordering physically.
- **E2E-40-02 (Mass Move Operation)**: Trigger Epic 11 Grid mechanics inherently allowing simultaneous Multi-Selection (Ctrl+Click) of exactly 15 Mod items dragging them onto the`Amber` Object inside the Sidebar ObjectList physically thereby simultaneously queueing 15 distinct`move_mod_to_object` API invocations resolving properly`S2`.
