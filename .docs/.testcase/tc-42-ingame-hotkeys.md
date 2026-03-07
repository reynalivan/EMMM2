# Test Cases: In-Game Hotkeys & Live Controls (Epic 42)

## A. Requirement Summary

- **Feature Goal**: Background global hotkey listener (`tauri-plugin-global-shortcut`) with configurable combos, gated by foreground game window check, 500ms debounce, and`switch_lock` mutex. Allows triggering actions like Toggle Safe Mode, Switch Presets, and Switch Variant Folders while the game is running.
- **User Roles**: Application User / Gamer.
- **Acceptance Criteria** (Summary):
 - Hotkeys intercept globally but only execute if the mapped game executable is the current Windows foreground process.
 - Active 500ms debounce drops rapid sequential keystrokes.
 -`switch_lock` mechanism actively drops inputs if a previous hotkey operation is currently generating artifacts or applying workspace.
 - Fail-fast pipeline: If ANY step (recomputation, artifact generation, workspace writing) fails, the entire stack halts, the exception is logged, and NO visual garbage is rendered to the game.
 - In-game Banner (Phase 5): Hotkey execution writes`runtime_status.txt` atomically, displaying the status string (e.g.,`Safe: ON` or`Preset: Beta`), and auto-clears after 3.0 seconds TTL.
 - Dynamic Reload Key (Phase 5): Automatically discovers the`reload_fixes` key from`d3dx.ini` (e.g., F10), falling back.
 - Switch Variant Folder (Phase 5): Cycles accurately through available active variant folders matching the active context.
 - WS-1/WS-2 Atomicity (Phase 5): Ensures workspace modification (`junction` recreation or`DISABLED` rename) is transactional and race-condition free.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-42-ingame-hotkeys.md`

| Acceptance Criteria | Covered by TC IDs |
| :------------------------------------------- | :---------------- |
| AC-42.1.1 (Hotkey intercept) | TC-42-001 |
| AC-42.1.2 (Foreground gate) | TC-42-002 |
| AC-42.1.3 (500ms debounce) | TC-42-003 |
| AC-42.1.4 (switch_lock drops input) | TC-42-004 |
| AC-42.1.5 (EMMM2 closed = inactive) | TC-42-005 |
| AC-42.1.6 (Conflict warning) | TC-42-006 |
| AC-42.2.1 (Safe Mode ON pipeline) | TC-42-007 |
| AC-42.2.2 (Safe Mode OFF pipeline) | TC-42-008 |
| AC-42.2.3 (Fail-fast on error) | TC-42-009 |
| AC-42.2.4 (0 NSFW mods no-op) | TC-42-010 |
| Phase 5: Next Preset Cycle | TC-42-011 |
| Phase 5: Next Variant Folder Cycle | TC-42-012 |
| Phase 5: Banner textual format write | TC-42-013 |
| Phase 5: Banner TTL Auto-Clear | TC-42-014 |
| Phase 5: Reload Key Discovery (`d3dx.ini`) | TC-42-015 |
| Phase 5: Workspace Matrix (`WS-1` vs`WS-2`) | TC-42-016 |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :-------------------------------------- | :------- | :------- | :--------------- | :------------------------------------------------------------------------------------------------------- | :--------------------------------------------- | :-------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------- |
| TC-42-001 | Hotkey intercepted while app minimized | Positive | High | S1 | Game is running and in foreground. EMMM2 is minimized to tray.`F5` bound to Safe Mode. | Hotkey:`F5` | 1. Press`F5` while focused on the game window. | EMMM2 backend captures`F5`. Safe mode is toggled.`runtime_status.txt` is updated. Game reload key is dispatched. | AC-42.1.1 |
| TC-42-002 | Hotkey dropped when game not focused | Negative | High | S1 | EMMM2 is running. Desktop or browser is the active 100% foreground window. | Foreground: chrome.exe | 1. Press`F5` in the browser window. | EMMM2 logs "hotkey discarded: foreground check failed". Safe mode status is untouched. Original keystroke passes through accurately to the browser. | AC-42.1.2 |
| TC-42-003 | 500ms debounce protection | Edge | High | S2 | Game in foreground. | Rapid double press within`400ms`. | 1. Spam push`F5` twice extremely fast. | Action fires exactly once. The second keystroke logs "cooldown active" immediately dropping execution without queueing. Payload executes exactly once. | AC-42.1.3 |
| TC-42-004 |`switch_lock` exclusivity | Edge | High | S2 | Application is actively compiling a large workspace (Mock`3.0s` delay). | Delay mock active.`F6` Next Preset. | 1. Trigger`F6` Next Preset.<br>2. While loading, hit`F5` Safe Mode.<br>3. Wait 4s. |`F5` is explicitly dropped logging "switch_lock held". The application completes the Preset swap safely without race conditions mapping corrupted workspaces. | AC-42.1.4 |
| TC-42-005 | App close clears global hook | Positive | High | S2 | EMMM2 is closed. Game is running. | EMMM2 terminated. | 1. Press`F5` in game. | No action is triggered because the background listener thread`tauri-plugin-global-shortcut` is dead. Game executes native default action for`F5`. | AC-42.1.5 |
| TC-42-006 | Key Conflict System Warning | Edge | Medium | S3 | Settings UI Hotkey config screen. | Binding`F10` which matches game's reload key. | 1. Navigate to Settings.<br>2. Map Toggle Safe Mode to`F10`. | UI proactively warns "Conflict detected: F10 is used by 3DMigoto". User can override, but the warning remains strictly visible. | AC-42.1.6 |
| TC-42-007 | Safe Mode ON pipeline | Positive | High | S1 | 1 NSFW mod enabled. Safe Mode is OFF. | Key:`F5` | 1. Hit`F5` in-game. | Database flips`safe_mode=true`. Workspace explicitly disables the NSFW mod. Banner`Safe: ON` generated. Game reload fired`<2.0s`. | AC-42.2.1 |
| TC-42-008 | Safe Mode OFF pipeline | Positive | High | S1 | Safe mode currently ON. Same payload. | Key:`F5` | 1. Hit`F5` in-game. | Database flips`safe_mode=false`. Workspace restores the NSFW mod physically. Banner`Safe: OFF` generated. Game reload fired. | AC-42.2.2 |
| TC-42-009 | Fail-fast rollback | Negative | High | S1 | Mock the workspace`fs::rename` operation to throw an explicit OS permission denied error. | Mock Error. | 1. Hit`F5`. | Pipeline halts. Output states are identical to input states.`switch_lock` explicitly`Drop`s avoiding a permanent dead-lock. Error string bubbled to backend logger. | AC-42.2.3 |
| TC-42-010 | Safe Mode toggle with 0 Context | Edge | Low | S3 | No NSFW mods in the current collection. |`0` NSFW payload. | 1. Hit`F5`. | Pipeline succeeds updating`Safe: ON` banner, bypassing physical filesystem writes optimizing purely without unnecessary overhead calculations. | AC-42.2.4 |
| TC-42-011 | Phase 5: Next Preset Cycle | Positive | High | S1 | Database has exactly 3 collections:`A`,`B`,`C`. Current is`A`. | Key:`F6` (Next Preset). | 1. Hit`F6`.<br>2. Wait 2s.<br>3. Hit`F6` again.<br>4. Hit`F6` again. | Cycle 1: Loads`B`. Banner:`Preset: B`.<br>Cycle 2: Loads`C`. Banner:`Preset: C`.<br>Cycle 3: Wraps explicitly back to`A`. Banner:`Preset: A`. Workspace explicitly overwrites mapped to definitions. | Phase 5 |
| TC-42-012 | Phase 5: Next Variant Cycle | Positive | High | S1 | A Mod contains variants`Blue`,`Red`,`Green`. Current is`Blue`. Game scoped functionally via keyview. | Key:`F8` (Next Variant Folder). | 1. Hit`F8`.<br>2. Wait.<br>3. Hit`F8`. | Automatically disables`Blue`, enables`Red`. Banner emits`Folder: Red, Scope: Keqing`. Cycle 2 enables`Green`. Mutually exclusive rules respected. | Phase 5 |
| TC-42-013 | Phase 5: Banner textual format write | Positive | Medium | S2 | File watcher targeted at`EMM2/status/runtime_status.txt`. | Action executed. | 1. Execute any hotkey action. | Backend organically generates UUID temporary`.tmp` text blob, injecting raw UTF-8 banner string, triggering an atomic`fs::rename(tmp, target)` bypassing half-rendered file read states. | Phase 5 |
| TC-42-014 | Phase 5: Banner TTL Auto-Clear | Positive | Medium | S2 | Banner visible in game currently. | Delay:`3500ms`. | 1. Execute action.<br>2. Idle without hotkey presses for strictly > 3.0 seconds. | Tokio backend timer explicitly triggers a deletion sequence clearing`runtime_status.txt` removing the on-screen banner without race conditions. | Phase 5 |
| TC-42-015 | Phase 5: Reload Discovery (`d3dx.ini`) | Positive | High | S2 |`d3dx.ini` actively maps`[KeyReloadFixes] key = F11`. | String:`reload_fixes = F11` | 1. Fire hotkey action. | Backend regex scanner securely detects`F11` as the active target, firing the exact virtual keystroke. | Phase 5 |
| TC-42-016 | Phase 5: Workspace Strategy Validations | Positive | High | S1 | Setting actively swaps deployment strategy between`WS-1` (Junction) and`WS-2` (Disabled Prefix). |`deployment_strategy`. | 1. Set to`WS-1`. Fire Action.<br>2. Set to`WS-2`. Fire Action. | WS-1 constructs`NTFS` Symlinks. WS-2 renames prefixes. | Phase 5 |

## D. Missing / Implied Test Areas

_No outstanding items — UIPI elevated context limitation is documented in`req-42-ingame-hotkeys.md` (Security & Privacy section)._

## E. Open Questions / Gaps

- When`AUTO_SCOPE_FROM_GAME = ON`, the scope is read from`$kv_active_code` in`d3dx_user.ini`. What if this file is locked by 3DMigoto when EMMM2 reads it mid-render? Is there a retry strategy?

## F. Automation Candidates

- **TC-42-003**: Headless execution generating exact duplicate mapped key codes inside`200ms` window verifying accurate debounce count.
- **TC-42-011**: Mock sequential cycle calls asserting accurately iterative wrapping output accurately matching exact database mapping precisely.

## G. Test Environment Setup

- **OS**: Windows 10/11
- **App**: EMMM2 dev build (`cargo tauri dev`)
- **Game**:`GenshinImpact.exe` configured in EMMM2 with valid`mods_path` and`d3dx.ini`
- **DB State**:`emmm2.db` with 3 Collections (`A`,`B`,`C`) and mods with`is_safe` flags set
- **Status Path**:`EMM2/status/runtime_status.txt` (must exist as writable path)

## H. Cross-Epic E2E Scenarios

- **E2E-42-001**: User actively presses Hotkey`F6` repeatedly iterating Collection presets (Epic 42) mapping explicitly against assigned Randomizer outputs natively generated (Epic 35) safely updating.`S1`.
- **E2E-42-002**: Active Safe Mode triggered (Epic 30) triggers filtering.`S1`.
