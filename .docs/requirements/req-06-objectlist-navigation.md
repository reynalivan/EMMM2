# Epic 06: ObjectList Navigation & Resizing

## 1. Executive Summary

- **Problem Statement**: The objectlist is the primary navigation hub for the entire 3-panel workspace — without min/max constraints and persistent sizing, users resize it past usable bounds or lose their layout on restart.
- **Proposed Solution**: A resizable objectlist panel (180px–600px) powered by `react-resizable-panels`, with category-grouped object list sections (collapsible), persistent width via `localStorage`, and a responsive overlay drawer for viewports < 768px.
- **Success Criteria**:
  - ObjectList drag resize renders at ≥ 60fps (≤ 16ms/frame) measured via Chrome DevTools Performance tab.
  - ObjectList width persisted within ≤ 200ms of drag-end and restored on next launch in ≤ 50ms.
  - Category collapse/expand with 500 items re-renders in ≤ 100ms (no main-thread freeze).
  - ObjectList width never falls below 180px or exceeds 600px regardless of window size.
  - On viewport < 768px, objectlist renders as an overlay drawer in ≤ 100ms of viewport detection.

---

## 2. User Experience & Functionality

### User Stories

#### US-06.1: Resizable ObjectList Layout

As a user, I want to adjust the width of the objectlist, so that I can allocate screen space to the grid or preview panel as needed.

| ID        | Type        | Criteria                                                                                                                                                                           |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-06.1.1 | ✅ Positive | Given the main workspace, when the user drags the left divider, then the objectlist width updates in real-time between 180px (min) and 600px (max) at ≥ 60fps                         |
| AC-06.1.2 | ✅ Positive | Given the user has resized the objectlist, when the app restarts, then the previous width (as a fractional percentage) is restored from `localStorage` in ≤ 50ms — before first paint |
| AC-06.1.3 | ❌ Negative | Given the user drags the divider past the minimum (180px), then the drag stops at the boundary — objectlist content is never clipped or zero-width                                    |
| AC-06.1.4 | ⚠️ Edge     | Given the OS window is resized to < 1024px total width, then all three panels redistribute fractionally while each stays above its minimum — no panel fully disappears             |

---

#### US-06.2: Category Sections

As a user, I want the objects in the objectlist grouped by logical categories (Characters, Weapons, UI), so that I can scan a large object list without scrolling through an unsorted flat list.

| ID        | Type        | Criteria                                                                                                                                                                                      |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-06.2.1 | ✅ Positive | Given a parsed `GameSchema` with defined categories, when the objectlist renders, then objects are grouped under collapsible category headers matching the schema                                |
| AC-06.2.2 | ✅ Positive | Given a category header, when clicked, then it toggles expanded/collapsed with a CSS animation ≤ 200ms — and the toggle state persists in `localStorage`                                      |
| AC-06.2.3 | ❌ Negative | Given an object with no schema category match, then it is placed in an "Uncategorized" section — the render cycle does not throw or produce a blank objectlist                                   |
| AC-06.2.4 | ⚠️ Edge     | Given a category with ≥ 500 items being expanded, the list renders using virtualization (react-virtual or equivalent) — the expand action completes in ≤ 100ms without blocking the UI thread |

---

#### US-06.3: Mobile / Responsive Adaptation

As a small-viewport user, I want the objectlist to become a slide-out drawer, so that the folder grid uses the full screen width.

| ID        | Type        | Criteria                                                                                                                                                                               |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-06.3.1 | ✅ Positive | Given viewport width < 768px, when the workspace renders, then the objectlist transforms into an absolutely-positioned overlay drawer (CSS `position: fixed`) — not a shrunk static panel |
| AC-06.3.2 | ✅ Positive | Given the mobile drawer is open, when the user selects an object, then the drawer closes automatically within ≤ 200ms, revealing the full-width folder grid                            |
| AC-06.3.3 | ❌ Negative | Given viewport < 768px and the drawer is closed, then the underlying grid/preview content is fully interactable — no invisible overlay blocking clicks                                 |
| AC-06.3.4 | ⚠️ Edge     | Given the user rotates the device from portrait (< 768px) to landscape (≥ 768px) mid-session, then the objectlist transitions from drawer to panel layout without requiring a page reload |

---

### Non-Goals

- No sub-category nesting (e.g., "Characters → 5 Star") in this phase — categories are single-level.
- No persistent per-category collapse state synced to DB; `localStorage` only.
- No virtual scrolling for the objectlist panel itself (only for individual category item lists with ≥ 100 items).
- Category order is dictated by the `GameSchema` definition — no user-draggable reordering in this phase.

---

## 3. Technical Specifications

### Architecture Overview

```
ObjectList (react-resizable-panels Panel)
  ├── width: [180px, 600px] — persisted as fraction in localStorage['panelLayout']
  └── ObjectList (Epic 07)
      ├── CategorySection (collapsible, per schema category)
      │   ├── CategoryHeader (count badge, expand toggle)
      │   └── VirtualizedObjectList (react-virtual, ≥ 100 items threshold)
      └── CategorySection: "Uncategorized" (fallback)

Mobile (viewport < 768px):
  └── DrawerOverlay (position: fixed, z-index: 50)
      └── same ObjectList tree
```

### Integration Points

| Component      | Detail                                                                                                          |
| -------------- | --------------------------------------------------------------------------------------------------------------- |
| Panel Width    | `react-resizable-panels` `Panel` with `minSize`, `maxSize` props; `onLayout` → debounced `localStorage.setItem` |
| Game Schema    | `useGameSchema()` hook — reads from React Query cache seeded by Epic 09                                         |
| Object List    | `useObjects()` hook — reads from Epic 07's React Query cache                                                    |
| Collapse State | `localStorage['sidebarCategoryStates']` — `Map<categoryId, boolean>`                                            |
| Virtualization | `@tanstack/react-virtual` — activated for category lists with ≥ 100 items                                       |

### Security & Privacy

- **Pure frontend component** — no backend calls are made directly from the objectlist; it reads from React Query caches populated by other epics.
- **`localStorage` stores only panel fractions (numbers) and collapse booleans** — no user paths, mod names, or game data are stored in localStorage.
- **Category grouping is derived from the schema enum** — no user-supplied strings are used as category identifiers; no XSS risk from category label rendering.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap — state stores), Epic 05 (Workspace Layout — `react-resizable-panels` shell), Epic 07 (Object List), Epic 09 (Game Schema — category definitions).
- **Blocks**: Nothing directly — objectlist is a consumer of data from other epics.
