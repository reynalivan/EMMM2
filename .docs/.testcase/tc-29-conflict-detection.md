# Test Cases: Conflict Detection & Resolution (Epic 29)

## A. Requirement Summary

- **Feature Goal**: Protect users from visual glitches by checking for identically overlapping mod hashes (texture overrides) across various mods at once. Ensure only one active mod exists per 'Object' category via 'Enable Only This' atomic toggle button.
- **User Roles**: Application User.
- **User Story**: As a user, I want a warning when my enabled mods conflict so I don't run into visual game glitches. I want to swap skins quickly.
- **Acceptance Criteria**:
 -`DuplicateInfo` returns during duplicate toggling per Object (< 50ms).
 -`ConflictResolveDialog` prompts users to 'Enable Only This'.
 - 'Enable Only This' atomically drops currently enabled mods for target in ≤ 500ms using`OperationLock`.
 - Global shader hash scan compiles`hash -> [{mod, section, line}]` entries, discovering collisions over 100 INIs in < 10s.
 - Zero false positives. Skipped bad INIs log.
- **Success Criteria**: Duplicate object UI handles atomic switches. Deep hash scanner logs out specific line numbers for hash collisions.
- **Main Risks**: Heavy deep hash scans might tie up tokio thread.
## B. Coverage Matrix

| Acceptance Criteria | Covered by TC IDs | Requirement File |
| :-------------------------------------- | :---------------- | :---------------------------------------------------------------- |
| AC-29.1.1 (Duplicate check hit) | TC-29-001 |`e:\Dev\EMMM2NEW\.docs\requirements\req-29-conflict-detection.md` |
| AC-29.1.2 (Conflict Resolve Modal) | TC-29-002 |`e:\Dev\EMMM2NEW\.docs\requirements\req-29-conflict-detection.md` |
| AC-29.1.3 ("Enable Anyway" bypass) | TC-29-003 |`e:\Dev\EMMM2NEW\.docs\requirements\req-29-conflict-detection.md` |
| AC-29.1.4 (Atomic disable multi) | TC-29-004 |`e:\Dev\EMMM2NEW\.docs\requirements\req-29-conflict-detection.md` |
| AC-29.2.1 (Context -> Enable Only This) | TC-29-005 |`e:\Dev\EMMM2NEW\.docs\requirements\req-29-conflict-detection.md` |
| AC-29.2.2 (UI Cache updates) | TC-29-006 |`e:\Dev\EMMM2NEW\.docs\requirements\req-29-conflict-detection.md` |
| AC-29.2.3 (No-op trigger) | TC-29-007 |`e:\Dev\EMMM2NEW\.docs\requirements\req-29-conflict-detection.md` |
| AC-29.3.1 (Global Hash Scan) | TC-29-008 |`e:\Dev\EMMM2NEW\.docs\requirements\req-29-conflict-detection.md` |
| AC-29.3.2 (Hash Collision View) | TC-29-009 |`e:\Dev\EMMM2NEW\.docs\requirements\req-29-conflict-detection.md` |
| AC-29.3.3 (Clean Hash run) | TC-29-010 |`e:\Dev\EMMM2NEW\.docs\requirements\req-29-conflict-detection.md` |
| AC-29.3.4 (Malformed INI skip) | TC-29-011 |`e:\Dev\EMMM2NEW\.docs\requirements\req-29-conflict-detection.md` |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| :-------- | :--------------------------- | :------- | :------- | :---------------------------------------------- | :----------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------- | :--------------- | :-------- |
| TC-29-001 | Duplicate mod enabled detect | Positive | High |`ModB` under Object "Albedo" | 1. "Albedo" has`ModA` already enabled.<br>2. Click Enable on`ModB`. | Warning returned with`DuplicateInfo` tracking`ModA`. | S1 | AC-29.1.1 |
| TC-29-002 | Conflict Resolve Dialog | Positive | High | The UI payload | 1. User hits a Duplicate Warning.<br>2. System issues duplicate warning. | Prompts`ConflictResolveDialog` with options: Enable Only This / Enable Anyway. | S1 | AC-29.1.2 |
| TC-29-003 | "Enable Anyway" Action | Negative | Medium | Duplicate warning present | 1. Conflict Resolve Dialog is open.<br>2. User clicks "Enable Anyway". | Both`ModA` and`ModB` become/stay enabled. No automatic renaming. | S2 | AC-29.1.3 |
| TC-29-004 | Complex Atomic "Enable Only" | Edge | High |`Mod_4` disabled | 1. Object has 3 enabled mods previously.<br>2. Toggle`Mod_4` using "Enable Only This". | The 3 loaded mods disable.`Mod_4` alone activates. Atomic DB batch + Rename lock. | S1 | AC-29.1.4 |
| TC-29-005 | Context Menu Trigger | Positive | High | Context action payload | 1. Mod is disabled, Object has another enabled.<br>2. Right-click mod → select Enable Only This. | Single lock fires disabling all, enabling target. | S1 | AC-29.2.1 |
| TC-29-006 | State UI changes immediately | Positive | Med | Successful atomic execution | 1. "Enable Only This" executed physically.<br>2. UI re-renders context. | React query hits invalidation. ObjectList Object count is verified to equal 1. | S2 | AC-29.2.2 |
| TC-29-007 | Enable Only This -> No-op | Edge | Low |`Mod_1` is currently enabled | 1. Mod is the single currently enabled mod in category.<br>2. Select Enable Only This. | Returns immediately, no files renamed, no DB changes issued. | S4 | AC-29.2.3 |
| TC-29-008 | Global Hash Conflict Engine | Positive | High | Mock payload with identical Hex Hashes included | 1. 100 Enabled Mods active across objects.<br>2. Press 'Run Global Collision Scan'. | Command scans >200 files returning`hash → [{path, section, line}]`. | S1 | AC-29.3.1 |
| TC-29-009 | Conflict Entry UI data | Positive | High | Hit found between`X.ini` and`Y.ini` | 1. Hash scanner returns collisions.<br>2. View results UI. | UI maps colliding hash against both relative mod names, INI paths, precise lines. | S1 | AC-29.3.2 |
| TC-29-010 | Zero Conflict return | Positive | Low | Valid distinct mod geometries | 1. Scan run with pristine mods.<br>2. Trigger collision scan. | Output: "No conflicts detected". | S3 | AC-29.3.3 |
| TC-29-011 | Ignore binary/junk`.ini` | Negative | Medium | Random`data.ini` byte dump | 1. Corrupt INI present realistically.<br>2. Trigger collision scan. | Mod logs`warn`, file skipped, rest of scan succeeds. | S2 | AC-29.3.4 |

## D. Missing / Implied Test Areas

- **[Implied] Permission Denied Atomic Stop**: Ensure when`ModA` disable physically fails due to an opened file handle (e.g., INI open in Notepad++ locked),`ModB` does not proceed to get enabled. The entire batch fails.
- **[Implied] Unicode Hashes**: Hashes using unexpected character sequences inside the ini`.txt` parsing logic handled.

## E. Open Questions / Gaps

- No specific questions.

## F. Automation Candidates

- **TC-29-004, TC-29-007**: Core Rust Unit Test simulating`enable_only_this_cmd` over`Vec<Folder>`.
- **TC-29-008**: Mock integration hash scanning test over a fixture folder filled with 200 dummy`TextOverrideHash` ini files to guarantee`<10s` benchmarking threshold.

## G. Test Environment Setup

- **Mods Collision Fixture**: A specially constructed`mods_path` directory structurally housing strictly overlapping`TextureOverride` sections inside 3 separate`*.ini` files cleverly.
- **Enabled Database State**: Seed SQLite DB physically linking conflicting.

## H. Cross-Epic E2E Scenarios

- **E2E-29-01 (Atomic Multi-Disable Pipeline)**: User navigates via ObjectList (Epic 15) to an Object possessing 5 enabled legacy mods. The user triggers the 'Enable Only This' action on a newly imported Mod (Epic 29). The backend executes a transactional operation disabling all 5 legacy items while enabling the new mod. If any I/O conflict occurs during the transaction, the entire sequence halts, and the state rollback is processed immediately without desync.
