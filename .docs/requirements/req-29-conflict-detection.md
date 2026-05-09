# Epic 29: Conflict Detection & Resolution

## 1. Executive Summary

- **Problem Statement**: 3DMigoto uses `hash = <hex>` keys in `.ini` files to intercept specific draw calls — two enabled mods with overlapping hashes produce visual glitches or game crashes; users have no in-app tool to detect these conflicts before launching.
- **Proposed Solution**: Three-tier conflict system: (1) **Implicit Variant Swap**: Toggling a variant within the same `VariantContainer` (same parent folder) disables the previous variant without warning; (2) **Try-First Duplicate Warning**: the workspace switch command/service returns a `DuplicateConflict` error if non-variant siblings are enabled, triggering an `ObjectConflictModal` for resolution or "Ignore Warning" persistence; (3) **Deep Hash Scanner**: A multi-threaded scanner parsers `.ini` files for exact `hash = <hex>` collisions.
- **Success Criteria**:
  - Duplicate Object check via workspace switch completes in ≤ 100ms (SQL index optimized).
  - Toggling a mod with "Enable Only Selected" resolution completes in ≤ 500ms (atomic transaction).
  - "Ignore Warning" persists the specific mod combination to the `ignored_object_conflicts` table.
  - Variant detection has 100% accuracy for mods inside a `VariantContainer` folder.

---

## 2. User Experience & Functionality

### User Stories

#### US-29.1: Duplicate Object Warning During Toggle

As a user, I want to be warned when I try to enable a second mod for the same Object, so that I don't accidentally cause visual overlap.

| ID        | Type        | Criteria                                                                                                                                                                                                   |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-29.1.1 | ✅ Positive | Given Object "Keqing" has one mod already enabled, when I toggle a second mod for "Keqing" on, then the workspace switch command returns a `DuplicateConflict` error containing the enabled mod's metadata |
| AC-29.1.2 | ✅ Positive | Given a `DuplicateConflict` error, then `ObjectConflictModal` shows: "Mod X is already enabled — Keep Only Selected (atomically disables siblings), or Ignore Warning (both stay on)"                      |
| AC-29.1.3 | ✅ Positive | Given "Ignore Warning" is selected, then the specific `(object_id, mod_ids)` combination is persisted to the DB; subsequent toggles of this set skip the warning                                           |
| AC-29.1.4 | ✅ Positive | Given a mod is inside a `VariantContainer` (Epic 11), when it is toggled while a sibling variant is enabled, then the sibling is disabled _silently_ without a conflict prompt                             |
| AC-29.1.5 | ❌ Negative | Given any mod combination is "Ignored", then the user can view and revoke this status in the `IgnoreManagementModal` (Settings > Advanced > Ignored Conflicts)                                             |
| AC-29.1.4 | ⚠️ Edge     | Given an Object has 3 mods enabled (e.g., from a past bulk import), when a 4th is toggled with "Enable Only This", then all 3 are disabled atomically and only the 4th is enabled                          |

---

#### US-29.2: "Enable Only This" Quick Switch

As a user, I want a one-click way to switch between two character skins, so that I don't have to manually disable one and enable the other.

| ID        | Type        | Criteria                                                                                                                                                                                                                |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-29.2.1 | ✅ Positive | Given the context menu for any mod in an Object, clicking "Enable Only This" disables all currently enabled mods in the same Object and enables the target mod — all inside one atomic `OperationLock` + DB transaction |
| AC-29.2.2 | ✅ Positive | Given the atomic swap completes, then the grid reflects the new enabled state and the objectlist `enabled_count = 1` for that Object — both update via React Query cache invalidation                                   |
| AC-29.2.3 | ⚠️ Edge     | Given "Enable Only This" is used on a mod that is the only enabled mod in its Object, then the operation is no-op — the mod stays enabled, no extra renaming occurs                                                     |

---

#### US-29.3: Deep Shader Hash Conflict Scanner

As a user, I want the app to scan all active mods for colliding texture override hashes, so that I can identify incompatible mods from different Objects.

| ID        | Type        | Criteria                                                                                                                                                                                                                       |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-29.3.1 | ✅ Positive | Given the user triggers a global conflict scan, then the scanner parses all enabled `.ini` files for `[TextureOverride...]` sections and builds a `hash → [{mod_path, section}]` map; any hash with ≥ 2 entries is a collision |
| AC-29.3.2 | ✅ Positive | Given a detected collision, the UI shows a `ConflictModal` with: conflicting hash, section name, and paths of the conflicting mod folders                                                                                      |
| AC-29.3.3 | ❌ Negative | Given 0 hash collisions are found, then the conflict modal (or trigger) indicates no shader conflicts were detected                                                                                                            |
| AC-29.3.4 | ⚠️ Edge     | Given a malformed `.ini` (missing `=`, binary content), then the scanner skips that file with a `warn` log — it does not abort the entire scan                                                                                 |

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
detect_conflicts_service(game_id, folder_path) → Result<(), CommandError>:
  1. Determine `object_id` from `folder_path`.
  2. IDENTIFY siblings: `SELECT * FROM mods WHERE object_id = ? AND status = 'ENABLED'`.
  3. VARIANT CHECK: If `folder_path` and a sibling share a `VariantContainer` parent, PERFORM "Implicit Swap" (disable sibling, enable target) -> return Ok.
  4. IGNORE CHECK: If `(object_id, target_path, sibling_paths)` is in `ignored_object_conflicts` -> return Ok.
  5. CONFLICT: return `Err(DuplicateConflict(siblings))`.

Frontend:
  ObjectConflictModal.tsx (shown for DuplicateConflict errors)
  IgnoreManagementModal.tsx (lists and revokes ignored entries)
  ConflictScanReport.tsx (shown after deep hash scan)
```

### Integration Points

| Component         | Detail                                                                                  |
| ----------------- | --------------------------------------------------------------------------------------- |
| Duplicate Check   | Embedded in workspace switch flow — returns error if non-variant duplicates are enabled |
| Persistent Ignore | `ignored_object_conflicts` table stores (game_id, object_id, mod_ids_hash)              |
| Variant Support   | Backend detects shared `VariantContainer` folders and performs implicit swapping        |
| INI Parser        | `services/ini/document.rs` (Epic 18) — reused for hash extraction                       |
| Deep Scan         | `conflict_cmds.rs::check_shader_conflicts` → multi-threaded Tokio task                  |
| Frontend          | `useAppStore` handles `DuplicateConflict` globally → opening `ObjectConflictModal`      |

### Security & Privacy

- **All folder paths used in `enable_only_this`** are loaded from DB (not from user input) — no path injection possible.
- **`OperationLock` scope covers all renames in `enable_only_this`** — the entire multi-mod disable + single-mod enable is atomic.
- **Deep scan is read-only** — it only reads `.ini` files, never writes or renames during conflict detection.

---

## 4. Dependencies

- **Blocked by**: Epic 20 (Mod Toggle — conflict check is part of toggle flow), Epic 18 (INI Viewer — INI parser reused for hash extraction), Epic 28 (File Watcher — WatcherSuppression for enable_only_this).
- **Blocks**: Nothing — conflict detection is a terminal diagnostic feature.
