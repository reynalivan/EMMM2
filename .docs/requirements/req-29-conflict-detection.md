# Epic 29: Conflict Detection & Resolution

## 1. Executive Summary

- **Problem Statement**: 3DMigoto uses `hash = <hex>` keys in `.ini` files to intercept specific draw calls — two enabled mods with overlapping hashes produce visual glitches or game crashes; users have no in-app tool to detect these conflicts before launching.
- **Proposed Solution**: Two-tier conflict system: (1) a fast "duplicate Object" check during toggle (same Object, more than one mod enabled), surfacing a `ConflictResolveDialog` with "Enable Only This" action; (2) a deep hash scanner that parses all enabled `.ini` files and builds a `hash → {mod, line}` map, reporting every collision with exact file/line detail.
- **Success Criteria**:
  - Duplicate Object check completes in ≤ 50ms (DB `COUNT` query + in-memory check).
  - "Enable Only This" atomically disables all other enabled mods in the same Object and enables the target in ≤ 500ms.
  - Deep hash scan over 100 enabled mods (avg 2 `.ini` each, ~200 files) completes in ≤ 10s on SSD.
  - Hash conflict report correctly identifies colliding `[TextureOverride]` sections with ≥ 95% accuracy against a 20-mod benchmark set of known conflicting mods.
  - Zero false positives from the duplicate Object warning when only one mod is enabled per Object.

---

## 2. User Experience & Functionality

### User Stories

#### US-29.1: Duplicate Object Warning During Toggle

As a user, I want to be warned when I try to enable a second mod for the same Object, so that I don't accidentally cause visual overlap.

| ID        | Type        | Criteria                                                                                                                                                                                                        |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-29.1.1 | ✅ Positive | Given Object "Keqing" has one mod already enabled, when I toggle a second mod for "Keqing" on, then `check_duplicate_enabled` returns `DuplicateInfo` containing the name and path of the currently enabled mod |
| AC-29.1.2 | ✅ Positive | Given a `DuplicateInfo` warning, then `ConflictResolveDialog` shows: "Mod X is already enabled for Keqing — Enable Only This (disabling X), or Enable Anyway (both on)"                                         |
| AC-29.1.3 | ❌ Negative | Given "Enable Anyway" is selected, then both mods remain enabled — no further action; user accepts the risk                                                                                                     |
| AC-29.1.4 | ⚠️ Edge     | Given an Object has 3 mods enabled (e.g., from a past bulk import), when a 4th is toggled with "Enable Only This", then all 3 are disabled atomically and only the 4th is enabled                               |

---

#### US-29.2: "Enable Only This" Quick Switch

As a user, I want a one-click way to switch between two character skins, so that I don't have to manually disable one and enable the other.

| ID        | Type        | Criteria                                                                                                                                                                                                                |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-29.2.1 | ✅ Positive | Given the context menu for any mod in an Object, clicking "Enable Only This" disables all currently enabled mods in the same Object and enables the target mod — all inside one atomic `OperationLock` + DB transaction |
| AC-29.2.2 | ✅ Positive | Given the atomic swap completes, then the grid reflects the new enabled state and the objectlist `enabled_count = 1` for that Object — both update via React Query cache invalidation                                      |
| AC-29.2.3 | ⚠️ Edge     | Given "Enable Only This" is used on a mod that is the only enabled mod in its Object, then the operation is no-op — the mod stays enabled, no extra renaming occurs                                                     |

---

#### US-29.3: Deep Shader Hash Conflict Scanner

As a user, I want the app to scan all active mods for colliding texture override hashes, so that I can identify incompatible mods from different Objects.

| ID        | Type        | Criteria                                                                                                                                                                                         |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-29.3.1 | ✅ Positive | Given the user triggers a global conflict scan, then the scanner parses all enabled `.ini` files and builds a `hash → [{mod_path, section, line}]` map; any hash with ≥ 2 entries is a collision |
| AC-29.3.2 | ✅ Positive | Given a detected collision, the UI shows: conflicting hash, both mod names, both INI file paths, and exact line numbers — enough information to manually resolve                                 |
| AC-29.3.3 | ❌ Negative | Given 0 hash collisions are found, then the conflict report shows "No conflicts detected — all enabled mods are compatible"                                                                      |
| AC-29.3.4 | ⚠️ Edge     | Given a malformed `.ini` (missing `=`, binary content), then the scanner skips that file with a `warn` log — it does not abort the entire scan                                                   |

---

### Non-Goals

- No automatic conflict auto-resolution — system detects and reports; users decide what to disable.
- Conflict scanner only reads enabled mods — disabled folders are not parsed.
- No real-time conflict check on each INI save (Epic 18 edit) — only on explicit scan trigger.
- No "Fix All" button — user must resolve conflicts one by one.

---

## 3. Technical Specifications

### Architecture Overview

```
check_duplicate_enabled(game_id, object_id) → Option<Vec<DuplicateInfo>>:
  SELECT folder_path, name FROM folders WHERE object_id = ? AND is_enabled = true
  → if count > 0: return Some(Vec<DuplicateInfo>)

enable_only_this(game_id, target_folder_path) → ():
  1. Acquire OperationLock(game_id) + WatcherSuppression(all paths in object)
  2. SELECT folder_path FROM folders WHERE object_id = target.object_id AND is_enabled = true
  3. For each currently_enabled != target: rename to "DISABLED " + name
  4. If target is disabled: rename to strip "DISABLED "
  5. UPDATE is_enabled flags in DB atomically
  6. Return Ok(())

detect_conflicts_in_folder_cmd(game_id) → Vec<ConflictReport>:
  1. Collect all enabled mod paths → parse each .ini with ini_parser
  2. Build: HashMap<hash_string, Vec<ConflictEntry { path, section_name, line }>>
  3. Return entries where vec.len() >= 2

Frontend:
  ConflictResolveDialog.tsx (shown for DuplicateInfo errors)
  ConflictScanReport.tsx (shown after deep scan, lists all ConflictReport entries)
```

### Integration Points

| Component        | Detail                                                                                              |
| ---------------- | --------------------------------------------------------------------------------------------------- |
| Duplicate Check  | Hook into `toggle_mod` flow — called before `fs::rename`, inside `OperationLock`                    |
| Enable Only This | `conflict_cmds.rs::enable_only_this` — shares `OperationLock` + `WatcherSuppression` with toggle    |
| INI Parser       | `services/ini/document.rs` (Epic 18 INI parser) — reused for hash extraction                        |
| Deep Scan        | `conflict_cmds.rs::detect_conflicts_in_folder_cmd` → background Tokio task                          |
| Frontend         | `useFolderGridActions.ts` catches `CommandError::DuplicateConflict` → opens `ConflictResolveDialog` |

### Security & Privacy

- **All folder paths used in `enable_only_this`** are loaded from DB (not from user input) — no path injection possible.
- **`OperationLock` scope covers all renames in `enable_only_this`** — the entire multi-mod disable + single-mod enable is atomic.
- **Deep scan is read-only** — it only reads `.ini` files, never writes or renames during conflict detection.

---

## 4. Dependencies

- **Blocked by**: Epic 20 (Mod Toggle — conflict check is part of toggle flow), Epic 18 (INI Viewer — INI parser reused for hash extraction), Epic 28 (File Watcher — WatcherSuppression for enable_only_this).
- **Blocks**: Nothing — conflict detection is a terminal diagnostic feature.
