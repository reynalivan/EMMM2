# Epic 03: Onboarding & Welcome Screen

## 1. Executive Summary

- **Problem Statement**: First-time users have no context on how to point the app at their game installations — without a guided flow, they misconfigure paths or abandon setup entirely.
- **Proposed Solution**: A polished FTUE (First-Time User Experience) state machine at `/welcome` with two paths (auto-detect from XXMI root, manual form), animated transitions, and inline validation — completing without any backend crashes on bad input.
- **Success Criteria**:
  - A new user with a standard XXMI installation completes the happy path in ≤ 3 clicks (select folder → confirm → dashboard).
  - Auto-detect surfaces detected game list in ≤ 1s for a root containing ≤ 5 game subfolders.
  - 0 unhandled errors shown to the user when they select an invalid folder during setup.
  - The welcome screen is never shown again after the first successful game save (gated 100% by `ConfigStatus::HasConfig`).
  - Welcome screen renders first meaningful paint in ≤ 500ms on app start (Lighthouse LCP target).

---

## 2. User Experience & Functionality

### User Stories

#### US-03.1: Welcome Screen Display

As a first-time user, I want to see a welcoming introduction screen, so that I understand what the app does and how to proceed.

| ID        | Type        | Criteria                                                                                                                                                                                                |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-03.1.1 | ✅ Positive | Given `ConfigStatus::FreshInstall`, when the app loads, then the user is routed to `/welcome` and sees the animated logo, aurora background, and the two primary CTAs: "Auto-Detect" and "Manual Setup" |
| AC-03.1.2 | ✅ Positive | Given the welcome screen, then both CTA buttons are fully accessible (keyboard-navigable, aria-label set) and visible without scrolling at 1024×768 minimum resolution                                  |
| AC-03.1.3 | ❌ Negative | Given `ConfigStatus::HasConfig`, when the app loads, then the `/welcome` route is never shown — the router immediately navigates to `/dashboard`                                                        |
| AC-03.1.4 | ⚠️ Edge     | Given the user resizes the window to < 800px width during the intro animation, the layout scales correctly and CTA buttons are not clipped or hidden                                                    |

---

#### US-03.2: Auto-Detect Flow

As a first-time user, I want to select my XXMI launcher folder and have games detected automatically, so that setup is fast without entering paths manually.

| ID        | Type        | Criteria                                                                                                                                                                                               |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-03.2.1 | ✅ Positive | Given the user clicks "Auto-Detect" and selects a valid XXMI root via the OS folder dialog, then detected games are listed with name, type, and path within ≤ 1s                                       |
| AC-03.2.2 | ✅ Positive | Given detected games are listed, then the user can remove individual games from the list before confirming                                                                                             |
| AC-03.2.3 | ✅ Positive | Given ≥ 1 games remain in the list, when the user clicks "Confirm & Continue", then all games are saved via `add_game`, the first is set as `active_game_id`, and the router navigates to `/dashboard` |
| AC-03.2.4 | ❌ Negative | Given the selected folder contains no recognized game subfolders, when auto-detect finishes, then "No games found in this folder" message is displayed and the user can try a different path           |
| AC-03.2.5 | ❌ Negative | Given the user cancels the native folder dialog (presses Escape or closes it), then the welcome screen state is unchanged — no error is shown                                                          |
| AC-03.2.6 | ⚠️ Edge     | Given the scanner encounters a symlink loop or extreme nesting (> 5 levels), then the heuristic bails out after the depth limit and returns any valid games found so far — no UI freeze or crash       |

---

#### US-03.3: Manual Setup Flow

As a first-time user, I want to manually configure a single game by entering its type and path, so that I can proceed even if auto-detect fails or misses my game.

| ID        | Type        | Criteria                                                                                                                                                                                                                         |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-03.3.1 | ✅ Positive | Given the user navigates to manual setup, then a form with a `GameType` dropdown (Genshin/HSR/ZZZ/WuWa/Endfield) and a folder picker is shown                                                                                    |
| AC-03.3.2 | ✅ Positive | Given valid `game_type` and valid folder path, when the user submits, then the game is added, set as active, and the router navigates to `/dashboard`                                                                            |
| AC-03.3.3 | ❌ Negative | Given a selected folder where the expected `mods` subfolder or game executable is missing, when submit is attempted, then an inline validation error is shown and submission is blocked — the backend `add_game` is never called |
| AC-03.3.4 | ❌ Negative | Given the user clicks submit without selecting a `GameType` from the dropdown, then the form shows a "Game type is required" validation message at the field level                                                               |
| AC-03.3.5 | ⚠️ Edge     | Given the user submits the form rapidly twice before the first response returns, then only one `add_game` IPC call is in-flight (button disabled while pending), preventing duplicate DB records                                 |

---

#### US-03.4: Welcome Screen Navigation

As a first-time user, I want to navigate back from sub-steps to the initial welcome screen, so that I can switch from auto-detect to manual setup (or vice versa) without restarting.

| ID        | Type        | Criteria                                                                                                                                                                                      |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-03.4.1 | ✅ Positive | Given the user is on the auto-detect results screen, then a "Back" button is visible and clicking it returns to the initial landing state                                                     |
| AC-03.4.2 | ✅ Positive | Given the user is on the manual setup form, then a "Back" button returns to the initial landing state, clearing form inputs                                                                   |
| AC-03.4.3 | ⚠️ Edge     | Given the user presses the browser's back gesture (Mouse4 or Alt+Left), then the Tauri WebView intercepts it and keeps the user on the onboarding flow — they do not navigate to a blank page |

---

### Non-Goals

- No multi-game simultaneous setup in a single onboarding session (one at a time via manual; multiple via auto-detect).
- No Steam / Epic Games library parsing for game discovery.
- No cloud account creation or sign-in during onboarding.
- No tutorial video or interactive walkthrough overlay; setup is self-explanatory from the UI.
- Onboarding cannot be re-triggered after setup except via Settings → Factory Reset.

---

## 3. Technical Specifications

### Architecture Overview

```
/welcome route (React)
  └── WelcomeStateMachine: landing → scanning → results → manual
      ├── landing:  Two CTA buttons
      ├── scanning: invoke('auto_detect_games', rootPath) → GameConfig[]
      ├── results:  Editable list → invoke('add_game') * N → navigate('/dashboard')
      └── manual:   Form → invoke('add_game', gameType, path) → navigate('/dashboard')

Backend
  ├── check_config_status() → FreshInstall | HasConfig   (Epic 01)
  ├── auto_detect_games(root_path) → Vec<GameConfig>     (Epic 02)
  └── add_game(game_type, path) → GameRecord             (Epic 02)
```

### Integration Points

| Component      | Detail                                                                      |
| -------------- | --------------------------------------------------------------------------- |
| Config Status  | `commands/app/settings_cmds.rs` → `check_config_status` (Epic 01)           |
| Game Detection | `commands/games/auto_detect_games` (Epic 02)                                |
| Game Save      | `commands/games/add_game` (Epic 02)                                         |
| Folder Dialog  | `tauri-plugin-dialog` → `open({ directory: true })`                         |
| Routing        | React Router v6 — programmatic `navigate('/dashboard')` on success          |
| Animation      | Framer Motion — entrance transitions ≤ 400ms, no layout-blocking animations |

### Security & Privacy

- **All folder paths entered during onboarding** are passed through `std::fs::canonicalize()` before `add_game` processes them — prevents path traversal on the very first interaction.
- **`GameType` is always deserialized as a typed enum** (`serde` `#[serde(rename_all = "snake_case")]`) — unrecognized strings are rejected at the serde boundary, never reach DB.
- **Submit button is disabled while an IPC call is in-flight** — enforced in React state, not just UX polish — to prevent double-insert race conditions on rapid clicking.
- **No user data is sent externally** during onboarding; all detection and validation is local.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap), Epic 02 (Game Management — CRUD services).
- **Blocks**: Nothing directly — onboarding completes into the main app covered by all subsequent epics.
