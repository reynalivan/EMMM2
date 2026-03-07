# Test Cases: Image Gallery (Epic 19)

## A. Requirement Summary

- **Feature Goal**: Allow users to inspect mod images directly. Automatically discover`.png`,`.jpg`,`.jpeg`, and`.webp` files in the root and subdirectories (up to depth 3). Provides a Lightbox for viewing large versions, logic to promote an image as the main`preview.png`, and Ctrl+V clipboard paste support.
- **User Roles**: End User
- **User Story**:
 - US-19.1: Auto-Detect Images
 - US-19.2: Thumbnail Management
 - US-19.3: Image Paging & Lightbox
- **Success Criteria**:
 -`list_mod_preview_images` in ≤ 100ms.
 - Initial load of 50 thumbnail renders in ≤ 200ms utilizing lazy loading.
 - Lightbox modal opens ≤ 100ms.
 - "Set as Thumbnail" copies the image ≤ 300ms, and updates UI without full file reload.
 - Graceful lazy-loading prevents Out of Memory issues on large files.
- **Main Risks**: Gigantic source images crashing the React runtime by allocating too much RAM if not relying on modern browser decoding.`preview.png` replacement destroying the old thumbnail if done improperly.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-19-image-gallery.md`

- AC-19.1.1, AC-19.1.2 → TC-19-001
- AC-19.1.3 → TC-19-002
- AC-19.1.4 → TC-19-003
- AC-19.2.1, AC-19.2.2 → TC-19-004
- AC-19.2.3, AC-19.2.4 → TC-19-005
- AC-19.3.1, AC-19.3.2 → TC-19-006
- AC-19.3.3 → TC-19-007
- AC-19.3.4 → TC-19-008
- Implied Path Scoping Constraints → TC-19-009
- Phase 5: Scan depth to 3 subfolders → TC-19-010
- Phase 5: Ctrl+V clipboard paste → TC-19-011

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :--------------------------- | :------- | :------- | :--------------- | :----------------------------------------------------------------------------------------- | :--------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------- |
| TC-19-001 | Automatic Discovery | Positive | High | S3 | Target Mod contains exactly 5 image files (`.png`,`.jpg`,`.webp`) in its root/`images/`. |`Mixed png, webp` | 1. Select the Mod in Grid.<br>2. Open the Gallery Section in the Preview Panel. | Gallery explicitly finds all 5 images and renders the grid instantly (≤ 200ms). If 0 images, shows a native "Add Images" prompt. | AC-19.1.1, AC-19.1.2 |
| TC-19-002 | Lazy Loading Massive Volumes | Positive | High | S2 | Selected Mod contains 500 images. |`Folder with 500 images` | 1. Open Gallery.<br>2. Scroll aggressively to the bottom. | The viewport loads image chunks sequentially. No Out of Memory (OOM) crashes occur. The UI thread does not freeze entirely while images are sourced. | AC-19.1.3 |
| TC-19-003 | Heavy Single Image Decoding | Edge | Medium | S3 | Mod contains one massive 50MB 8K uncompressed image. |`50MB 8K uncompressed image` | 1. Open Gallery for this Mod.<br>2. Ensure the heavy file card is mapped and visible in the viewport. | The application renders the thumbnail using native browser decoding without dropping frames or hard-crashing memory limits. | AC-19.1.4 |
| TC-19-004 | Set New Thumbnail | Positive | High | S2 | Mod selected. Gallery open.`screenshot.jpg` exists. |`screenshot.jpg` | 1. Hover on`screenshot.jpg` card in the Gallery.<br>2. Click "Set as Thumbnail". | Disk copies the target as the root`preview.png` in ≤ 300ms. FolderGrid safely refetches the object thumbnail intelligently without forcing a full reset. | AC-19.2.1, AC-19.2.2 |
| TC-19-005 | No-Op Same Thumbnail | Edge | Low | S4 | Mod selected.`preview.png` exists and is visible in Gallery. |`preview.png` | 1. Hover on`preview.png` card.<br>2. Click "Set as Thumbnail". | Function completes instantly without rewriting the file or triggering unnecessary React Query cache invalidations. | AC-19.2.3, AC-19.2.4 |
| TC-19-006 | Lightbox Basic Flow | Positive | High | S3 | Mod selected with 2+ images. Gallery open. |`N/A` | 1. Click on a specific thumbnail card. | Lightbox modal visually opens in ≤ 100ms. Image explicitly expands to fit screen boundaries. "Next" and "Previous" toggles are present and responsive. | AC-19.3.1, AC-19.3.2 |
| TC-19-007 | Lightbox Exit Controls | Positive | High | S3 | Lightbox is currently open. |`N/A` | 1. Press`ESC` key (or click the background dim layer outside the image). | Lightbox closes and instantly unmounting DOM nodes securely returning focus to the Gallery. | AC-19.3.3 |
| TC-19-008 | Single Image Paging UI | Negative | Medium | S4 | Selected Mod has exactly 1 image. |`1 image` | 1. Open Lightbox. | "Next" and "Previous" arrows are strictly hidden. The sole image does not wrap confusingly. | AC-19.3.4 |
| TC-19-009 | Safe Mode Enforcements | Edge | High | S1 | Safe Mode Global Toggle is ON. User navigates to a Mod that has`is_safe: false`. |`Safe Mode ON` | 1. Select the NSFW Mod from the Grid.<br>2. Expand Gallery. | The Gallery section does NOT request local image paths via Tauri. It renders a safe privacy filter overlay blocking payload rendering strictly and totally. | Implied |
| TC-19-010 | Subfolder Scan Depth (3) | Positive | High | S2 | Mod contains image inside`variants/hair_color/blonde/art.png` (Depth 3). |`art.png` at Depth 3 | 1. Select the Mod.<br>2. Open Gallery. | The backend`list_mod_preview_images` command discovers`art.png`, despite being nested 3 folders deep. | Phase 5 |
| TC-19-011 | Clipboard Paste (Ctrl+V) | Positive | High | S2 | Clipboard currently holds valid Image binary data (e.g., Snipping Tool screenshot). |`Clipboard Image` | 1. Select Mod in Grid.<br>2. Click anywhere inside the Preview Panel (to gain focus).<br>3. Press`Ctrl+V`.<br>4. Enter requested filename in the popup modal.<br>5. Confirm. | The binary payload is transmitted to Tauri, written as a`.png` file physically into the Mod folder, and the Gallery immediately refreshes to display it. | Phase 5 |

## D. Missing / Implied Test Areas

- **Unsupported File extensions**: How does the gallery handle`bmp`,`gif`,`heic` files if they exist in the folder? (Implied: Filter discards them).
- **Corrupted Images**: What happens if an image is 0-bytes or not functionally an image despite matching extensions? (Browser native broken-image icon should handle).

## E. Open Questions / Gaps

- "Copying`preview.png` overwrites existing file". Does it delete the old image if it is inside the root, but not if it was inside`images/` folder originally? (Copying implies source integrity remains intact, only target overwrites).

## F. Automation Candidates

- **TC-19-004 (Set Thumbnail)**: Crucial UI → Backend FS boundary and React Query cache validation integration to test.
- **TC-19-009 (Safe Mode Gallery Blocking)**: Absolute requirement ensuring`is_safe` logic effectively neuters`list_mod_preview_images` commands immediately.

## G. Test Environment Setup

- **Preconditions**: OS: Windows 10/11. App: EMMM2 dev build. Custom protocol`emmm2://` registered internally returning images physically.
- **Context Injection**:
 - Sample mod payload containing`screenshot.jpg`,`art.png`,`huge50mb.webp` and nested`variants/v1/sub/hidden.jpg`.

## H. Cross-Epic E2E Scenarios

- **E2E-19-001 (Thumbnail Sync Protocol)**: User selects an active mod natively in the Grid UI (Epic 15) opening the specific Gallery Details pane (Epic 19). User invokes`Set Target Thumbnail`. Backend (Epic 41/Thumbnail System) captures payload mechanically rewriting`preview.png`. The application triggers an internal React Query invalidation causing the Folder Grid Item to fetch the fresh`emmm2://.../preview.png?timestamp=...`, preventing stale layouts.
