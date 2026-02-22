# EMM2 In-game Controls Add-on — Final Requirements (No Gray Areas)

## 1) Goal

Provide **three in-game control actions** while the game is running, driven by **OS-level hotkeys** handled by EMM2:

1.  Toggle **Safe / Unsafe Mode** (privacy/SFW gating)
2.  Switch **Presets** (Next/Previous)
3.  Switch **Variant Folder** (Next/Previous) — quick browsing without changing the preset

Each action must:

- Apply changes **atomically** to the runtime workspace
- Trigger a **reliable 3DMigoto reload**
- Show a short **in-game status banner** confirming the new state

Performance constraints:

- **No in-game directory scanning**
- **No per-frame heavy logic** (only a simple TTL banner draw)

---

## 2) Terms

- **Runtime Workspace**: the folder tree that 3DMigoto actually includes at runtime.
- **Preset**: atomic selection of enabled mods (EMM2 Collections).
- **Variant Folder**: a mutually-exclusive folder choice inside a **Folder Group** (e.g., cape/no-cape).
- **Folder Group**: ordered list of Variant Folders where exactly one is active at a time (default).
- **Reload**: invoking 3DMigoto’s reload key(s) so it re-reads INIs/resources.

---

## 3) Architecture

### 3.1 Control Plane (EMM2)

- Owns all file operations, workspace swaps, generation steps.
- Runs OS hotkey listener.
- Maintains persistent runtime state: `safe_mode`, `current_preset_id`, `folder_group_selection`.

### 3.2 Data Plane (3DMigoto)

- Loads INIs/resources from Runtime Workspace via `include_recursive` rules.
- Applies changes only after Reload.
- Renders the status banner using the same PrintText adapter as KeyViewer.

---

## 4) Runtime Workspace Strategy (must choose exactly one)

EMM2 must implement exactly one primary strategy (others optional fallbacks):

### WS-1 Recommended: Junction/Symlink Swap

- EMM2 maintains two workspaces:
  - `Runtime/current` and `Runtime/next`

- 3DMigoto points to a stable path (e.g., `Mods`) that is a junction/symlink to `Runtime/current`.
- Apply change:
  1.  Build `Runtime/next`
  2.  Swap `current ↔ next` using atomic rename operations
  3.  Update junction to point to the new `current` (if needed)

### WS-2 Fallback: DISABLED Prefix + exclude_recursive

- EMM2 enables/disables by renaming folders:
  - enable = normal name
  - disable = prefix `DISABLED_...`

- Requires the user’s d3dx.ini to have `exclude_recursive = DISABLED*` enabled.

---

## 5) Hotkeys

### IC-1 Hotkeys (configurable)

Defaults (user-configurable):

- Toggle Safe Mode: `F5`
- Next Preset: `F6`
- Previous Preset: `Shift+F6`
- Next Variant Folder: `F8`
- Previous Variant Folder: `Shift+F8`
- Optional: Toggle Status Banner: `F7`

### IC-1.1 Focus rule (no ambiguity)

- Hotkeys only trigger actions when the **target game process is foreground focused**.
- A setting may allow “Global hotkeys” but default is **OFF**.

### IC-1.2 Debounce + lock

- Cooldown: `HOTKEY_COOLDOWN_MS = 500` (default)
- If an action is executing (`switch_lock = true`), all hotkeys are ignored.

### IC-1.3 Queue behavior

- No queuing: if user presses keys during `switch_lock`, inputs are dropped.

---

## 6) Action: Toggle Safe / Unsafe Mode

### IC-2 Safe Mode definition

Safe Mode is a global runtime filter that affects:

- Which mods are included in the Runtime Workspace
- Which keybinds appear in generated KeyViewer text

### IC-2.1 Safety classification

Each mod must have `is_safe` resolved at scan-time using:

- explicit metadata (preferred)
- else tag mapping rules

Missing policy (must be explicit):

- Default policy = **SAFE** (conservative)

### IC-2.2 Safe toggle algorithm

On Safe toggle hotkey:

1.  Flip `safe_mode` (persist it).
2.  Recompute `enabled_mod_set`:
    - Start from active preset’s mods
    - Apply current Variant Folder selection(s)
    - Filter out unsafe mods if `safe_mode = ON`

3.  Regenerate derived artifacts:
    - `EMM2/keybinds/active/*.txt`
    - `EMM2/status/runtime_status.txt`
    - Regenerate `Mods/EMM2_System/KeyViewer.ini` only if its mapping scope changed

4.  Apply workspace update using chosen WS strategy.
5.  Trigger Reload.
6.  Show status banner (post-reload).

---

## 7) Action: Switch Presets (Next/Previous)

### IC-3 Preset switching algorithm

On Next/Prev preset hotkey:

1.  Resolve next preset ID deterministically (wrap-around allowed).
2.  Persist `current_preset_id`.
3.  Recompute `enabled_mod_set` using the same pipeline as Safe toggle:
    - preset mods + variant selection + safe filter

4.  Regenerate derived artifacts:
    - `EMM2/keybinds/active/*.txt`
    - `EMM2/status/runtime_status.txt`
    - `Mods/EMM2_System/KeyViewer.ini` only if needed

5.  Apply workspace update (WS-1/WS-2).
6.  Trigger Reload.
7.  Show status banner (post-reload).

---

## 8) Action: Switch Variant Folder (Next/Previous)

### IC-4 Folder Groups

- Folder Groups are built offline during library scan.
- Each group:
  - `group_id`
  - `scope_code_hash` (optional; character-scoped)
  - `folders[]` (ordered)
  - `active_index` (persisted)

### IC-4.1 Folder group sourcing rules

EMM2 must support at least one deterministic rule set (configurable):

- Convention A: `Mods/<Character>/Variants/<FolderName>/...`
- Convention B: `Mods/<Character>/<FolderName>/...` with explicit metadata mark

### IC-4.2 Scope selection (no gray)

Folder switching uses a scope selected by the following strict priority order:

1.  If `AUTO_SCOPE_FROM_GAME = ON` AND `d3dx_user.ini` contains a non-zero persisted `$kv_active_code`, use that as `scope_code_hash`.
2.  Else use EMM2’s `last_selected_scope_code_hash` (chosen in UI).
3.  Else use a global fallback group configured by user.

### IC-4.3 Persist active folder selection

- EMM2 must persist `active_index` per `{scope_code_hash, preset_id}` (default), or per `{scope_code_hash}` if user selects that mode.

### IC-4.4 Folder switch algorithm

On Next/Prev folder hotkey:

1.  Resolve scope (IC-4.2).
2.  Select Folder Group by scope.
3.  Move `active_index` (wrap-around allowed), persist.
4.  Recompute `enabled_mod_set`:
    - preset mods + selected folder in group (mutually exclusive) + safe filter

5.  Regenerate derived artifacts:
    - `EMM2/keybinds/active/*.txt`
    - `EMM2/status/runtime_status.txt`

6.  Apply workspace update.
7.  Trigger Reload.
8.  Show status banner (post-reload).

---

## 9) Reload Handshake (no ambiguity)

### IC-5 Reload key discovery

EMM2 must read the user’s `d3dx.ini` and determine:

- `reload_fixes` key binding
- `reload_config` key binding

If either is missing:

- Fallback key = `F10` for that action

### IC-5.1 Which reload to use

- For mod/preset/folder changes: always trigger **reload_fixes**.
- Trigger **reload_config** only if EMM2 modified d3dx.ini itself (rare).

### IC-5.2 Execution

- Preferred: auto-send reload key to focused game window.
- If auto-send fails or is disabled:
  - EMM2 shows a CTA in its UI: `Reload in-game (<key>)`.

---

## 10) Status Banner Overlay

### IC-6 Banner file contract

EMM2 must write:

- `EMM2/status/runtime_status.txt`

Banner must be <= 10 lines and <= 4 KB.

### IC-6.1 Banner content (exact fields)

Always include:

- `Safe: ON/OFF`
- `Preset: <preset_name>` Optionally include if available:
- `Folder: <folder_name>`
- `Scope: <character_name or code_hash>`

### IC-6.2 Banner timing

- Banner TTL: `STATUS_TTL_SECONDS = 3.0` default
- Banner appears within <= 1s of Reload completion.

### IC-6.3 Rendering constraints

- Rendering uses the same renderer adapter as KeyViewer.
- If renderer unavailable, must fall back safely (show nothing or fallback message) without spam.

---

## 11) Atomicity + File Safety

### IC-7 Atomic writes

EMM2 must write generated files atomically:

- write to `*.tmp`
- fsync (optional)
- rename swap

### IC-7.1 Atomic workspace swap

Workspace changes must be all-or-nothing:

- runtime ends up in old state OR new state
- never a partially mixed state

---

## 12) Error Handling

### IC-8 Fail-fast rules

If an action fails at any step:

- Do not trigger Reload
- Do not advance preset/folder pointer
- Release `switch_lock`
- Show an EMM2 toast with error code + short reason

---

## 13) Acceptance Criteria

### AC-IC1 Safe toggle applies filtering

Safe OFF with unsafe mod enabled → toggle Safe ON → unsafe excluded, keybind text regenerated, reload_fixes triggered, banner shows `Safe: ON`.

### AC-IC2 Safe toggle round-trip

Safe ON → toggle Safe OFF → unsafe may return (subject to preset/folder), reload_fixes, banner `Safe: OFF`.

### AC-IC3 Next preset correctness

P1→Next→P2 applied, reload_fixes, banner `Preset: P2`.

### AC-IC4 Preset wrap-around

P3→Next→P1.

### AC-IC5 Folder switch only touches folder-group mods

Preset unchanged; group \[A,B,C\]; Next→B active; other folders in group disabled; reload_fixes; banner shows `Folder: B`.

### AC-IC6 Folder switch respects Safe Mode

Safe ON; switching folders never enables unsafe mods.

### AC-IC7 Atomic safety

Interrupted update → runtime is old OR new, never mixed.

### AC-IC8 Debounce

Holding hotkey triggers at most once per cooldown.

### AC-IC9 Reload discovery fallback

If reload key not found, fallback F10 is used and clearly shown in UI.

---

## 14) Example Banner Prints

Example A: After preset switch

```
EMM2 Runtime
Safe: ON
Preset: Arlecchino DPS Pack
Folder: Variants/NoCape
Scope: Arlecchino
```

Example B: After safe toggle

```
EMM2 Runtime
Safe: OFF
Preset: Casual Mix
Scope: 0xA1B2C3D4
```
