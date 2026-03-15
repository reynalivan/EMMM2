# Epic 08: Smart Filters & Sorting

## 1. Executive Summary

- **Problem Statement**: With hundreds or thousands of objects in the objectlist, users can't find the right character without scrolling — and there's no way to quickly focus on objects that have mods or hide NSFW content in public settings.
- **Proposed Solution**: A filter toolbar with fuzzy text search (debounced, Web Worker-offloaded), sort options (A-Z, New, Rarity), status filters (Enabled/Disabled), and a Safe Mode filter that runs server-side to isolate content corridors.
- **Success Criteria**:
  - Filter/search list updates within ≤ 100ms after each keystroke for a dataset of ≤ 10,000 objects.
  - Fuzzy match uses Jaro-Winkler with a score threshold ≥ 0.75; search is offloaded to a Web Worker for multi-threading.
  - Sort preference persists across restarts via `localStorage`.
  - 0 results returned for any object that contains no mods matching the active safety corridor.

---

## 2. User Experience & Functionality

### User Stories

#### US-08.1: ObjectList Sorting

As a user, I want to sort the objectlist by name, date added, or rarity, so that I can organize my mods based on game value or chronological addition.

| ID        | Type        | Criteria                                                                                                                                          |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-08.1.1 | ✅ Positive | Given "A-Z", the list sorts alphabetically by display name (Name column in DB)                                                                  |
| AC-08.1.2 | ✅ Positive | Given "New", the list sorts by `created_at` descending, showing most recently added objects first                                                 |
| AC-08.1.3 | ✅ Positive | Given "★" (Rarity), the list sorts by numeric rarity metadata (from MasterDB/Schema) descending                                                  |
| AC-08.1.4 | ✅ Positive | Given any sort selection, the preference is written to `localStorage['sidebarSort']` and restored on next launch                                  |
| AC-08.1.5 | ⚠️ Edge     | Ties in rarity or date sort are resolved by alphabetical name fallback to ensure a stable, deterministic list position                            |

---

#### US-08.2: Fuzzy Text Searching

As a user, I want to search for objects by name using a text bar with typo tolerance, so that I can jump to a character even with imperfect spelling.

| ID        | Type        | Criteria                                                                                                                                                                                              |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-08.2.1 | ✅ Positive | Given the search bar, when I type a partial or slightly misspelled name (e.g., "hutao" → matches "Hu Tao"), then the list filters to show matching objects with score ≥ 0.75 in ≤ 100ms per keystroke |
| AC-08.2.2 | ✅ Positive | Given ≥ 1,000 objects, when typing rapidly at ≥ 5 keystrokes/second, then the search offloads computation to a Web Worker — the main thread never blocks                                              |
| AC-08.2.3 | ❌ Negative | Given a search query containing only special characters (e.g., `!@#$`) or unicode emoji, the fuzzy engine returns 0 results — it does not crash or return undefined                                   |
| AC-08.2.4 | ⚠️ Edge     | Given the user clears the search input while a previous async Worker search is pending, then the stale result is discarded — the full unfiltered list restores within ≤ 50ms                          |

---

#### US-08.3: Status & Category Filtering

As a user, I want to filter the objectlist by status (enabled/disabled) or category, so that I can focus on specific subsets of my mods.

| ID        | Type        | Criteria                                                                                                                                                                             |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-08.3.1 | ✅ Positive | Given "Enabled Only", the list is filtered to objects having `enabled_count > 0`. "Disabled Only" shows objects with `enabled_count = 0` or missing files.                            |
| AC-08.3.2 | ✅ Positive | Given a Category selection (e.g., Characters), the list is filtered to objects where `object_type` matches. Non-matching objects are excluded.                                       |
| AC-08.3.3 | ✅ Positive | Given multiple metadata filters (Element: Pyro, Rarity: 5), the list uses an AND logic intersection — only objects matching ALL active chips are shown                                |
| AC-08.3.4 | ⚠️ Edge     | Given a newly sync'd object that changes category while a filter is active, it immediately moves or disappears from the list based on match criteria via cache invalidation          |

---

#### US-08.4: Safe Mode Content Filter

As a privacy-conscious user, I want NSFW-flagged objects to be hidden or masked in the objectlist when Safe Mode is active, so that sensitive content is never visible in public.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-08.4.1 | ✅ Positive | Given Safe Mode is ENABLED, then the `get_objects_cmd` backend query excludes any object where ALL its mods are NSFW (`is_safe = false`) — the object does not appear in the "Safe" corridor list at all |
| AC-08.4.2 | ✅ Positive | Given Safe Mode is DISABLED (Unsafe Corridor), then only objects with at least one NSFW mod are shown (or all if configured for a mixed view, but current logic enforces corridor isolation)  |
| AC-08.4.3 | ❌ Negative | Given an object contains only mods with explicit `is_safe = false` flags and Safe Mode is ENABLED, it is excluded even if its folder name contains no NSFW keywords |
| AC-08.4.4 | ⚠️ Edge     | Given Safe Mode is toggled via the global toggle mid-session while a sensitive object is currently selected in the objectlist, then within ≤ 100ms: the selected object is deselected, the list re-fetches with the new filter, and no sensitive data remains visible in the preview panel |

---

### Non-Goals

- No server-side or remote search; all fuzzy matching runs locally (Web Worker or main thread).
- No advanced query syntax (e.g., `is:enabled type:weapon`) in this phase — plain text + toggles only.
- No saved named filter presets.
- Safe Mode filter is applied at the `get_objects_cmd` query level — not as a post-process mask on the frontend (prevents hidden data traveling over IPC).

---

## 3. Technical Specifications

### Architecture Overview

```
FilterToolbar (ObjectListToolbar component)
  ├── SearchInput → debounced (150ms) → fuzzySearch(query, items) in Web Worker
  ├── SortChips → ['name', 'date', 'rarity'] → persisted in localStorage['sidebarSort']
  └── FilterPanel (Collapsible)
      ├── StatusChips: [All, Enabled, Disabled]
      ├── TypeChips: [All, Characters, Weapons, UI, Other]
      └── MetadataChips: [Dynamic based on schema]

useObjects(gameId, filterState) → React Query
  └── invoke('get_objects_cmd', { gameId, safeMode, hideEmpty, sortBy })
      └── SQL: WHERE ... ORDER BY ... (sort applied DB-side)

Fuzzy search (heavy path):
  └── Web Worker: fuzzysort.js OR custom Jaro-Winkler impl
      → postMessage([ {id, score} ]) → intersect with React Query cache
```

### Integration Points

| Component        | Detail                                                                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| Object Query     | `invoke('get_objects_cmd', { gameId, hide_empty, safe_mode, sort_by })` — sorting and empty-filter done DB-side via `ORDER BY` and `HAVING` |
| Fuzzy Search     | `fuzzysort` npm package (Jaro-Winkler variant) — threshold 0.75, runs in Web Worker for ≥ 500 objects                                       |
| Sort Persist     | `localStorage['sidebarSort']` — string enum `'az'                                                                                           | 'active_first'` |
| Safe Mode        | Read from `useAppStore.safeMode` Zustand state → passed as query param to `get_objects_cmd`                                                 |
| Query Invalidate | All filter changes call `queryClient.invalidateQueries(['objects', gameId])` to re-fetch with new params                                    |

### Security & Privacy

- **Safe Mode filter is enforced backend-side** in the SQL `WHERE` clause — objects that do not contain mods in the active corridor never travel over the IPC channel.
- **Fuzzy search operates on the already-fetched object name array** — no additional IPC call is made with the raw search string; the string never reaches the Rust process.
- **Sort key deserialization** uses a typed Rust `enum SortBy { Az, ActiveFirst }` — unrecognized values are rejected by `serde` before the query runs.

---

## 4. Dependencies

- **Blocked by**: Epic 07 (Object List — data source), Epic 30 (Privacy / Safe Mode — `safeMode` global flag).
- **Blocks**: Nothing directly — the filter toolbar is a pure query-parameter modifier for Epic 07's data.
