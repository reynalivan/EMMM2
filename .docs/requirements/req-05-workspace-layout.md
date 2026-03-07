# Epic 05: Workspace Layout & Navigation

## 1. Executive Summary

- **Problem Statement**: A mod manager with three interconnected panels (objectlist, grid, preview) needs a stable, persistent layout shell or users constantly re-adjust panel sizes and lose their place when switching views.
- **Proposed Solution**: A `ResizableWorkspace` shell with fraction-based panel persistence, a `TopBar` for cross-view navigation and game switching, and a `LaunchBar` that exposes play/conflicts/randomizer as primary actions — all fully independent of backend state.
- **Success Criteria**:
  - Panel drag resize updates visually within ≤ 16ms (one frame at 60fps), measured via browser DevTools Performance tab.
  - Panel widths are persisted to `localStorage` within ≤ 200ms of the drag end event and restored on next app launch within ≤ 50ms.
  - Top bar navigation click-to-route-change completes in ≤ 100ms (no full page reload).
  - Active game switch via top bar propagates to all child views (objectlist, grid) in ≤ 200ms.
  - Minimum panel widths are enforced: ObjectList ≥ 180px, Preview ≥ 240px, Explorer ≥ 300px — never collapses to zero.

---

## 2. User Experience & Functionality

### User Stories

#### US-05.1: 3-Panel Resizable Workspace

As a user, I want to adjust the size of the objectlist, grid, and preview panel, so that I can customize my workspace for browsing vs. reading INI files.

| ID        | Type        | Criteria                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-05.1.1 | ✅ Positive | Given the resizable workspace, when the user drags a divider, then panel proportions update fluidly at ≥ 60fps with no visual tearing or ghost elements                     |
| AC-05.1.2 | ✅ Positive | Given the user changes panel widths, when the app reloads, then the stored fractions are restored within ≤ 50ms so the user sees their saved layout immediately             |
| AC-05.1.3 | ❌ Negative | Given a panel reaches its defined minimum width (ObjectList: 180px, Preview: 240px, Explorer: 300px), then the divider stops moving — content is never hidden or zero-width    |
| AC-05.1.4 | ⚠️ Edge     | Given the user resizes the OS window to < 1024px width, then panels redistribute fractionally but all three remain visible above their minimums — no single panel is hidden |

---

#### US-05.2: Top Bar Navigation

As a user, I want a persistent top navigation bar, so that I can switch between Dashboard, Mod Manager, Collections, and Settings without losing my place.

| ID        | Type        | Criteria                                                                                                                                                                                      |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-05.2.1 | ✅ Positive | Given the top bar, when I click the EMMM2 logo/menu, a navigation popover renders all destinations (Dashboard, Mods Manager, Collections, Settings) with the current active route highlighted |
| AC-05.2.2 | ✅ Positive | Given the Game Selector dropdown, when I select a different game, then `activeGameId` in Zustand updates, the watcher restarts, and all dependent queries invalidate — all within ≤ 200ms     |
| AC-05.2.3 | ❌ Negative | Given the user clicks the route they are currently viewing, then the router performs a no-op — no DOM unmount/remount cycle occurs                                                            |
| AC-05.2.4 | ⚠️ Edge     | Given the user types keyboard shortcut Alt+1 / Alt+2 / Alt+3 while any modal is open, then the navigation shortcut is suppressed — modals take focus priority                                 |

---

#### US-05.3: Actionable Launch Bar

As a user, I want a dedicated launch bar area to start my game and check for conflicts, so that I can go from mod management to gameplay in one click.

| ID        | Type        | Criteria                                                                                                                                                                                          |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-05.3.1 | ✅ Positive | Given the Launch Bar, when I click "Play", then the game launch sequence from Epic 02 is invoked and the button shows a loading state for ≤ 200ms until the process spawns                        |
| AC-05.3.2 | ✅ Positive | Given hash conflicts are detected by the scan engine, when the Launch Bar renders, then a pulsing amber "Conflicts" badge is visible with a count, and clicking it navigates to the conflict view |
| AC-05.3.3 | ✅ Positive | Given `auto_close = true` in Settings, when Play is clicked and the game spawns successfully, then the EMMM2 process gracefully terminates within 2s                                              |
| AC-05.3.4 | ❌ Negative | Given no active game is configured, when the Launch Bar renders, then the "Play" button is disabled and shows "No game configured" tooltip                                                        |
| AC-05.3.5 | ⚠️ Edge     | Given background jobs (bulk import, dedup scan) are running when Play is clicked, then a warning dialog asks "Background tasks will be cancelled — continue?" before proceeding                   |

---

### Non-Goals

- No dual-monitor or detachable secondary window support for the preview panel in this phase.
- No fully collapsible objectlist (zero-width); objectlist minimum is 180px.
- No per-view layout presets or named workspace profiles.
- Panel persistence uses `localStorage` — not synced to DB or across machines.
- No animations on route change beyond CSS transitions ≤ 300ms.

---

## 3. Technical Specifications

### Architecture Overview

```
MainLayout (React)
  ├── ExternalChangeHandler (file watcher event listener)
  ├── GlobalModals (portal-mounted)
  ├── ToastContainer
  └── TopBar
      ├── NavPopover → React Router navigate()
      ├── GameSelector → invoke('set_active_game') + invalidateQueries
      └── [children route outlet]

ResizableWorkspace (react-resizable-panels)
  ├── Panel: ObjectList → ObjectList (Epic 06/07)
  ├── PanelResizeHandle (divider)
  ├── Panel: Explorer → FolderGrid (Epic 12)
  ├── PanelResizeHandle
  └── Panel: Preview → PreviewPanel (Epic 16)

LaunchBar
  ├── PlayButton → invoke('launch_game', activeGameId)
  ├── ConflictsBadge → navigate('/conflicts')
  └── RandomizerButton → (Epic 35)
```

### Integration Points

| Component         | Detail                                                                                                                |
| ----------------- | --------------------------------------------------------------------------------------------------------------------- |
| Panel Persistence | `react-resizable-panels` `onLayout` callback → debounced `localStorage.setItem('panelLayout', JSON.stringify(sizes))` |
| Panel Restore     | `defaultLayout` prop reads `localStorage.getItem('panelLayout')` on mount                                             |
| Game Switch       | `invoke('set_active_game', { gameId })` (Epic 02) + `queryClient.invalidateQueries()`                                 |
| Launch            | `invoke('launch_game', { gameId })` (Epic 02)                                                                         |
| Conflicts         | Conflict count read from React Query cache seeded by Epic 29 scan results                                             |
| Routing           | React Router v6 — `useNavigate()` — no full page reloads                                                              |

### Security & Privacy

- **No backend calls** for layout or panel sizing; persistence is `localStorage`-only — no user data at risk.
- **Active game switching** validates the new `gameId` against the `games` DB table on the backend (`set_active_game` rejects unknown IDs).
- **Launch Bar Play button** does not execute arbitrary paths — it delegates to `launch_game` which uses the pre-validated path stored in the `games` DB record.
- **Navigation popover** uses a static hardcoded route list — no user-supplied route strings are ever passed to `navigate()`.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap), Epic 02 (Game Management — game switcher, launch), Epic 04 (Settings — `auto_close` flag).
- **Blocks**: Epic 06 (ObjectList Navigation), Epic 07 (Object List), Epic 12 (Folder Grid UI), Epic 16 (Preview Panel) — all are mounted inside the `ResizableWorkspace`.
