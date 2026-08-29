# Epic 30: Privacy & Safe Mode (Lightweight View Filter)

> **[2026-08-29] Catatan arsitektur:** Desain "Dual Corridor Handoff" (fisik mematikan folder unsafe via prefix `DISABLED `) dan PIN lock/boot guard **telah dibatalkan**. Arsitektur dirombak menjadi jauh lebih ringan: Safe Mode kini murni beroperasi sebagai **View Filter (UI Masking)** dan **Collection Tagging** (Koleksi dapat berisi tag/mods unsafe).

## 1. Executive Summary

- **Problem Statement**: Users manage mods with varying content sensitivity. Opening the app on stream or in public risks displaying NSFW thumbnails and names. A fast, trustworthy privacy layer is required to prevent accidental exposure in the UI.
- **Proposed Solution**: A lightweight View Filter system backed by the `is_safe` flag on individual mods. Rather than physically disabling mods on the disk, the frontend simply hides or blurs out mods marked as unsafe (`is_safe = false`) when Safe Mode is active. Collections themselves can contain unsafe mods, and applying them simply operates on the database/disk normally, while the UI masks the unsafe items.
- **Success Criteria**:
  - **View-Level Isolation**: When Safe Mode is ON, the Object List and Folder Grid either hide or blur unsafe items. The backend still returns the full dataset (as seen in `workspace_read_model` containing `contains_unsafe_mods`, `is_safe`, etc.), and the frontend applies the filter.
  - **Collection Integration**: Collections can safely snapshot and restore loadouts containing unsafe mods without triggering a physical corridor wipe.
  - **No Physical File Manipulation for Privacy**: Privacy toggles no longer cause massive disk I/O (renaming folders to `DISABLED `). Disk state remains untouched; only the UI presentation changes.
  - **Auto-Tagging**: New imports containing restricted keywords are automatically tagged `is_safe = false` during the scan engine phase (via `services::scanner::sync::helpers::classify_safety`).

---

## 2. User Experience & Functionality

### User Stories

#### US-30.1: Toggle Safe Mode UI Filter

As a user, I want a quick toggle to hide/blur my unsafe mods so I can stream or share my screen without risk.

| ID        | Type        | Criteria                                                                                                                                                                  |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.1.1 | ✅ Positive | Given the Safe Mode shield icon is clicked, the frontend immediately filters out or blurs mods where `is_safe == false` from the Object List and Folder Grid.             |
| AC-30.1.2 | ✅ Positive | The backend does NOT perform any physical disk renames. `workspace_view_model` continues to serve the true disk state, relying on the frontend to apply the privacy mask. |

#### US-30.2: Auto-Tagging of Imported Mods

As a user, I want mods with known NSFW keywords in their names to be automatically marked as unsafe during import.

| ID        | Type        | Criteria                                                                                                                                             |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.2.1 | ✅ Positive | Given an import containing a folder with a restricted keyword (e.g., "NSFW"), the scanner (`classify_safety`) automatically flags `is_safe = false`. |
| AC-30.2.2 | ✅ Positive | The `mods` table stores `is_safe = false` and this is reflected in the UI upon completion of the import batch.                                       |

#### US-30.3: Manual Safety Tagging

As a user, I want to manually override the safety tag of a mod or object.

| ID        | Type        | Criteria                                                                                                                                                        |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.3.1 | ✅ Positive | Given the user toggles the safety flag on a mod in the UI, the backend updates the `is_safe` column in the DB and writes the flag to `info.json` if applicable. |

---

## 3. Architecture & Technical Design

### Data Model

- **`mods` Table**: Contains `is_safe` (BOOLEAN).
- **`objects` Table**: Safety is derived dynamically or cached based on its children's safety status.
- **Backend View Model**: `workspace_read_model` maps nodes with `is_safe`, `contains_safe_mods`, and `contains_unsafe_mods`. The backend deliberately **leaves the preview selection independent of safety** (as verified in `workspace/tests/safety_filter.rs`), delegating the masking responsibility to the frontend.

### Component Interactions

1. **Frontend Grid/List**: Reads the `is_safe` property of each item. If Global Safe Mode is active, items with `is_safe == false` are either removed from the DOM or blurred using CSS `filter: blur()`.
2. **Scanner**: `services::scanner::sync::helpers::classify_safety` uses a dictionary of keywords to assign `SAFETY_SOURCE_AUTO_TAGGED`.
3. **Collections**: A Collection simply stores the array of `enabled_mod_ids`. When a Collection is applied, it physically enables the mods (removing `DISABLED ` prefix) regardless of their `is_safe` flag. The UI continues to mask them if Safe Mode is currently active.

### Removed Mechanisms (Historical)

- `switch_corridor` command and physical prefix renaming for Safe Mode.
- Boot guard, UI PIN locking, and Crash Resiliency `tasks` (PENDING state).
- Complex `active_collection_id` corridor handoffs.
