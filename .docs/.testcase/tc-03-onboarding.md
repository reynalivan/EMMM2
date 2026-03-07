# Test Cases: Onboarding & Welcome Screen (req-03)

## A. Requirement Summary

- **Feature Goal**: Provide a First-Time User Experience (FTUE) that guides users through essential setup: language selection, theme pref, and auto-detecting or manually adding their first game.
- **User Stories**: Display Welcome Screen, Auto-Detect Flow, Manual Setup Flow, Navigation to Dashboard, Re-entry Guard.
- **Acceptance/Success**: Language switches instantly, Auto-detect finds supported games <2s. Proceed to dashboard only when ≥1 game exists. Welcome screen is inaccessible once configured.
- **Main Risks**: User creates 0 games but bypasses into dashboard, Database write locked during setup, language preference not persisting.
- **Gaps / Ambiguities**: Should the 'Skip & Add Manually Later' option be available to bypass the FTUE and enter an empty dashboard? Current REQ says ≥1 game required.

## B. Coverage Matrix

- AC-03.1.1, AC-03.1.2 → TC-03-01 (FTUE Initial Display)
- AC-03.1.3, AC-03.1.4 → TC-03-02 (FTUE Preferences)
- AC-03.2.1, AC-03.2.2 → TC-03-03 (FTUE Auto-Detect Success)
- AC-03.2.3 → TC-03-04 (FTUE Auto-Detect Failed)
- AC-03.3.1, AC-03.3.2 → TC-03-05 (FTUE Manual Add Success)
- AC-03.3.3 → TC-03-06 (FTUE Manual Add Invalid)
- AC-03.4.1 → TC-03-07 (FTUE Proceed Blocking)
- AC-03.4.2 → TC-03-08 (FTUE Completion Routing)
- AC-03.5.1 → TC-03-09 (FTUE Re-entry Guard)

## C. Test Cases

| TC ID | Scenario | Type | Priority | Preconditions | Test Data | Steps | Expected Result | Coverage |
| -------- | -------------------------------------- | -------- | -------- | --------------------- | --------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------- | -------------------- |
| TC-03-01 | Fresh Boot pushes to Welcome | Positive | High | Cleared DB, no games | N/A | 1. Launch EMMM2 | Router defaults to`/welcome`. Dashboard is hidden. | AC-03.1.1, AC-03.1.2 |
| TC-03-02 | Language & Theme Preference | Positive | Med | On`/welcome` |`ja-JP`,`dark` | 1. Select 'Japanese'<br>2. Select 'Dark' | UI translates instantly via i18next. Theme updates. Setting saved to schema instantly. | AC-03.1.3, AC-03.1.4 |
| TC-03-03 | Auto-Detect FTUE populates list | Positive | High | Games found on C: | N/A | 1. Click 'Scan Entire PC' | Progress bar shows. Games appear in list. Proceed button unlocks. | AC-03.2.1, AC-03.2.2 |
| TC-03-04 | Auto-Detect finds nothing | Negative | Med | No games on system | N/A | 1. Click 'Scan Entire PC' | Progress finishes. UI says "No games found". Proceed button stays disabled. | AC-03.2.3 |
| TC-03-05 | Manual Add overrides lock | Positive | High | Valid path copied | Valid Game | 1. Click 'Add Manually'<br>2. Paste path<br>3. Submit | Game appears in list. Proceed button unlocks. | AC-03.3.1, AC-03.3.2 |
| TC-03-06 | Manual Add invalid shows inline error | Negative | High | Invalid directory | N/A | 1. Click 'Add Manually'<br>2. Select Desktop<br>3. Submit | Input turns red stating`Executable not found`. Proceed stays locked. | AC-03.3.3 |
| TC-03-07 | Proceed requires >= 1 game | Negative | High | db count == 0 | N/A | 1. Inspect 'Get Started' button | Button is strictly disabled and unclickable. | AC-03.4.1 |
| TC-03-08 | Finishing Setup transitions app | Positive | High | 1 game in list | N/A | 1. Click 'Get Started' | Router animates`/welcome` out, mounts`/dashboard`. DB`onboarding_complete` set. | AC-03.4.2 |
| TC-03-09 | Welcome Screen inaccessible post-setup | Positive | High | db count >= 1 | N/A | 1. Open app normally | App skips`/welcome` entirely, routing straight to`/dashboard`. | AC-03.5.1 |
| TC-03-10 | Force URL Navigation block | Edge | Med | db count >= 1 | URL`/welcome` | 1. Force inject router memory to Push(`/welcome`) | Router explicitly redirects back to`/dashboard` instantly. | AC-03.5.1 |
| TC-03-11 | Auto-Detect cancel aborts IO | Edge | Med | In-progress deep scan | N/A | 1. Click 'Scan'<br>2. Immediately click 'Cancel' | Rust thread kills watcher. UI resets. | AC-03.2.2 |

## D. Missing / Implied Test Areas

- **Theme OS Sync**: If the user's OS is dark mode, does the Welcome screen default to Dark Mode?
- **Locale OS Sync**: Does i18n auto-initialize to the OS system language before the user even clicks the dropdown?

## E. Open Questions / Gaps

- _None_

## F. Automation Candidates

- **TC-03-08 (Finish Setup)**: Critical E2E Test via WebdriverIO confirming the DOM updates from`[data-testid="welcome-screen"]` to`[data-testid="dashboard-layout"]`.
- **TC-03-07 (Proceed Blocking)**: Vitest UI testing confirming the button`<button disabled>` attribute is inherently reactive to the local React state tracking the game list count.
- **TC-03-09 (Re-entry Guard)**: Vitest checking the Route Guard component explicitly verifies`DB_GAME_COUNT > 0` and denies access.
