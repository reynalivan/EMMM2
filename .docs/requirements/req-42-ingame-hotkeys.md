# Epic 42: In-Game Hotkeys & Live Controls

## 1. Executive Summary

- **Problem Statement**: Users who stream or play competitively need to react instantly to social situations (Safe Mode panic), switch mod loadouts (preset cycle), or swap character variants (NoCape/Cape) without Alt-Tabbing out of the game — doing so mid-session breaks immersion and takes 5–10 second- **Proposed Solution**: A background OS-level global hotkey listener (`tauri-plugin-global-shortcut`) intercepting configurable key combos (Safe Mode, Preset Cycle, and a Unified Overlay Toggle), gated by a foreground window check, debounced with a `HOTKEY_COOLDOWN_MS = 500ms`. All runtime artifacts (KeyViewer, status banners, keybind texts) are consolidated within the `Mods/.emmm_data/` directory, which is excluded from the mod scanner.
- **Success Criteria**:
  - Hotkey response time (keystroke → 3DMigoto reload triggered): ≤ 2s total.
  - `switch_lock` correctly drops all hotkey inputs during action execution.
  - All workspace changes are atomic — runtime ends up in old state OR new state, never mixed.
  - `runtime_status.txt` is written atomically to `.emmm_data/status/`.
  - **Zero-Leak Policy**: Artifact directories are cleared before regeneration to prevent stale data.
  - Status banner and KeyViewer share a **unified toggle** (default `F7`); they persist as long as the toggle is ON (no auto-clear TTL).

---

## 2. User Experience & Functionality

### User Stories

#### US-42.1: Global Hotkey Listener

As a gamer, I want EMMM to listen for keyboard shortcuts globally, so that I can trigger mod changes while my game is focused.

| ID        | Type        | Criteria                                                                                                                                                                                                      |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-42.1.1 | ✅ Positive | Given EMMM is running (minimized or tray), when I press a configured hotkey combo, then the OS intercepts it via `tauri-plugin-global-shortcut` and the mapped action is queued                               |
| AC-42.1.2 | ✅ Positive | Given a hotkey fires, `HOTKEY_COOLDOWN_MS = 500ms` debounce prevents a second trigger — pressing the key twice within 500ms results in only 1 action                                                          |
| AC-42.1.3 | ✅ Positive | Given an action is currently executing (`switch_lock = true`), then ALL hotkey inputs are dropped — no queuing; the input is lost                                                                             |
| AC-42.1.4 | ❌ Negative | Given EMMM is fully closed, then hotkeys are inactive — no background process handles them; this is expected behavior and clearly documented                                                                  |
| AC-42.1.5 | ⚠️ Edge     | Given a hotkey combo conflicts with the game's own keybind (e.g., `F5` is `reload_config`), then the Settings UI shows a yellow "Conflict detected" warning — no hard block, but the user is advised to remap |

---

#### US-42.2: Live Action — Toggle Safe / Unsafe Mode

As a streamer, I want a panic-button hotkey that instantly filters NSFW mods and updates the game, so that sensitive content disappears from screen in under 2 seconds.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                                                          |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-42.2.1 | ✅ Positive | Given Safe Mode is OFF (default hotkey: `F5`), when pressed: (1) `safe_mode` flips and persists, (2) `enabled_mod_set` is recomputed, (3) derived artifacts are regenerated in `.emmm_data/`, (4) workspace is updated atomically, (5) `reload_fixes` key is sent to game window, (6) unified status banner updates to `Safe: ON` |
| AC-42.2.2 | ✅ Positive | Given Safe Mode is ON, when `F5` is pressed again, then `is_safe = false` mods may re-appear, reload_fixes triggered, banner shows `Safe: OFF`                                                                                                                                                                                    |
| AC-42.2.3 | ❌ Negative | Given the workspace update fails at any step, then reload is NOT triggered, `safe_mode` is reverted to its previous value, `switch_lock` is released, and a toast shows the error reason                                                                                                                                          |
| AC-42.2.4 | ⚠️ Edge     | Given 0 mods are marked NSFW (`is_safe = false`), then toggling still flips the global flag and writes the banner — it's a no-op file-wise but state is correct                                                                                                                                                                   |

---

#### US-42.3: Live Action — Switch Presets (Next / Previous)

As a user, I want to switch between my preset Collections in-game without Alt-Tabbing.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                             |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-42.3.1 | ✅ Positive | Given "Next Preset" (`F6`), when pressed: (1) next preset resolved, (2) `current_preset_id` persists, (3) `enabled_mod_set` recomputed, (4) artifacts regenerated, (5) workspace updated atomically, (6) `reload_fixes` triggered, (7) banner shows `Preset: {name}` |
| AC-42.3.2 | ✅ Positive | Given "Previous Preset" (`Shift+F6`), then the previous collection is selected with the same pipeline as Next                                                                                                                                                        |
| AC-42.3.3 | ✅ Positive | Given I'm on the last Collection, pressing "Next Preset" wraps to the first                                                                                                                                                                                          |
| AC-42.3.4 | ❌ Negative | Given the workspace update or artifact generation fails, then `current_preset_id` is NOT advanced, reload is NOT triggered, `switch_lock` released, toast shows error                                                                                                |

---

#### US-42.4: Unified In-Game Overlay Toggle

As a user, I want a single hotkey to show or hide all in-game status and keybind information, so that I can quickly reference my settings and then clear the screen.

| ID        | Type        | Criteria                                                                                                                                                                                           |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-42.4.1 | ✅ Positive | Given the Unified Toggle key (`F7`), when pressed: (1) 3DMigoto `$kv_active` variable flips (0 ↔ 1), (2) the overlay (Status Banner or KeyViewer) appears or disappears instantly within ≤ 1 frame |
| AC-42.4.2 | ✅ Positive | Given the overlay is ON and a character hash is detected, the character's keybinds are shown. Otherwise, the global status banner is shown.                                                        |
| AC-42.4.3 | ✅ Positive | Given any hotkey action (Safe Mode/Preset) occurs while the overlay is ON, the banner updates its text on the next frame after the artifact is written.                                            |

---

#### US-42.5: In-Game Status Banner

As a user, I want to see a clear confirmation of my active settings in-game.

| ID        | Type        | Criteria                                                                                                                                   |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------ | ------------------- |
| AC-42.5.1 | ✅ Positive | Given any hotkey action completes, `.emmm_data/status/runtime_status.txt` is written atomically with the format: `[EMM2] Safe: {On/Off}    | Collection: {Name} | Toggle: [{f7_key}]` |
| AC-42.5.2 | ✅ Positive | The status banner resides at `.emmm_data/status/runtime_status.txt`. It is **persistent** while the overlay is toggled ON (no auto-clear). |
| AC-42.5.3 | ❌ Negative | Given the 3DMigoto renderer is unavailable, the text file is still written correctly for external verification.                            |

---

#### US-42.6: Reload Key Discovery

As a system, I want to auto-discover the actual 3DMigoto reload key from `d3dx.ini`, so that the in-game reload is sent to the correct key binding.

| ID        | Type        | Criteria                                                                                                           |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------ |
| AC-42.6.1 | ✅ Positive | Given `d3dx.ini` is readable, the backend parses `[Key*]` sections to find `reload_fixes` bindings and stores them |
| AC-42.6.2 | ✅ Positive | Given any mod/preset/safe-mode change, the backend sends `reload_fixes` key to the game window via `enigo`         |
| AC-42.6.3 | ❌ Negative | Given `d3dx.ini` is missing or `reload_fixes` binding is not found, then `F10` is used as fallback                 |

---

### Non-Goals

- No in-game overlay rendered by EMMM directly — 3DMigoto `help.ini` pipeline is used.
- No hotkey for individual mod toggling — only Safe Mode, Preset cycle, and Variant cycle.
- Status Banner is NOT temporary — it toggles ON/OFF with the KeyViewer overlay.

---

## 3. Technical Specifications

### Architecture Overview

```
Hotkey Defaults (configurable in HotkeysTab):
  F5          → Toggle Safe Mode
  F6 / S+F6   → Next / Previous Preset
  F7          → Unified Overlay Toggle (Status + KeyViewer)

Runtime Artifacts (.emmm_data/):
  status/runtime_status.txt   -> Unified status string
  keybinds/active/*.txt       -> Per-character keybind lists
  KeyViewer.ini               -> 3DMigoto render logic

Render Pipeline (3DMigoto help.ini):
  - For full end-to-end pipeline and "Big Picture" details, see [req-43-dynamic-keyviewer.md](file:///e:/Dev/EMMMNEW/.docs/requirements/req-43-dynamic-keyviewer.md#4-architecture--pipeline-the-big-picture).
  1. User toggles F7 ($kv_active = 0|1)
  2. [Present] resets $kv_has_active = 0
  3. TextureOverride hits set $kv_has_active = 1 and $kv_active_code = 0xHASH
  4. Decision Tree:
     if $kv_active == 1:
       if $kv_has_active == 1: Render keybinds/{hash}.txt
       else: Render status/runtime_status.txt
     else: Notification = null (hide)
```

### Integration Points

| Component           | Detail                                                                          |
| ------------------- | ------------------------------------------------------------------------------- |
| Global Shortcut     | `tauri-plugin-global-shortcut` — registered on app bootstrap                    |
| Foreground Window   | `winapi::um::winuser::GetForegroundWindow` + `GetWindowText` → match `game_exe` |
| Reload Discovery    | Parse `d3dx.ini` `[Key*]` sections for `reload_fixes` bindings                  |
| `enigo`             | Sends `reload_fixes` key to the game window                                     |
| Atomic Status Write | `.tmp` → rename; single-line format with key labels                             |
| .emmm_data          | Standardized location for all runtime assets; hidden from EMMM scanner          |

istent State (store.json):
{ safe_mode: bool, current_preset_id: String,
folder_group_selection: HashMap<{scope_code_hash, preset_id}, active_index> }

Folder Groups (built offline during scan, Epic 11/25):
FolderGroup { group_id, scope_code_hash?, folders: Vec<PathBuf>, active_index: usize }

fire_hotkey_action(action: HotkeyAction) → ():
if switch_lock: return // drop input, no queuing
if cooldown not elapsed: return
switch_lock = true
reset cooldown timer

match action:
ToggleSafeMode → toggle_safe_mode_live()
NextPreset / PrevPreset → cycle_preset_live(direction)
NextVariant / PrevVariant → cycle_variant_live(direction)
on error at any step: revert state, release switch_lock, addToast(error)

toggle_safe_mode_live():

1. flip safe_mode, persist
2. recompute enabled_mod_set:
   preset_mods ∪ active_variant_selection − (if safe_mode: is_safe=false mods)
3. regenerate: keybinds/active/\*.txt + runtime_status.txt
   optionally regenerate: KeyViewer.ini (if mapping scope changed)
4. apply_workspace(enabled_mod_set) // WS-1/WS-2
5. send reload_fixes via enigo (fallback: show CTA)
6. `trigger_overlay_refresh()` utility regenerates all artifacts.
7. switch_lock = false

cycle_preset_live(direction):

1. resolve next/prev preset_id (ordered alphabetically, wrap-around)
2. persist current_preset_id
3. recompute enabled_mod_set (same pipeline as safe toggle)
4. regenerate artifacts
5. apply_workspace
6. send reload_fixes
7. `trigger_overlay_refresh()` utility regenerates all artifacts.
8. switch_lock = false

cycle_variant_live(direction):

1. resolve scope_code_hash (priority: $kv_active_code in d3dx_user.ini
   → last_selected_scope in store → global fallback)
2. select FolderGroup by scope_code_hash
3. move active_index ± 1 (wrap-around), persist per {scope_code_hash, preset_id}
4. recompute enabled_mod_set:
   preset_mods + new active variant folder − other variants in group + safe filter
5. regenerate: keybinds/active/\*.txt + runtime_status.txt
6. apply_workspace
7. send reload_fixes
8. write_status({ safe, preset, folder: variants[active_index].name, scope })
9. schedule clear_status 3.0s
10. switch_lock = false

write_status(fields) → ():
content = "EMM2 Runtime\nSafe: {ON/OFF}\nPreset: {name}\n[Folder: {name}]\n[Scope: {char}]"
assert len(content) <= 10 lines, 4KB
fs::write(status_path.tmp, content)
fs::rename(status_path.tmp, status_path) // atomic

apply_workspace(enabled_mod_set): // WS-1 preferred, WS-2 fallback
WS-1: build Runtime/next, atomic junction swap (current ↔ next)
WS-2: DISABLED prefix renames with OperationLock + WatcherSuppression

```

### Integration Points

| Component           | Detail                                                                                        |
| ------------------- | --------------------------------------------------------------------------------------------- |
| Global Shortcut     | `tauri-plugin-global-shortcut` — registered on app bootstrap                                  |
| Foreground Window   | `winapi::um::winuser::GetForegroundWindow` + `GetWindowText` → match `game_exe`               |
| Reload Discovery    | Parse `d3dx.ini` `[Key*]` sections for `reload_fixes` / `reload_config` bindings on game load |
| `enigo`             | `enigo::Keyboard::key(discovered_reload_key)` — simulates reload keypress to game window      |
| Atomic Status Write | `.tmp` → rename; single-line format; no TTL (persistent with toggle)                |
| Folder Groups       | Built during Epic 11 scan; persisted per `{scope_code_hash, preset_id}` in `store.json`       |
| Collections Apply   | Reuses `apply_collection` machinery (Epic 31) for preset switching                            |
| WatcherSuppression  | Applied during WS-2 workspace updates (DISABLED prefix renames)                               |

### Security & Privacy

- **`enigo` key simulation** targeted at the foreground game window — not a global keyboard injection.
- **`runtime_status.txt` path scoped** to `.emmm_data/status/` within `game_mods_path` — validated `starts_with(mods_path)`.
- **`switch_lock` prevents concurrent actions** — no race conditions between two simultaneous hotkey triggers.
- **UIPI (User Interface Privilege Isolation) Limitation**: If the target game is running as Administrator, EMMM must also be run as Administrator for `enigo` to successfully send keystrokes. Otherwise, Windows UIPI will silently block the simulated keypresses.
- **All workspace writes atomic** — either old or new state, never partial (WS-1: junction swap; WS-2: OperationLock scope).
- **Fail-fast on any step error**: no reload, no state advance, immediate lock release + toast — prevents corrupt mid-state.

---

## 4. Dependencies

- **Blocked by**: Epic 30 (Privacy Safe Mode — toggle logic), Epic 31 (Collections — cycle apply logic), Epic 28 (File Watcher — WatcherSuppression for WS-2), Epic 11 (Folder Grid — Folder Group classification), Epic 43 (Dynamic KeyViewer — artifact regeneration triggers).
- **Blocks**: Epic 43 (Dynamic KeyViewer — shares reload handshake + `runtime_status.txt` write pattern).
```
