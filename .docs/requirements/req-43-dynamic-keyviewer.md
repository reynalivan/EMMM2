# Epic 43: Dynamic KeyViewer Overlay

## 1. Executive Summary

- **Problem Statement**: Users who have tens of mods enabled across multiple characters have no in-game reference for which keybinds control which mod — displaying all keybinds at once clutters the screen; showing nothing requires memorization.
- **Proposed Solution**: A background generator in EMMM that monitors active mod sets and character hashes, producing a set of keybind text files in `Mods/.emmm_data/keybinds/active/`. A `KeyViewer.ini` system mod manages a 3DMigoto-level arbitration tree using a unified toggle (`$kv_active`). It uses the `help.ini` pipeline (`ResourceNotification` + `FormatText`) to render text persistently. Character detection is handled via `$kv_active_code` and a `$kv_last_seen` timestamp logic with a 1.5s threshold to prevent flickering during scene transitions or animation breaks.
- **Success Criteria**:
  - KeyViewer displays only when the unified toggle (`F7` by default) is ON.
  - Character detection is stable — text remains on screen as long as the hash was seen in the last 1.5 seconds.
  - No "if-chain" performance hit — uses resource mapping via `$kv_active_code`.
  - Content matches the Arlecchino mockup (Name, Dash-Underline, Key, Back, Toggle, Footer).

---

## 2. User Experience & Functionality

### User Stories

#### US-43.1: Hash Harvesting

As a system, I want to scan enabled mods for their `.ini` hashes and link them to known Objects via the Resource Pack, so that I know which keybinds to display per character.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                                                                                             |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-43.1.1 | ✅ Positive | Given a mod is enabled and its `.ini` contains `hash = XXXXXXXX` lines inside `TextureOverride*`, `ShaderOverride*`, `TextureOverrideIB*`, `ShaderOverridePS*`, or `ShaderOverrideVS*` sections, when harvest runs, then those hashes are extracted with `occurrence_count_total`, `section_names_seen[]`, `source_mod_ids[]`, and `first_seen/last_seen` timestamps |
| AC-43.1.2 | ✅ Positive | Given extracted hashes, each is scored against Resource Pack objects: `score = Σ weight(hash)` where `weight = base(10) + log(1 + occurrence_count) + rarity_bonus + hint_bonus`; the object with highest score wins if `score >= threshold`                                                                                                                         |
| AC-43.1.3 | ✅ Positive | Given a harvested hash not present in any Resource Pack object, it is stored as a **learned hash** in the `mod_hash_index` cache — available for future matching and optional suggestion to add to the Resource Pack                                                                                                                                                 |
| AC-43.1.4 | ❌ Negative | Given an enabled mod has no `.ini` or no hashes in the required section types, then it is excluded from hash harvest — no `[TextureOverride]` sentinel is generated for it                                                                                                                                                                                           |
| AC-43.1.5 | ✅ Positive | Given two different enabled mods hash to the same Object (same character, two skins both enabled), then keybinds are **grouped by mod name** with `[Mod: Name]` headers — no confusion between multiple mod sources.                                                                                                                                                 |
| AC-43.1.6 | ⚠️ Edge     | Given a hash appears in `known_hashes` of ≥ 3 different Resource Pack objects OR it appears in active-mod hashes across ≥ 2 different `code_hash` candidates with score margin < 15%, then it is marked `blacklisted_for_sentinel = true` — never used as a sentinel hash for any object                                                                             |
| AC-43.1.7 | ⚠️ Edge     | Given a file's `{size, mtime, fast_hash}` has not changed since the last harvest run, then that file is skipped — only changed files are re-parsed (incremental scan)                                                                                                                                                                                                |

---

#### US-43.2: Sentinel Selection & Resource Pack Integration

As a system, I want to select the best sentinel hashes per object using scoring and rarity rules, so that the runtime state machine can detect characters precisely with minimal per-frame overhead.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                                                                             |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-43.2.1 | ✅ Positive | Given a scored object match, sentinel selection picks top K hashes (K=1–3, default configurable) from the intersection `I = active_mod_hashes ∩ known_hashes(object)` ranked by: (1) rarity (fewer objects in Resource Pack own this hash), (2) more occurrences in active mods for the same object, (3) `hash_hints.role = ib\|position` if present |
| AC-43.2.2 | ✅ Positive | Given a score tie between two objects, the winner is: (1) higher `priority` field in Resource Pack entry, (2) higher score, (3) stable alphabetical order — fully deterministic                                                                                                                                                                      |
| AC-43.2.3 | ❌ Negative | Given `I` is empty (no intersection), fallback sentinels are chosen: top hashes from active mods by occurrence that are **not** `blacklisted_for_sentinel` — the object can still be detected via mod-extracted hashes alone                                                                                                                         |
| AC-43.2.4 | ⚠️ Edge     | Given a Resource Pack entry has `hash_hints[]` with `weight` overrides, those weights replace the `base = 10` default for those hashes in the scoring formula                                                                                                                                                                                        |

---

#### US-43.3: Dynamic Overlay Generation

As a user, I want the on-screen overlay to show only the keybinds for the character currently on screen, so that the overlay is always relevant and uncluttered.

| ID        | Type        | Criteria                                                                                                                                                                                                                                          |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-43.3.1 | ✅ Positive | Given hash harvest: (1) Artifact directories are **cleared** (`Zero-Leak Policy`); (2) `KeyViewer.ini` is generated; (3) `{code_hash}.txt` written per object; (4) `_fallback.txt` generated.                                                     |
| AC-43.3.2 | ✅ Positive | Given a keybind text file would exceed 8 KB or 60 lines (configurable), then content is safely truncated and `"..."` is appended — or paging is used if enabled.                                                                                  |
| AC-43.3.3 | ✅ Positive | Given multiple mods control the same character, the overlay displays a header for each mod followed by its specific keybinds, ensuring clarity on which mod owns which hotkey.                                                                    |
| AC-43.3.4 | ✅ Positive | Given the user swaps characters in-game, the game engine hits the new character's sentinel hash, `$kv_active_code` updates per the arbitration rules, and the `[Present]` overlay redraws the new character's keybind text within ≤ 1 frame       |
| AC-43.3.5 | ✅ Positive | Given Safe Mode is active, the generated `{code_hash}.txt` files exclude keybind entries from mods with `is_safe = false` — NSFW keybinds are never shown in the overlay.                                                                         |
| AC-43.3.6 | ❌ Negative | Given the primary PrintText pipeline (GIMI/SRMI renderer) is unavailable, `KeyViewer.ini` falls back to the legacy notification-style renderer — if both fail, `_fallback.txt` is shown with "KeyViewer renderer not available" and no error spam |
| AC-43.3.7 | ⚠️ Edge     | Given an Object has no associated keybinds in any enabled mod, its `{code_hash}.txt` is still generated as empty — the overlay shows nothing for that object; no missing-file error                                                               |

---

#### US-43.4: Runtime State Machine & Toggle

As a user, I want the overlay to respond precisely to which character is on screen and allow toggling visibility, so that I always see the right keybinds without cluttering the screen.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                                                                                                                             |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-43.4.1 | ✅ Positive | Given a sentinel hash hit for object `CX` with priority `P` at time `t`, the 5-step arbitration applies: (1) if stale (`t - $kv_active_last_seen_time > TTL_SECONDS=0.35`): set active=CX; (2) if `CX == $kv_active_code`: refresh last_seen; (3) if `P > $kv_active_priority`: set active=CX; (4) if `P == $kv_active_priority` AND `t > $kv_active_last_seen_time`: set active=CX; (5) else ignore |
| AC-43.4.2 | ✅ Positive | Given toggle key (default `H`) is pressed AND `has_active = (time - $kv_active_last_seen_time <= TTL)` is true, then `$kv_on` flips — overlay appears/hides within ≤ 1 frame                                                                                                                                                                                                                         |
| AC-43.4.3 | ✅ Positive | Given `$kv_on == 1` but `has_active` becomes false (no sentinel hit for > TTL), then the overlay auto-hides automatically                                                                                                                                                                                                                                                                            |
| AC-43.4.4 | ✅ Positive | Given anti-flipflop: once an object becomes active (`$kv_active_since_time` set), it cannot be replaced for `MIN_HOLD_SECONDS = 0.20s` unless: (a) old active becomes stale OR (b) new candidate has strictly higher priority AND `time - $kv_active_since_time >= MIN_HOLD_SECONDS`                                                                                                                 |
| AC-43.4.5 | ❌ Negative | Given toggle key pressed while `has_active == false` (no active character detected), then `$kv_on` does NOT change — overlay remains hidden; no state corruption                                                                                                                                                                                                                                     |
| AC-43.4.6 | ⚠️ Edge     | Given debug mode enabled, the `[Present]` block appends a footer showing: `$kv_active_code`, `age_ms` (time since last sentinel hit), and optionally `last_sentinel_hash`                                                                                                                                                                                                                            |

---

#### US-43.5: Regeneration Triggers & Atomicity

As a system, I want KeyViewer artifacts to be regenerated on any relevant state change and written atomically, so that the game never reads a partial file.

| ID        | Type        | Criteria                                                                                                                                                                                            |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-43.5.1 | ✅ Positive | Given any relevant event: (1) Collection apply, (2) Toggle/Rename/Delete mod, (3) Explorer change (FS Watcher), (4) Safe Mode toggle — then artifacts are regenerated automatically.                |
| AC-43.5.2 | ✅ Positive | Given any generated file write (`KeyViewer.ini` or `*.txt`), it is written to `*.tmp` then atomically renamed — the game engine never reads a partially-written file                                |
| AC-43.5.3 | ⚠️ Edge     | Given `KeyViewer.ini` is regenerated while 3DMigoto is actively reading it (mid-frame), the atomic rename guarantees the game reads either the old or the new complete file — never a partial state |

---

### Non-Goals

- No real-time rendering by EMMM — all overlay rendering is delegated to 3DMigoto's `[Present]` block.
- No manual editing of the generated `KeyViewer.ini` — it is always machine-generated and overwritten on mod state changes.
- No per-frame hash scanning by EMMM — all runtime hash detection is done by 3DMigoto sentinel sections.
- No support for non-3DMigoto games.
- Paging (PageDown/PageUp) is optional — if not enabled, content is truncated at 8KB/60 lines with `"..."` appended.

---

---

## 3. Architecture & Pipeline (The Big Picture)

### 🔁 The Big Picture

The EMMM Overlay System is a **hybrid architecture** where EMMM handles the complex data harvesting and artifact generation, while 3DMigoto handles high-performance, frame-accurate rendering using the `help.ini` pipeline.

1. **EMMM (The Brain)**: Scans `.ini` files, harvests character hashes, extracts keybinds, and generates static text assets + a logic-bridge (`KeyViewer.ini`).
2. **3DMigoto (The Muscle)**: Hooks the game's rendering pipeline. On every frame, it checks for "Sentinel Hashes". If a hash hits, it swaps the active text resource and renders it.

### 🚀 End-to-End Pipeline

| Step  | Component             | Action                                                                                                       | Result                                        |
| :---- | :-------------------- | :----------------------------------------------------------------------------------------------------------- | :-------------------------------------------- |
| **1** | **EMMM Backend**      | **Scan & Map**: Scans enabled mods for character hashes and extracts `[Key*]` sections.                      | `HashMap<Hash, Keybinds>`                     |
| **2** | **EMMM Generator**    | **Artifact Write**: Generates files into `Mods/.emmm_data/`.                                                 | `KeyViewer.ini`, `*.txt`                      |
| **3** | **EMMM App**          | **Reload Sync**: Sends the `reload_fixes` key to 3DMigoto.                                                   | Game reloads all `.ini` files.                |
| **4** | **3DMigoto Engine**   | **Hash Hit**: Game renders a character; `TextureOverride` triggers in `KeyViewer.ini`.                       | `$kv_has_active = 1`, `$kv_last_seen = time`. |
| **5** | **3DMigoto Present**  | **Arbitration**: `[Present]` block checks the 1.5s threshold and `$kv_active` toggle.                        | Correct `CommandList` is selected.            |
| **6** | **help.ini Pipeline** | **Render**: `FormatText` prints to a single `Notification` slot in `help.ini` for both KeyViewer and Status. | **Overlay Visible!**                          |

---

## 4. Technical Specifications

### Detection Logic (Persistence & Thresholds)

- **1.5s Threshold**: To prevent flickering during camera movements or animation breaks, the system maintains the active character state for **1.5 seconds** after the last sentinel hash hit.
- **Unified Toggle**: A single global variable `$kv_active` (toggled via `F7`) gates all rendering.
- **Priority Stack**: Rendering uses a single `Notification` slot in `help.ini`. If a character is detected (`$kv_has_active`), the character-specific keybinds are shown. Otherwise, if `$kv_active` is ON, the global "Status" banner is shown. This prevents overlay overlap.

### Runtime Regeneration Trigger Matrix

| Trigger | Owner | Regenerates KeyViewer | Emits `disk_reconcile:result` |
| --- | --- | --- | --- |
| Watcher / external rename-move-add-delete-enable-disable | Disk Reconcile | Yes | Yes |
| Window refocus / first Mods entry / game switch hydrate / manual repair | Disk Reconcile | Yes when runtime state changed | Yes |
| Explicit toggle / rename / move / delete mod from UI | Explicit runtime mutation service | Yes | No |
| `write_mod_ini` / `update_mod_info` | Disk Reconcile (`InternalMutation`) | Yes | Yes |
| Preview image / thumbnail-only mutation | Disk Reconcile (`InternalMutation`) | No | Yes |
| Safe mode / collection apply / corridor switch | Apply-switch pipeline | Yes | No |

---

## 5. File Specimens

### 📄 `Mods/.emmm_data/KeyViewer.ini` (Bridge Logic)

```ini
[Constants]
global $kv_active = 0           ; Master toggle (F7)
global $kv_has_active = 0       ; Reset every frame
global $kv_active_code = 0      ; Active character hash
global $kv_last_seen = 0        ; Persistence timestamp

[KeyEMMM_Toggle]
key = f7
type = cycle
$kv_active = 0, 1

[TextureOverride_EMM_Arlecchino_S0]
hash = a1b2c3d4
$kv_has_active = 1
$kv_active_code = 0xa1b2c3d4
$kv_last_seen = time

[Present]
post $kv_has_active = 0
if time - $kv_last_seen > 1.5
    $kv_active_code = 0
endif
run = CommandList_EMM_Render

[CommandList_EMM_Render]
[CommandList_EMM_Render]
if $kv_active == 1
    if $kv_has_active == 1
        ; KeyViewer has priority
        if $kv_active_code == 0xa1b2c3d4
            pre Resource\ShaderFixes\help.ini\Notification = ref ResourceKeyViewer_a1b2c3d4
        endif
    else
        ; Fallback to global status banner if no character detected
        pre Resource\ShaderFixes\help.ini\Notification = ref ResourceStatus
    endif
    pre Resource\ShaderFixes\help.ini\NotificationParams = ref ResourceBox
    run = CustomShader\ShaderFixes\help.ini\FormatText
endif
```

### 📄 `Mods/.emmm_data/keybinds/active/a1b2c3d4.txt` (Character Keybinds)

```text
Arlecchino
-----------
[Mod: Arlecchino_Base]
Key: CTRL+[
Back: NO_CTRL+ALT+]

[Mod: Arlecchino_FX]
Toggle: N

[F7] Toggle Overlay
```

### 📄 `Mods/.emmm_data/status/runtime_status.txt` (Global Banner)

```text
[EMM2] Safe: On | Collection: Maid Pack | Toggle: [F7]
```

---

## 6. Dependencies

- **Blocked by**: Epic 09, Epic 18, Epic 26, Epic 30, Epic 42.
- **Blocks**: Nothing — terminal advanced feature.
