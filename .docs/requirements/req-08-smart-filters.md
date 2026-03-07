# Epic 08: Smart Filters & Sorting

## 1. Executive Summary

- **Problem Statement**: With hundreds or thousands of objects in the objectlist, users can't find the right character without scrolling — and there's no way to quickly focus on objects that have mods or hide NSFW content in public settings.
- **Proposed Solution**: A filter toolbar with fuzzy text search (debounced, Web Worker-offloaded), sort options (A-Z, Active First), toggle filters (Hide Empty, Show Uncategorized), and a Safe Mode filter that runs server-side on the `get_objects_cmd` query — not client-side on cached data.
- **Success Criteria**:
  - Filter/search list updates within ≤ 100ms after each keystroke for a dataset of ≤ 10,000 objects.
  - Fuzzy match uses Jaro-Winkler with a score threshold ≥ 0.75 on a 0–1 scale; partial matches (e.g., "hutao" → "Hu Tao") score ≥ 0.80.
  - Safe Mode toggle clears all sensitive content from the list in ≤ 100ms — no visible NSFW item remains even momentarily.
  - Sort preference persists across restarts — read from `localStorage` in ≤ 50ms on mount.
  - 0 results returned for any query when Safe Mode is active for objects flagged `is_safe = false` — enforced backend-side.

---

## 2. User Experience & Functionality

### User Stories

#### US-08.1: ObjectList Sorting

As a user, I want to sort the objectlist objects by name or active status, so that I can find objects quickly based on my current workflow.

| ID        | Type        | Criteria                                                                                                                                          |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-08.1.1 | ✅ Positive | Given the sort control set to "A-Z", then the object list is alphabetically sorted ascending, case-insensitive, by object name                    |
| AC-08.1.2 | ✅ Positive | Given "Active First", then objects with `enabled_count > 0` appear before objects with `enabled_count = 0` within each category section           |
| AC-08.1.3 | ✅ Positive | Given any sort selection, the preference is written to `localStorage['sidebarSort']` and restored on next launch                                  |
| AC-08.1.4 | ❌ Negative | Given an invalid or unrecognized sort key in `localStorage` (e.g., tampered value), the UI defaults to "A-Z" — no error thrown                    |
| AC-08.1.5 | ⚠️ Edge     | Given a tie in "Active First" (multiple objects with the same `enabled_count`), the secondary sort falls back to A-Z — no non-deterministic order |

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

#### US-08.3: Empty / Uncategorized Toggles

As a user, I want to toggle visibility of empty objects and uncategorized folders, so that my objectlist shows only what is relevant.

| ID        | Type        | Criteria                                                                                                                                                                             |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-08.3.1 | ✅ Positive | Given "Hide Empty Objects" is enabled, then any object with `total_count = 0` is excluded from the virtual list within ≤ 100ms of the toggle                                         |
| AC-08.3.2 | ✅ Positive | Given "Show Uncategorized" is enabled, then folders in the game `mods_path` root that don't match any schema object appear in an "Uncategorized" section below all categories        |
| AC-08.3.3 | ❌ Negative | Given "Hide Empty" is toggled while the currently selected object has 0 mods, then `selectedObjectId` is set to `null` immediately — no orphaned selection persists                  |
| AC-08.3.4 | ⚠️ Edge     | Given a newly imported mod populates a previously empty object while "Hide Empty" is on, then after cache invalidation that object reappears in the list — no stale hide-state stuck |

---

#### US-08.4: Safe Mode Content Filter

As a privacy-conscious user, I want NSFW-flagged objects to be hidden or masked in the objectlist when Safe Mode is active, so that sensitive content is never visible in public.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-08.4.1 | ✅ Positive | Given Safe Mode is ENABLED, then the `get_objects_cmd` backend query excludes any object where ALL its mods have `is_safe = false` — the object does not appear in the list at all                                                                                                      |
| AC-08.4.2 | ✅ Positive | Given Safe Mode is DISABLED, then all objects render regardless of `is_safe` flag values                                                                                                                                                                                                |
| AC-08.4.3 | ❌ Negative | Given an object has the explicit `is_safe = false` flag on a mod and Safe Mode is ENABLED, it is excluded even if its folder name contains no NSFW keywords — the flag is authoritative                                                                                                 |
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
FilterToolbar (objectlist above ObjectList)
  ├── SearchInput → debounced (150ms) → fuzzySearch(query, objects) in Web Worker
  ├── SortSelect → ['az', 'active_first'] → persisted in localStorage['sidebarSort']
  ├── Toggle: Hide Empty → filterState.hideEmpty
  └── Toggle: Show Uncategorized → filterState.showUncategorized

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

- **Safe Mode filter is enforced backend-side** in the SQL `WHERE` clause — NSFW object records never travel over the IPC channel when Safe Mode is on; they are not masked client-side.
- **Fuzzy search operates on the already-fetched object name array** — no additional IPC call is made with the raw search string; the string never reaches the Rust process.
- **Sort key deserialization** uses a typed Rust `enum SortBy { Az, ActiveFirst }` — unrecognized values are rejected by `serde` before the query runs.

---

## 4. Dependencies

- **Blocked by**: Epic 07 (Object List — data source), Epic 30 (Privacy / Safe Mode — `safeMode` global flag).
- **Blocks**: Nothing directly — the filter toolbar is a pure query-parameter modifier for Epic 07's data.
