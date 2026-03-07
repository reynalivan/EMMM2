# Epic 42: In-Game Hotkeys & Live Controls

## 1. Executive Summary

- **Problem Statement**: Users who stream or play competitively need to react instantly to social situations (Safe Mode panic), switch mod loadouts (preset cycle), or swap character variants (NoCape/Cape) without Alt-Tabbing out of the game — doing so mid-session breaks immersion and takes 5–10 seconds.
- **Proposed Solution**: A background OS-level global hotkey listener (`tauri-plugin-global-shortcut`) intercepting 5 configurable key combos, gated by a foreground window check for the active `game_exe`, debounced with a `HOTKEY_COOLDOWN_MS = 500ms` and a `switch_lock` mutex that drops all hotkey inputs during execution. Three live actions are supported: (1) Toggle Safe/Unsafe Mode, (2) Switch Presets (Next/Prev), (3) Switch Variant Folder (Next/Prev). Each action recomputes the `enabled_mod_set`, regenerates artifacts, applies the workspace update atomically, triggers the 3DMigoto reload, and writes a structured status banner to `EMM2/status/runtime_status.txt`.
- **Success Criteria**:
  - Hotkey response time (keystroke → 3DMigoto reload triggered): ≤ 2s total (backend ≤ 500ms + enigo keypress ≤ 100ms + game reload ≤ 1s).
  - `switch_lock` correctly drops all hotkey inputs during action execution — 0 accidental double-applies regardless of key hold duration.
  - All workspace changes are atomic — runtime ends up in old state OR new state, never mixed.
  - On any mid-action failure: reload is NOT triggered, preset/folder pointer is NOT advanced, `switch_lock` is released, and a toast with error code is shown in UI.
  - `runtime_status.txt` is written atomically (`*.tmp` → rename) — ≤ 10 lines, ≤ 4KB.
  - Status banner appears in-game within ≤ 1s of reload completion; auto-clears after `STATUS_TTL_SECONDS = 3.0s`.

---

## 2. User Experience & Functionality

### User Stories

#### US-42.1: Global Hotkey Listener

As a gamer, I want EMMM2 to listen for keyboard shortcuts globally, so that I can trigger mod changes while my game is focused.

| ID        | Type        | Criteria                                                                                                                                                                                                          |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-42.1.1 | ✅ Positive | Given EMMM2 is running (minimized or tray), when I press a configured hotkey combo, then the OS intercepts it via `tauri-plugin-global-shortcut` and the mapped action is queued                                  |
| AC-42.1.2 | ✅ Positive | Given "Only trigger when game is focused" is ON (default ON), when the active window foreground process is `game_exe`, then hotkeys are processed; any other foreground window causes them to be silently ignored |
| AC-42.1.3 | ✅ Positive | Given a hotkey fires, `HOTKEY_COOLDOWN_MS = 500ms` debounce prevents a second trigger — pressing the key twice within 500ms results in only 1 action                                                              |
| AC-42.1.4 | ✅ Positive | Given an action is currently executing (`switch_lock = true`), then ALL hotkey inputs are dropped — no queuing; the input is lost                                                                                 |
| AC-42.1.5 | ❌ Negative | Given EMMM2 is fully closed, then hotkeys are inactive — no background process handles them; this is expected behavior and clearly documented                                                                     |
| AC-42.1.6 | ⚠️ Edge     | Given a hotkey combo conflicts with the game's own keybind (e.g., `F5` is `reload_config`), then the Settings UI shows a yellow "Conflict detected" warning — no hard block, but the user is advised to remap     |

---

#### US-42.2: Live Action — Toggle Safe / Unsafe Mode

As a streamer, I want a panic-button hotkey that instantly filters NSFW mods and updates the game, so that sensitive content disappears from screen in under 2 seconds.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-42.2.1 | ✅ Positive | Given Safe Mode is OFF (default hotkey: `F5`), when pressed: (1) `safe_mode` flips and persists, (2) `enabled_mod_set` is recomputed from active preset's mods + current variant selections, then `is_safe = false` mods are filtered out, (3) derived artifacts are regenerated (`keybinds/active/*.txt`, `runtime_status.txt`, optionally `KeyViewer.ini` if mapping scope changed), (4) workspace is updated atomically, (5) `reload_fixes` key is sent to game window, (6) banner shows `Safe: ON` |
| AC-42.2.2 | ✅ Positive | Given Safe Mode is ON, when `F5` is pressed again, then `is_safe = false` mods may re-appear (subject to preset/variant selection), reload_fixes triggered, banner shows `Safe: OFF`                                                                                                                                                                                                                                                                                                                   |
| AC-42.2.3 | ❌ Negative | Given the workspace update fails at any step, then reload is NOT triggered, `safe_mode` is reverted to its previous value, `switch_lock` is released, and a toast shows the error reason                                                                                                                                                                                                                                                                                                               |
| AC-42.2.4 | ⚠️ Edge     | Given 0 mods are marked NSFW (`is_safe = false`), then toggling still flips the global flag and writes the banner — it's a no-op file-wise but state is correct                                                                                                                                                                                                                                                                                                                                        |

---

#### US-42.3: Live Action — Switch Presets (Next / Previous)

As a user, I want to switch between my preset Collections in-game without Alt-Tabbing.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                                                                                                     |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-42.3.1 | ✅ Positive | Given "Next Preset" (`F6`), when pressed: (1) next preset ID resolved deterministically (alphabetical, wrap-around), (2) `current_preset_id` persists, (3) `enabled_mod_set` recomputed (preset mods + variant selection + safe filter), (4) artifacts regenerated, (5) workspace updated atomically, (6) `reload_fixes` triggered, (7) banner shows `Preset: {preset_name}` |
| AC-42.3.2 | ✅ Positive | Given "Previous Preset" (`Shift+F6`), then the previous collection is selected with the same pipeline as Next                                                                                                                                                                                                                                                                |
| AC-42.3.3 | ✅ Positive | Given I'm on the last Collection, pressing "Next Preset" wraps to the first                                                                                                                                                                                                                                                                                                  |
| AC-42.3.4 | ❌ Negative | Given the workspace update or artifact generation fails, then `current_preset_id` is NOT advanced, reload is NOT triggered, `switch_lock` released, toast shows error                                                                                                                                                                                                        |
| AC-42.3.5 | ⚠️ Edge     | Given 0 Collections exist, then "Next Preset" is a no-op — `runtime_status.txt` is written "No presets configured" and clears after 3s; no reload                                                                                                                                                                                                                            |

---

#### US-42.4: Live Action — Switch Variant Folder (Next / Previous)

As a user, I want to cycle through variant sub-folders (e.g., Cape / NoCape) for a character in-game, so that I can swap looks without changing the whole preset.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-42.4.1 | ✅ Positive | Given "Next Variant Folder" (`F8`), when pressed: (1) scope is resolved (from `$kv_active_code` in `d3dx_user.ini` if `AUTO_SCOPE_FROM_GAME = ON`, else last-selected scope in UI, else user-configured global fallback), (2) the Folder Group for that scope is selected, (3) `active_index` advances (wrap-around), (4) `enabled_mod_set` recomputed (preset mods + new variant in group + safe filter), (5) workspace updated, (6) reload_fixes triggered, (7) banner shows `Folder: {folder_name}` + `Scope: {character_name}` |
| AC-42.4.2 | ✅ Positive | Given "Previous Variant Folder" (`Shift+F8`), then `active_index` decrements with wrap-around — same pipeline                                                                                                                                                                                                                                                                                                                                                                                                                      |
| AC-42.4.3 | ✅ Positive | Given the switch succeeds, only the variant folder mods change — all other preset mods are untouched; the toggle is mutually exclusive within the Folder Group                                                                                                                                                                                                                                                                                                                                                                     |
| AC-42.4.4 | ✅ Positive | Given Safe Mode is ON during a folder switch, then the newly active variant's mods are filtered by `is_safe` — switching variants never enables NSFW mods                                                                                                                                                                                                                                                                                                                                                                          |
| AC-42.4.5 | ❌ Negative | Given no Folder Group is resolvable for the current scope, then the variant hotkey is a no-op — banner shows "No variant group for current scope"; no reload                                                                                                                                                                                                                                                                                                                                                                       |
| AC-42.4.6 | ⚠️ Edge     | Given a Folder Group's `active_index` was persisted per `{scope_code_hash, preset_id}` and the preset changes, then the variant selection resets to `active_index = 0` unless an explicit per-scope-per-preset record exists                                                                                                                                                                                                                                                                                                       |

---

#### US-42.5: In-Game Status Banner

As a user, I want to see a brief text in-game confirming my hotkey action, so that I know exactly what changed.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-42.5.1 | ✅ Positive | Given any hotkey action completes, `EMM2/status/runtime_status.txt` is written atomically (`*.tmp` → rename) with these fields: always `Safe: ON\|OFF` + `Preset: {name}`; optionally `Folder: {folder_name}` + `Scope: {char_name or code_hash}` — total ≤ 10 lines, ≤ 4KB |
| AC-42.5.2 | ✅ Positive | Given the banner is written, a `STATUS_TTL_SECONDS = 3.0s` Tokio timer fires and clears `runtime_status.txt` (writes empty or deletes) — the message disappears from screen automatically                                                                                   |
| AC-42.5.3 | ❌ Negative | Given the renderer backend for the status banner is unavailable, the overlay shows nothing — no error spam; the text file is still written so it's readable on next reload                                                                                                  |

---

#### US-42.6: Reload Key Discovery

As a system, I want to auto-discover the actual 3DMigoto reload key from `d3dx.ini`, so that the in-game reload is sent to the correct key binding.

| ID        | Type        | Criteria                                                                                                                                                                                               |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-42.6.1 | ✅ Positive | Given `d3dx.ini` is readable, the backend parses `[Key*]` sections to find `reload_fixes` and `reload_config` key bindings and stores them                                                             |
| AC-42.6.2 | ✅ Positive | Given any mod/preset/variant/safe-mode change, the backend sends `reload_fixes` key to the game window via `enigo` — NOT `reload_config` (which is only triggered if EMMM2 modified `d3dx.ini` itself) |
| AC-42.6.3 | ❌ Negative | Given `d3dx.ini` is missing or `reload_fixes` binding is not found, then `F10` is used as fallback — the UI shows "Reload key: F10 (fallback)" clearly in the HotkeysTab settings                      |
| AC-42.6.4 | ⚠️ Edge     | Given auto-send via `enigo` fails (e.g., game uses exclusive input mode), then EMMM2 falls back to showing a CTA in its UI: "Reload in-game (F10)" — the user must press it manually                   |

---

### Non-Goals

- No in-game overlay rendered by EMMM2 directly — only `runtime_status.txt` is written; 3DMigoto displays it.
- No hotkey for individual mod toggling — only Safe Mode, Preset cycle, and Variant Folder cycle.
- No hotkey recording via in-app key capture — configured via text input in HotkeysTab.
- No Linux/macOS support — Windows WinAPI/global shortcut only.
- Junction/Symlink Swap (WS-1) is the recommended workspace strategy; DISABLED prefix (WS-2) is a valid fallback — implementation must choose exactly one primary strategy.

---

## 3. Technical Specifications

### Architecture Overview

```
Hotkey Defaults (configurable in HotkeysTab):
  F5          → Toggle Safe Mode
  F6          → Next Preset
  Shift+F6    → Previous Preset
  F8          → Next Variant Folder
  Shift+F8    → Previous Variant Folder
  F7          → Toggle Status Banner (optional)

Runtime Persistent State (store.json):
  { safe_mode: bool, current_preset_id: String,
    folder_group_selection: HashMap<{scope_code_hash, preset_id}, active_index> }

Folder Groups (built offline during scan, Epic 11/25):
  FolderGroup { group_id, scope_code_hash?, folders: Vec<PathBuf>, active_index: usize }

fire_hotkey_action(action: HotkeyAction) → ():
  if switch_lock: return  // drop input, no queuing
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
  3. regenerate: keybinds/active/*.txt + runtime_status.txt
     optionally regenerate: KeyViewer.ini (if mapping scope changed)
  4. apply_workspace(enabled_mod_set)    // WS-1/WS-2
  5. send reload_fixes via enigo (fallback: show CTA)
  6. write_status({ safe: ON/OFF, preset: current_preset_name })
  7. schedule clear_status after 3.0s
  8. switch_lock = false

cycle_preset_live(direction):
  1. resolve next/prev preset_id (ordered alphabetically, wrap-around)
  2. persist current_preset_id
  3. recompute enabled_mod_set (same pipeline as safe toggle)
  4. regenerate artifacts
  5. apply_workspace
  6. send reload_fixes
  7. write_status({ safe, preset: new_preset_name })
  8. schedule clear_status 3.0s
  9. switch_lock = false

cycle_variant_live(direction):
  1. resolve scope_code_hash (priority: $kv_active_code in d3dx_user.ini
                              → last_selected_scope in store → global fallback)
  2. select FolderGroup by scope_code_hash
  3. move active_index ± 1 (wrap-around), persist per {scope_code_hash, preset_id}
  4. recompute enabled_mod_set:
       preset_mods + new active variant folder − other variants in group + safe filter
  5. regenerate: keybinds/active/*.txt + runtime_status.txt
  6. apply_workspace
  7. send reload_fixes
  8. write_status({ safe, preset, folder: variants[active_index].name, scope })
  9. schedule clear_status 3.0s
  10. switch_lock = false

write_status(fields) → ():
  content = "EMM2 Runtime\nSafe: {ON/OFF}\nPreset: {name}\n[Folder: {name}]\n[Scope: {char}]"
  assert len(content) <= 10 lines, 4KB
  fs::write(status_path.tmp, content)
  fs::rename(status_path.tmp, status_path)  // atomic

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
| Atomic Status Write | `.tmp` → rename; ≤ 10 lines, ≤ 4KB; cleared after 3.0s TTL via Tokio timer                    |
| Folder Groups       | Built during Epic 11 scan; persisted per `{scope_code_hash, preset_id}` in `store.json`       |
| Collections Apply   | Reuses `apply_collection` machinery (Epic 31) for preset switching                            |
| WatcherSuppression  | Applied during WS-2 workspace updates (DISABLED prefix renames)                               |

### Security & Privacy

- **`enigo` key simulation** targeted at the foreground game window — not a global keyboard injection.
- **`runtime_status.txt` path scoped** to `EMM2/status/` within `game_mods_path` — validated `starts_with(mods_path)`.
- **`switch_lock` prevents concurrent actions** — no race conditions between two simultaneous hotkey triggers.
- **UIPI (User Interface Privilege Isolation) Limitation**: If the target game is running as Administrator, EMMM2 must also be run as Administrator for `enigo` to successfully send keystrokes. Otherwise, Windows UIPI will silently block the simulated keypresses.
- **All workspace writes atomic** — either old or new state, never partial (WS-1: junction swap; WS-2: OperationLock scope).
- **Fail-fast on any step error**: no reload, no state advance, immediate lock release + toast — prevents corrupt mid-state.

---

## 4. Dependencies

- **Blocked by**: Epic 30 (Privacy Safe Mode — toggle logic), Epic 31 (Collections — cycle apply logic), Epic 28 (File Watcher — WatcherSuppression for WS-2), Epic 11 (Folder Grid — Folder Group classification), Epic 43 (Dynamic KeyViewer — artifact regeneration triggers).
- **Blocks**: Epic 43 (Dynamic KeyViewer — shares reload handshake + `runtime_status.txt` write pattern).
