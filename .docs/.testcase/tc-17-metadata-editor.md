# Test Cases: Metadata Editor (Epic 17)

## A. Requirement Summary

- **Feature Goal**: Allow users to view and edit mod metadata (author, version, description, tags, link) directly in the app, with auto-save and link validation.
- **User Roles**: End User
- **User Story**:
 - US-17.1: View Mod Info
 - US-17.2: Auto-Saving Edits
 - US-17.3: Link Validation & Opening
- **Success Criteria**:
 -`info.json` populates form ≤ 100ms.
 - Auto-save on blur persists in ≤ 200ms.
 - Missing`info.json` handled.
 - Corrupted`info.json` shows an error banner but does not crash.
 - URL "Open Link" disabled for invalid URLs.
- **Main Risks**: Desynchronized concurrent edits (saving mod A's data to mod B) or filesystem locks causing silent write failures.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-17-metadata-editor.md`

- AC-17.1.1 → TC-17-01
- AC-17.1.2 → TC-17-02
- AC-17.1.3 → TC-17-03
- AC-17.2.1 → TC-17-04
- AC-17.2.2 → TC-17-05
- AC-17.2.3 → TC-17-06
- AC-17.2.4 → TC-17-07
- AC-17.3.1 → TC-17-08
- AC-17.3.2, AC-17.3.3 → TC-17-09

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | ------------------------- | -------- | -------- | ----------------------------- | --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- |
| TC-17-01 | Load Valid Metadata | Positive | High |`info.json` with all fields | 1. Select the mod folder containing a valid`info.json`. | Editor fields (name, author, version, description, tags, link) populate in ≤ 100ms. | S2 | AC-17.1.1 |
| TC-17-02 | Missing Metadata File | Positive | High |`N/A` | 1. Select a mod folder with no`info.json`.<br>2. Edit a field and blur it. | Fields show placeholders. File is created only after editing and blurring. | S3 | AC-17.1.2 |
| TC-17-03 | Corrupted Metadata File | Negative | High |`Invalid JSON string` | 1. Select the mod folder with invalid JSON in`info.json`. | Shows inline error "Metadata file is corrupted...". App does not crash. | S2 | AC-17.1.3 |
| TC-17-04 | Auto-Save on Blur | Positive | High |`NewAuthor` string | 1. Change the author field.<br>2. Click outside the field (`onBlur`). | "Saved ✓" indicator pulses for 1.5s. Changes persist to`info.json` in ≤ 200ms. | S2 | AC-17.2.1 |
| TC-17-05 | Auto-Save No-Op | Positive | Med |`N/A` | 1. Select field and blur without changing.<br>2. Switch to another mod. | No redundant save call is made when switching if no new edits exist. | S4 | AC-17.2.2 |
| TC-17-06 | Save Disk Error | Negative | High |`Read-only info.json` | 1. Edit a field in a read-only metadata file and blur. | Field reverts to previous value. Error toast "Save failed: [reason]" appears. | S2 | AC-17.2.3 |
| TC-17-07 | Rapid Mod Switching | Edge | High |`N/A` | 1. Edit Mod A.<br>2. Instantly click Mod B before save finishes. | Save completes for Mod A. Mod B's fields are not overwritten by Mod A's data safely avoiding race conditions. | S1 | AC-17.2.4 |
| TC-17-08 | Valid URL Action | Positive | Med |`https://example.com` | 1. Select Mod with valid link.<br>2. Click "Open Link" icon. | OS default browser opens the URL. | S3 | AC-17.3.1 |
| TC-17-09 | Invalid/Unsafe URL Action | Negative | High |`see readme`,`file://c/path` | 1. Select Mod with invalid link.<br>2. Check "Open Link" icon. | Icon is disabled. No URL is opened maliciously or otherwise. | S2 | AC-17.3.2, AC-17.3.3 |

## D. Missing / Implied Test Areas

- **Max Length Constraints**: Are there max lengths for descriptions or tags? (Implied: No hard DB limit, but UI should gracefully wrap).
- **Concurrent External Edits**: What happens if the user manually edits`info.json` externally while the app is running? (Implied file watcher updates the cache and UI).

## E. Open Questions / Gaps

- "Tags are stored as a`Vec<String>`". Is there a limit per tag, or total tags? Can a user enter a 1000-character tag?

## F. Automation Candidates

- **TC-17-01 / TC-17-02**: Core behavior of reading metadata configs (Tauri Unit integration tests).
- **TC-17-04**: React Query cache and debounce functionality (Vitest).
- **TC-17-07**: Crucial guard against data corruption on switch (Vitest / E2E).

## G. Test Environment Setup

- **Preconditions**: OS: Windows 10/11. App: EMMM2 dev build. Object Panel and File Watcher fully active.
- **Context Injection**:
 - Valid and invalid serialized`<ModTarget>/info.json` structures simulating missing, corrupted, and correct data schemas.

## H. Cross-Epic E2E Scenarios

- **E2E-17-01 (External File Watcher Metadata Sync)**: User opens EMMM2, selects an active Mod loading values into the Preview Panel Editor (Epic 17). User edits`info.json` from a standard text editor externally (Epic 28) and saves. The Watcher catches the change event syncing into Rust memory mapping actively firing Tauri event payload refreshing the UI React Query cache updating the Preview Editor avoiding race conditions.
