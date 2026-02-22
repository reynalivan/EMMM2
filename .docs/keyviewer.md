# Hash-Gated KeyViewer Requirements

## 0) Goal

Provide an **in-game KeyViewer overlay** for 3DMigoto-based games that:

- Shows keybinds **only for the currently detected character/object** (based on **live in-game hash hits**)
- Can be **shown/hidden via a toggle shortcut**
- Updates automatically when **mods/presets are enabled/disabled** (via EMM2 regeneration + in-game reload)
- Remains **efficient** (no polling, no heavy per-frame logic)

---

## 1) Core Design (2-Layer Hash Sources + 1 Cache)

### 1.1 Resource Pack (per game) — _Canonical reference, not a DB_

Files like `gimi.json / srmi.json / wwmi.json` in folder `\src-tauri\resources\databases` act as a **dictionary**:

- Defines what objects exist and their stable EMM2 identity
- Stores curated, known hash lists to strengthen matching

### 1.2 Active Mods — _Ground truth_

Enabled mods’ `.ini` files are the **authoritative current hash reality** (covers hash updates).

### 1.3 EMM2 Cache/DB — _State + extracted knowledge_

A persistent store (SQLite recommended; JSON cache acceptable) used for:

- Keybind metadata per mod and per preset
- Extracted hash occurrences from mods (incremental)
- Mapping results (object ↔ selected sentinels) with confidence + provenance
- Learned hashes (new hashes seen in active mods not yet present in resource pack)

---

## 2) Data Contracts

### 2.1 Resource Pack Entry Schema (per object/character)

Minimum:

- `name` (display name)
- `object_type` (Character / Weapon / UI / etc.)
- `tags[]`
- `**code_hash**`: uint32 (store as hex string like `"0xA1B2C3D4"` or integer)
  - Must be unique per `game_id`

- `priority`: int (higher wins in arbitration)

Hash lists:

- `known_hashes[]`: array of 3DMigoto hashes (shader/texture) associated with the object
  - Recommended: 3–20 (not all used at runtime)

- Optional: `hash_hints[]` items with weights/types:
  - `{ hash, kind: "shader"|"texture"|"unknown", role: "ib"|"position"|"misc", weight }`

Optional:

- `game_id`
- `thumbnail_path`
- `metadata`
- `notes`

### 2.2 EMM2 Cache/DB Tables (logical)

Required concepts (implementation flexible):

- `mods`: mod identity + version signature
- `mod_hash_index`: `{hash -> {mod_id, file, section, count}}`
- `object_sentinel_cache`: `{code_hash -> {sentinel_hashes[], confidence, last_updated, sources[]}}`
- `keybinds`: extracted keybinds from enabled mods
- `presets`: enabled mods list + user overrides/remaps

### 2.3 Generated Runtime Files

- `Mods/.EMMM2_System/KeyViewer.ini` (system mod, always loaded)
- `EMM2/keybinds/active/<code_hash>.txt` (per object for current preset)
- `EMM2/keybinds/active/_fallback.txt`

---

## 3) Offline Pipeline (EMM2) — End-to-End

### 3.1 Trigger: when to regenerate

Regeneration happens when any of these change:

- Apply/switch preset
- Enable/disable mod
- Update mod files
- Change keybind overrides/remaps
- Update resource pack entries (e.g., new `known_hashes`)

### 3.2 Step A — Harvest hashes from active mods (fast path)

Parse enabled mods’ `.ini` files and collect `hash = XXXXXXXX` occurrences with **strict rules** to avoid noise.

**Scope rules (by default):**

- Only scan **enabled mods in the active runtime workspace** (the same set that 3DMigoto will include).
- Prefer scanning under the character/object folder currently managed by EMM2 (e.g., `Mods/<CharacterName>/...`).

**Section rules (hash candidates):**

- Include hashes from sections whose names match:
  - `TextureOverride*`, `ShaderOverride*`, `TextureOverrideIB*`, `ShaderOverridePS*`, `ShaderOverrideVS*` (configurable)

- Exclude hashes from known shared/global patterns (configurable denylist):
  - UI/common overlays, global post-process, known shared shader packs

**Collection rules:**

- Record for each hash:
  - `occurrence_count_total`
  - `occurrence_count_by_file`
  - `section_names_seen[]`
  - `source_mod_ids[]`
  - `first_seen/last_seen` timestamps

- Treat multiple occurrences in the same file/section as a single hit for rarity purposes, but count them for “core-ness” scoring.

**Incremental scanning:**

- Each scanned file has a `file_signature` (recommended: `{size, mtime, fast_hash}`), only rescan when signature changes.

**Atomic write rule:**

- When writing any derived artifact (cache index or generated files), EMM2 must write to `*.tmp` then rename (atomic swap).

### 3.3 Step B — Match active-mod hashes to Resource Pack objects — Match active-mod hashes to Resource Pack objects

For each candidate object (preferably only objects relevant to enabled mods/preset):

- Compute intersection: `I = active_mod_hashes ∩ known_hashes(object)`
- Compute match score:
  - `score = Σ weight(hash)` for `hash ∈ I`

Recommended weighting (simple + effective):

- `base = 10` per intersecting hash
- `+ log(1 + occurrence_count(hash))` (hash frequent in active mods is stronger)
- `+ rarity_bonus` (hash seen in fewer objects across resource pack is stronger)
- `+ hint_bonus` from `hash_hints.role/kind` if available

Decision rule:

- Pick best object if `score >= threshold` (threshold configurable; default requires at least 1–2 strong intersections)
- If tie: prefer higher `priority`, then higher score, then stable order.

### 3.4 Step C — Choose sentinel hashes for runtime detection

For the chosen object:

- Primary sentinels: top K hashes from intersection `I` by weight (K default 1–3)
- Fallback sentinels (if `I` is empty): choose top hashes from active mods by occurrence that are not high-collision

**High-collision definition (must be numeric):** A hash is considered **high-collision** and must not be used as a sentinel if ANY of the following is true:

- Appears in `known_hashes` of **≥ 3** different objects in the resource pack (default; configurable)
- OR appears in harvested active-mod hashes across **≥ 2** different `code_hash` candidates with score margin < `MIN_MARGIN` (default 15%)

**Sentinel stability preferences (tie-breaking):**

- Prefer hashes that:
  1.  have higher rarity (seen in fewer objects)
  2.  appear in more sections for the same object in active mods
  3.  are hinted as `role = ib/position` if present in `hash_hints`

Collision handling:

- If a hash collides, mark it in cache as `blacklisted_for_sentinel = true` with reason and counters.

Persist:

- Write selected `sentinel_hashes[]` + confidence + provenance into cache (`object_sentinel_cache`).
- If new hashes appear, store them as **learned hashes** and optionally suggest adding to resource pack.

### 3.5 Step D — Build Keybind text for current preset — Build Keybind text for current preset

From enabled mods’ keybind metadata:

- Group actions by `code_hash`
- Generate `EMM2/keybinds/active/<code_hash>.txt`
- Always generate `_fallback.txt`

Content rules:

- Include only enabled mods
- Optional: include preset name + warning section for conflicts

### 3.6 Step E — Generate KeyViewer.ini (runtime mapping)

Generate `Mods/.EMMM2_System/KeyViewer.ini` containing:

- Toggle key behavior
- Runtime state machine and TTL
- Sentinel override sections for only relevant objects (current preset scope)
- Mapping: `$kv_active_code -> Resource(<code_hash>.txt)`

### 3.7 Step F — Reload handshake

Because 3DMigoto typically requires reload to pick up changes:

**Reload key discovery (must be robust):**

- EMM2 must attempt to read the user’s configured reload key from `d3dx.ini` (e.g., `reload_fixes` binding) if accessible.
- If not found, default to `F10` and clearly label it as a fallback.

**UX contract:**

- EMM2 must surface a prominent CTA: **“Reload in-game ()”** immediately after regeneration.
- Optional: best-effort “Send reload key to focused game window” (never required; must fall back to CTA if blocked).

---

## 4) Runtime Behavior (3DMigoto) — Efficient State Machine

### 4.1 Globals

- `$kv_on` (0/1 toggle)
- `$kv_active_code` (uint32 `code_hash`)
- `$kv_active_priority` (int)
- `$kv_active_last_seen_time` (float)
- `TTL_SECONDS` (default 0.35s)

### 4.2 Event-driven detection via sentinel hits

For each sentinel hash section (generated):

- On hash hit for object code `CX` with priority `P` at time `t`, apply arbitration (O(1)):

**Arbitration (deterministic + stable):**

1.  If active is stale (`t - $kv_active_last_seen_time > TTL_SECONDS`): set active = `CX`, set priority = `P`, set last_seen=t
2.  Else if `CX == $kv_active_code`: refresh last_seen=t
3.  Else if `P > $kv_active_priority`: set active = `CX` (priority wins)
4.  Else if `P == $kv_active_priority` and `t > $kv_active_last_seen_time`: set active = `CX` (last-hit wins)
5.  Else ignore

### 4.3 Toggle gating

- Toggle key (default `H`) toggles `$kv_on` **only if** `has_active == true`.
- `has_active := (time - $kv_active_last_seen_time <= TTL_SECONDS)`
- If `has_active == false`: do nothing (strict) or show brief message (optional).

### 4.4 Overlay binding

If `$kv_on == 1` AND `has_active == 1`:

- Display `EMM2/keybinds/active/<$kv_active_code>.txt` Else:
- Hide overlay

**Renderer Adapter (must be specified):** KeyViewer must support at least one primary renderer backend and one fallback:

- Primary: the latest available PrintText pipeline in the user’s loader stack (e.g., Core/GIMI style)
- Fallback: legacy notification-style pipeline (help.ini-like)

**Text constraints (hard limits to prevent failures):**

- Encoding: ASCII/UTF-8 without exotic control chars
- Max bytes per file: configurable, default **8 KB**
- Max lines: configurable, default **60**
- If content exceeds limits:
  - Truncate safely and append `"..."`
  - Or use **paging** if enabled (see UX-4)

**Missing resource behavior:**

- If the renderer backend or font resource is missing, KeyViewer must fall back to `_fallback.txt` with a clear message and must not spam errors.

### 4.5 Auto-hide

If `$kv_on == 1` but `has_active` becomes false:

- Overlay hides automatically

### 4.6 Anti flip-flop (hysteresis)

To prevent rapid switching when multiple objects are visible:

- Define `MIN_HOLD_SECONDS` (default **0.20s**)
- Once an object becomes active, it cannot be replaced by another object unless:
  - Active becomes stale (TTL exceeded), OR
  - New candidate has higher priority AND `time - $kv_active_since_time >= MIN_HOLD_SECONDS`, OR
  - Same priority AND last-hit wins AFTER `MIN_HOLD_SECONDS`

- Maintain `$kv_active_since_time` when setting active.

---

## 5) Efficiency / Performance Requirements

### NFR-1 — No polling

- No scanning all objects/hashes per frame.
- Only reacts to actual hash hits.

### NFR-2 — Minimal per-hit work

- Sentinel hit work is constant-time and only sets a few globals.

### NFR-3 — Minimal per-frame work

In `Present`:

- TTL check
- one resource selection
- one overlay draw call when visible

### NFR-4 — Sentinel limits

- Runtime sentinels per object: 1–3 (default)
- Total sentinels generated should be scoped to current preset’s relevant objects.

### NFR-5 — Offline heavy lifting only

- Parsing/mod scanning and text generation are offline EMM2 tasks.

---

## 6) UX Requirements

- Overlay text is readable and grouped (Variants / FX / UI / Debug)
- Optional: show preset name
- Optional: show conflict warnings
- Optional: Safe Mode omits NSFW keybinds in generated text

### UX-4 Paging (optional but recommended)

If `Max lines/bytes` would be exceeded:

- Enable paging with keys (defaults):
  - `PageDown`: next page
  - `PageUp`: previous page

- Page state is reset when active object changes.

### UX-5 Debug overlay (optional)

When enabled, append a footer showing:

- `$kv_active_code`
- `age_ms` (time since last sentinel hit)
- `last_sentinel_hash` (optional)

---

## 7) Acceptance Criteria (Testable)

### AC-1 Toggle gated

Given no sentinel hit within TTL, When user presses toggle, Then KeyViewer does not remain visible.

### AC-2 Toggle works when detected

Given a sentinel hit within TTL, When user presses toggle, Then overlay appears and shows `<code_hash>.txt` for the active object.

### AC-3 Toggle off

Given overlay visible, When user presses toggle again, Then overlay hides within ≤ 1 frame.

### AC-4 Object swap live update

Given overlay visible on object A, When object B sentinel hits within TTL, Then overlay updates to object B within ≤ 1 frame.

### AC-5 Auto-hide on stale

Given overlay visible, When no sentinel hits happen for > TTL, Then overlay hides.

### AC-6 Priority stability

Given A priority 10 and B priority 5 both hit within TTL, Then active remains A until stale.

### AC-7 Tie-breaker

Given A and B equal priority, When B hits after A, Then B becomes active.

### AC-8 Anti flip-flop

Given A becomes active, When B hits repeatedly within TTL, Then A remains active for at least `MIN_HOLD_SECONDS` unless A becomes stale or B has higher priority and the hold time has elapsed.

### AC-9 Preset refresh correctness

Given preset P1 enables mods set S1, When switch to preset P2 (mods set S2) and reload in-game, Then generated overlay shows only S2 keybinds and never includes disabled mods.

### AC-10 Hash update resilience

Given a mod update changes hashes, When preset applied again, Then matching uses active-mod hashes as ground truth and updates sentinels; overlay still works after reload.

### AC-11 Renderer fallback safety

Given the primary PrintText pipeline is unavailable, When KeyViewer toggles on, Then it falls back to the legacy renderer (or `_fallback.txt`) without error spam.

### AC-12 Atomic artifact safety

Given EMM2 regenerates `.txt` or `KeyViewer.ini`, When a write is interrupted, Then runtime never reads a partially-written file (only old or new complete version).
