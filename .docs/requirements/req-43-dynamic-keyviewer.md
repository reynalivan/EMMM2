# Epic 43: Dynamic KeyViewer Overlay

## 1. Executive Summary

- **Problem Statement**: Users who have tens of mods enabled across multiple characters have no in-game reference for which keybinds control which mod — displaying all keybinds at once clutters the screen; showing nothing requires memorization.
- **Proposed Solution**: A two-layer, hash-gated system: (1) an **offline EMM2 pipeline** that harvests hashes from active mod `.ini` files, scores them against a Resource Pack (`gimi.json`, `srmi.json`, etc.), selects per-object sentinel hashes, generates per-object keybind text files and a `KeyViewer.ini`, and writes all artifacts atomically; (2) a **3DMigoto runtime state machine** that reacts to sentinel hash hits on rendered frames, applies 5-step priority arbitration with hysteresis, and conditionally draws the relevant keybind text overlay via PrintText adapter.
- **Success Criteria**:
  - `generate_keyviewer_ini` runs in ≤ 500ms for ≤ 200 enabled mods.
  - All generated artifacts (`KeyViewer.ini`, `<code_hash>.txt`) are written atomically — `*.tmp` → rename swap — preventing the game from reading a partial file.
  - Hash harvest completes in ≤ 2s for 200 enabled mods (incremental: only files whose `{size, mtime, fast_hash}` changed are re-parsed).
  - Anti-flipflop holds `MIN_HOLD_SECONDS = 0.20s` — prevents `$kv_active_code` switches faster than 5 Hz.
  - Runtime sentinel hit work is O(1) — no per-frame scanning; only a TTL check + one resource read + one draw call in `[Present]`.
  - Overlay is fully toggled off within ≤ 1 frame of pressing the toggle key.

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
| AC-43.1.5 | ❌ Negative | Given two different enabled mods hash to the same Object (same character, two skins both enabled), then only one sentinel set is generated for that Object — duplicate hash sources are merged under that `code_hash`                                                                                                                                                |
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

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-43.3.1 | ✅ Positive | Given hash harvest produces N object mappings, `generate_keyviewer_ini` outputs: (1) `Mods/.EMMM2_System/KeyViewer.ini` with 5 runtime globals, one `[TextureOverride]` sentinel section per object (1–3 hashes each), and one `[Present]` block per object gated by `if $kv_on == 1 && has_active && $kv_active_code == {code_hash}`; (2) `EMM2/keybinds/active/{code_hash}.txt` per object (grouped: Variants / FX / UI / Debug); (3) `EMM2/keybinds/active/_fallback.txt` |
| AC-43.3.2 | ✅ Positive | Given a keybind text file would exceed 8 KB or 60 lines (configurable), then content is safely truncated and `"..."` is appended — or paging is used if enabled (PageDown/PageUp cycle pages; page state resets on object change)                                                                                                                                                                                                                                            |
| AC-43.3.3 | ✅ Positive | Given the user swaps characters in-game, the game engine hits the new character's sentinel hash, `$kv_active_code` updates per the arbitration rules, and the `[Present]` overlay redraws the new character's keybind text within ≤ 1 frame                                                                                                                                                                                                                                  |
| AC-43.3.4 | ✅ Positive | Given Safe Mode is active, the generated `{code_hash}.txt` files exclude keybind entries from mods with `is_safe = false` — NSFW keybinds are never shown in the overlay                                                                                                                                                                                                                                                                                                     |
| AC-43.3.5 | ❌ Negative | Given the primary PrintText pipeline (GIMI/SRMI renderer) is unavailable, `KeyViewer.ini` falls back to the legacy notification-style renderer — if both fail, `_fallback.txt` is shown with "KeyViewer renderer not available" and no error spam                                                                                                                                                                                                                            |
| AC-43.3.6 | ⚠️ Edge     | Given an Object has no associated keybinds in any enabled mod, its `{code_hash}.txt` is still generated as empty — the overlay shows nothing for that object; no missing-file error                                                                                                                                                                                                                                                                                          |

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

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                               |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-43.5.1 | ✅ Positive | Given any of these events: (1) apply/switch preset, (2) enable/disable mod, (3) update mod files, (4) change keybind overrides/remaps, (5) update Resource Pack entries, (6) Safe Mode toggle — then `generate_keyviewer_ini` is triggered within ≤ 1s if KeyViewer feature is enabled |
| AC-43.5.2 | ✅ Positive | Given any generated file write (`KeyViewer.ini` or `*.txt`), it is written to `*.tmp` then atomically renamed — the game engine never reads a partially-written file                                                                                                                   |
| AC-43.5.3 | ⚠️ Edge     | Given `KeyViewer.ini` is regenerated while 3DMigoto is actively reading it (mid-frame), the atomic rename guarantees the game reads either the old or the new complete file — never a partial state                                                                                    |

---

### Non-Goals

- No real-time rendering by EMMM2 — all overlay rendering is delegated to 3DMigoto's `[Present]` block.
- No manual editing of the generated `KeyViewer.ini` — it is always machine-generated and overwritten on mod state changes.
- No per-frame hash scanning by EMMM2 — all runtime hash detection is done by 3DMigoto sentinel sections.
- No support for non-3DMigoto games.
- Paging (PageDown/PageUp) is optional — if not enabled, content is truncated at 8KB/60 lines with `"..."` appended.

---

## 3. Technical Specifications

### Architecture Overview

```
Resource Pack (per game):
  {game_id}/gimi.json → [{ name, object_type, code_hash (uint32/hex), priority,
                            known_hashes[], hash_hints[{hash, kind, role, weight}],
                            tags[], thumbnail_path? }]

EMM2 Cache (SQLite):
  mod_hash_index: { hash → {mod_id, file, section, occurrence_count, source_mod_ids[]} }
  object_sentinel_cache: { code_hash → {sentinel_hashes[], confidence, last_updated, sources[]} }
  keybinds: { mod_id, code_hash, key_label, back_key, action_description }

Offline pipeline (triggered by any of 6 events):

Step A: harvest_hashes(enabled_mods) → HashMap<hash, HarvestEntry>
  for each enabled *.ini (skip if {size, mtime, fast_hash} unchanged):
    scan only [TextureOverride*], [ShaderOverride*] sections
    extract hash = XXXXXXXX lines
    update mod_hash_index

Step B: match_to_objects() → Vec<ObjectMatch { code_hash, score, sentinel_hashes[] }>
  for each Resource Pack object:
    I = active_mod_hashes ∩ known_hashes(object)
    score = Σ (10 + log(1 + occurrence_count) + rarity_bonus + hint_bonus) for hash in I
    if score >= threshold: candidate
  sort by (score DESC, priority DESC, stable)

Step C: select_sentinels(object_match) → Vec<hash> (1–3)
  from I: pick by (rarity DESC, occurrence_count DESC, role=ib|position priority)
  skip hashes with blacklisted_for_sentinel = true
  high-collision check: if hash in known_hashes of ≥3 objects → blacklist

Step D: generate_keybind_texts(code_hashes[]) → Vec<(code_hash, text)>
  for each code_hash: group keybinds from enabled mods by code_hash
  content rules: max 8KB, max 60 lines; truncate + "..." or page (PageDown/PageUp)
  write atomically: EMM2/keybinds/active/{code_hash}.tmp → rename
  write _fallback.txt atomically

Step E: generate_keyviewer_ini(sentinels_map) → ()
  header:
    global $kv_on = 0
    global $kv_active_code = 0
    global $kv_active_priority = 0
    global $kv_active_last_seen_time = 0
    global $kv_active_since_time = 0
    TTL_SECONDS = 0.35
    MIN_HOLD_SECONDS = 0.20

  for each (code_hash, sentinel_hashes, priority):
    [TextureOverrideKV_{code_hash}_S{i}]
    hash = {sentinel}
    → 5-step arbitration logic (sets globals)

  [Present]:
    for each code_hash:
      if $kv_on == 1 && has_active && $kv_active_code == {code_hash}:
        draw EMM2/keybinds/active/{code_hash}.txt
    if debug_mode: draw footer ($kv_active_code, age_ms)

  write atomically: Mods/.EMMM2_System/KeyViewer.ini.tmp → rename

Step F: reload_handshake()
  read d3dx.ini → find reload_fixes binding (fallback: F10)
  emit CTA in UI: "Reload in-game (F10)" or auto-send via enigo if enabled
```

### Integration Points

| Component      | Detail                                                                                                                                    |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Resource Pack  | `src-tauri/resources/databases/{game_id}.json` — bundled, updated via Epic 34 dynamic asset sync                                          |
| Hash Extractor | Shared with Epic 26 (Deep Matcher) — same `extract_hashes_from_ini_text` function                                                         |
| File Signature | `{size_bytes, mtime, blake3_fast_hash}` — incremental scan key stored in `mod_hash_index`                                                 |
| Atomic Write   | `.tmp` → `rename` — identical pattern to Epic 42 `runtime_status.txt`                                                                     |
| Trigger Events | `toggle_mod_enabled`, `apply_collection`, file watcher events, Safe Mode toggle, keybind override change                                  |
| Paging         | `PageDown`/`PageUp` bound in `KeyViewer.ini` `[KeyPageDown]`/`[KeyPageUp]` sections; page index global resets on `$kv_active_code` change |

### Security & Privacy

- **`KeyViewer.ini` and `*.txt` paths scoped to `mods_path/.EMMM2_System/` and `EMM2/keybinds/active/`** — validated with `starts_with(mods_path)`.
- **Atomic write** prevents partial file reads by 3DMigoto mid-frame.
- **Safe Mode filter applied at Step D** — `is_safe = false` mod keybinds are never written to `{code_hash}.txt` files.
- **High-collision blacklist** prevents ambiguous hashes from poisoning the sentinel detection — stored in `mod_hash_index` with reason + counters.

---

## 4. Dependencies

- **Blocked by**: Epic 09 (Object Schema/Resource Pack — `code_hash` identity), Epic 18 (INI Parser — hash extraction), Epic 26 (Deep Matcher — shared `extract_hashes_from_ini_text`), Epic 30 (Safe Mode — `is_safe` filter for keybind text generation), Epic 42 (In-Game Hotkeys — toggle key + `runtime_status.txt` write convention).
- **Blocks**: Nothing — terminal advanced feature.
