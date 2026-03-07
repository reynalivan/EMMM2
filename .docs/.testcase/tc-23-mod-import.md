# Test Cases: Mod Import Pipeline (Epic 23)

## A. Requirement Summary

- **Feature Goal**: Streamline adding mods by letting users drag and drop archives (`.zip`,`.rar`,`.7z`) into the app. Pipeline handles extraction, cleanup of extra folders, auto-categorization (via Deep Matcher), conflict handling, and final placement. Includes Phase 5 additions like the 5s Undo Toast, Duplicate Character Warnings, and Shader Conflict Notices.
- **User Roles**: End User
- **User Story**:
 - US-23.1: Drag and Drop Ingest
 - US-23.2: Archive Analysis & Auto-Categorization
- **Success Criteria**:
 - Drop UI overlay responds ≤ 200ms.
 - Extraction < 3s for a 50MB`<100` file archive using the new`compress-tools` backend.
 - Deep Matcher identifies accurate target Object for 70% of standard files, defaulting to "Uncategorized" automatically upon miss without failing.
 - Temp contexts systematically swept safely upon success and error unconditionally.
 - 5-second Undo Toast appears upon successful import, allowing immediate rollback.
- **Main Risks**: Destructive extraction loop vulnerabilities (zip bombs draining memory entirely or overriding core OS files through path traversal). Failed cleanups cluttering OS temp over time permanently.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-23-mod-import.md`

- AC-23.1.1 → TC-23-001
- AC-23.1.2 → TC-23-002
- AC-23.1.3 → TC-23-003
- AC-23.1.4 → TC-23-004
- AC-23.1.5 → TC-23-005
- AC-23.2.1 → TC-23-006
- AC-23.2.2 → TC-23-007
- AC-23.2.3 → TC-23-008
- AC-23.2.4 → TC-23-009
- Phase 5: compress-tools extraction → TC-23-010
- Phase 5: 5s Undo Toast Flow → TC-23-011
- Phase 5: Duplicate Character Warning → TC-23-012
- Phase 5: Shader Conflict Notice → TC-23-013

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :-------------------------------------- | :------- | :------- | :--------------- | :------------------------------------------------------------------------------------------------------- | :------------------------------ | :---------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------- |
| TC-23-001 | Drop Overlay Trigger | Positive | High | S3 | App is active. Target game is selected. |`Valid.zip` | 1. Drag the`.zip` file from OS over the application window. | UI overlay explicitly triggers in ≤ 200ms clearly showing "Drop to Install" and appropriate dropping zone icons. | AC-23.1.1 |
| TC-23-002 | Single Archive Pipeline | Positive | High | S1 | App is active. Valid mod archive ready. |`.zip with mod payload` | 1. Drop the valid mod archive into the app.<br>2. Wait for confirmation toast. | Extracts files, executes Deep Matcher precisely, sets structure, resolves to categorized folder without error, and physical files are placed. | AC-23.1.2 |
| TC-23-003 | Multi-Archive Sequential Streaming | Positive | Medium | S2 | 3 distinct`.zip` files selected simultaneously in OS. |`3 distinct archives` | 1. Drop all 3 archives at once. | Processing acts. Displays progress tracking "1/3 imported". Final toast summarizes successes and failures. | AC-23.1.3 |
| TC-23-004 | File Type Whitelisting | Negative | Medium | S2 | Unsupported payloads ready. |`.exe,.pdf payload` | 1. Drop unsupported extension into the app dropzone. | Dropzone rejects firmly, displaying a toast "Unsupported format". UI overlay clears. Backend is strictly bypassed. | AC-23.1.4 |
| TC-23-005 | Corrupted/Protected Content Safe Reject | Edge | High | S1 | Archive requires a password or has broken headers. |`Password locked.zip` | 1. Drop the corrupted/locked archive.<br>2. Check`AppData/Temp` folder manually for residue. | Fails extraction securely returning clear Error Toast. Crucially: wipes temporary extract folder entirely via`scopeguard`, leaving no temp files. | AC-23.1.5 |
| TC-23-006 | Deep Matcher Pathing | Positive | High | S2 | Mod archive contains clear character identifiers in folder name or inside`.ini`. |`Keqing_Neon_Skin_v2.zip` | 1. Ensure Deep matcher has alias mapping for`Keqing`.<br>2. Import test file.<br>3. Locate target in Grid. | Mod is categorized and mapped to the`Characters/Keqing` object list folder. | AC-23.2.1 |
| TC-23-007 | Wrapper Eradication | Variable | High | S2 | Archive contains a single parent folder (e.g.,`Export/`) wrapping the actual mod root (the`.ini` dir). |`Archive inside parent wrapper` | 1. Import the wrapped file.<br>2. Inspect the imported folder structure on disk. | Extractor effectively bypasses parent directory securely extracting actual mod root, placing the`.ini` at the root of the new target Mod folder. | AC-23.2.2 |
| TC-23-008 | No-Match Fallback Logic | Positive | High | S3 | Archive contains no identifiable character names. |`randommodtest1.7z` | 1. Import file with nonsense name. | Scans, misses, and defaults gracefully into the generic "Uncategorized" ObjectList. Import strictly succeeds. | AC-23.2.3 |
| TC-23-009 | Import Target Context Collision | Negative | High | S1 | Pre-existing mod folder named exactly the same as the incoming target directory name. |`Archive matching existing` | 1. Import file that will resolve to an already existing directory. | Extraction halts exactly before final directory rename.`ConflictResolveDialog` pops asking to "Overwrite, Keep Both, or Cancel". Existing files remain safe. | AC-23.2.4 |
| TC-23-010 |`compress-tools` Extraction (.rar,.7z) | Positive | High | S1 | Standard`.rar` or`.7z` file containing valid mod payload. |`mod.rar`,`mod.7z` | 1. Drop`.rar` or`.7z` archive into the app.<br>2. Verify extraction. | The new Rust`compress-tools` backend handles the formats, extracting contents in ≤ 3s for 50MB files without external dependencies. | Phase 5 |
| TC-23-011 | 5s Undo Toast Flow | Positive | High | S2 | Any valid archive. |`Valid.zip` | 1. Drop archive.<br>2. Immediately click the "Undo" button on the green success toast within 5 seconds. | Success toast appears with Undo option. Clicking Undo immediately deletes the imported folder physically from disk and removes it from the Grid state. | Phase 5 |
| TC-23-012 | Duplicate Character Warning | Edge | Medium | S3 | A Mod for`Venti` already exists and is **ENABLED**. |`Venti_NewSkin.zip` | 1. Import a new mod that gets categorized as`Venti`.<br>2. Observe Toast or Notification system. | After import, a non-blocking warning (Toast/Alert) clearly states: "You now have 2 mods enabled for Venti. This may cause conflicts." | Phase 5 |
| TC-23-013 | Shader Conflict Notice | Edge | Medium | S3 | Incoming mod contains specific shader overrides known to conflict globally. |`GlobalShader.zip` | 1. Import the archive.<br>2. Check UI notifications. | The system scans for shader overrides upon ingestion. If generic global hashes are found, a warning notice is emitted indicating potential shader conflicts. | Phase 5 |

## G. Test Environment Setup

- **Preconditions**: OS: Windows 10/11. App: EMMM2 dev build. Object Panel and File Watcher fully active.
- **Context Injection**:
 - Sample archives provided:`.zip`,`.rar`,`.7z` generated safely mapping to explicit match conditions manually.

## H. Cross-Epic E2E Scenarios

- **E2E-23-001 (Drag & Drop to Explorer Grid)**: User drops`Mod_Archive.7z` actively over the main UI globally (Epic 23). The App mechanically triggers extraction using`compress-tools`. During ingestion, Deep Matcher scans the headers identifying the Category (Epic 26). Final destination physically created.`WatcherSuppression` triggers to mask the direct initial rename.`FolderGrid` refetches displaying the Card. A 5-second Undo Toast appears, but the user ignores it, leaving the mod installed.
