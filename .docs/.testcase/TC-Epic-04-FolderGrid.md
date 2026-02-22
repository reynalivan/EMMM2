# Test Case Scenarios: Epic 4 - Folder Grid & Explorer

**Objective:** Validate file explorer capabilities, deep navigation, sorting, WebP thumbnail caching, keyboard navigation, external DnD, custom trash with metadata, and watcher suppression.

**Ref:** [epic4-foldergrid-manage.md](file:///e:/Dev/EMMM2NEW/.docs/epic4-foldergrid-manage.md) | TRD §2.2, §3.5

---

## 1. Functional Test Cases (Positive)

### US-4.1: Pro-Level Navigation & Sorting

| ID            | Title                 | Pre-Condition                     | Steps                                                | Expected Result                                  | Post-Condition | Priority |
| :------------ | :-------------------- | :-------------------------------- | :--------------------------------------------------- | :----------------------------------------------- | :------------- | :------- |
| **TC-4.1-01** | **Deep Navigation**   | - `/Raiden/Set1` exists.          | 1. Double-click "Raiden".<br>2. Double-click "Set1". | - Grid updates.<br>- Breadcrumb: `/Raiden/Set1`. | Navigated.     | High     |
| **TC-4.1-02** | **Sort by Date**      | - Items with different dates.     | 1. Click "Sort: Modified".                           | - Items reordered by date.<br>- State persisted. | Sorted.        | Medium   |
| **TC-4.1-03** | **State Persistence** | - Sorted by Name, view mode Grid. | 1. Navigate away.<br>2. Navigate back.               | - Sort and view mode restored from Zustand.      | Persisted.     | Medium   |

### US-4.2: Thumbnails (WebP Cache)

| ID            | Title                      | Pre-Condition               | Steps                                              | Expected Result                                                             | Post-Condition | Priority |
| :------------ | :------------------------- | :-------------------------- | :------------------------------------------------- | :-------------------------------------------------------------------------- | :------------- | :------- |
| **TC-4.2-01** | **Lazy Load 1000 Items**   | - 1000 folders with images. | 1. Scroll grid.                                    | - No freeze. Images fade in.<br>- TanStack Virtual active.                  | FPS > 50.      | High     |
| **TC-4.2-02** | **Custom Thumbnail Paste** | - Clipboard has PNG.        | 1. Right-click mod → "Set Thumbnail".<br>2. Paste. | - Converted to WebP.<br>- `preview_custom.webp` created.<br>- Size ≤ 500KB. | Updated.       | High     |

### US-4.3: Keyboard Navigation

| ID            | Title                   | Pre-Condition                | Steps                | Expected Result                                            | Post-Condition | Priority |
| :------------ | :---------------------- | :--------------------------- | :------------------- | :--------------------------------------------------------- | :------------- | :------- |
| **TC-4.3-01** | **Arrow Key Selection** | - Grid focused.              | 1. Press Arrow keys. | - Focus moves through items.<br>- Visual focus ring shown. | Item focused.  | High     |
| **TC-4.3-02** | **Enter to Open**       | - Item focused via keyboard. | 1. Press Enter.      | - Navigates into folder (same as double-click).            | Navigated.     | Medium   |

### US-4.4: External Drag & Drop

| ID            | Title                         | Pre-Condition            | Steps                         | Expected Result                                                                          | Post-Condition   | Priority |
| :------------ | :---------------------------- | :----------------------- | :---------------------------- | :--------------------------------------------------------------------------------------- | :--------------- | :------- |
| **TC-4.4-01** | **Drop Folder from Explorer** | - Windows Explorer open. | 1. Drag mod folder onto grid. | - Folder copied/moved to current path.<br>- DB updated.<br>- Toast: "Import successful". | New mod visible. | High     |

### US-4.5: Soft Delete (Custom Trash)

| ID            | Title               | Pre-Condition          | Steps            | Expected Result                                                                                      | Post-Condition | Priority |
| :------------ | :------------------ | :--------------------- | :--------------- | :--------------------------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-4.5-01** | **Delete to Trash** | - Mod folder selected. | 1. Press Delete. | - Moved to `./app_data/trash/`.<br>- Metadata JSON created for restore.<br>- Toast with "Undo" (5s). | In trash.      | High     |

---

## 2. Negative Test Cases (Error Handling)

### US-4.1: Navigation Errors

| ID            | Title                         | Pre-Condition             | Steps              | Expected Result                                                  | Post-Condition | Priority |
| :------------ | :---------------------------- | :------------------------ | :----------------- | :--------------------------------------------------------------- | :------------- | :------- |
| **NC-4.1-01** | **Access Restricted**         | - Folder 700 permissions. | 1. Open folder.    | - Toast: "Permission Denied".<br>- Stay on parent.               | Stable.        | Medium   |
| **NC-4.1-02** | **Folder Deleted Externally** | - Viewing folder X.       | 1. Delete X in OS. | - Detect missing via watcher.<br>- Auto-navigate to parent/root. | Correct state. | High     |

### US-4.2: Thumbnail Errors

| ID            | Title                     | Pre-Condition                | Steps                  | Expected Result                                                 | Post-Condition | Priority |
| :------------ | :------------------------ | :--------------------------- | :--------------------- | :-------------------------------------------------------------- | :------------- | :------- |
| **NC-4.2-01** | **Corrupt Image**         | - `preview.webp` is 0 bytes. | 1. Load grid.          | - Show "Broken Image" placeholder.<br>- No UI crash.            | Graceful.      | Medium   |
| **NC-4.2-02** | **Oversized Image Paste** | - Clipboard image > 10MB.    | 1. Paste as thumbnail. | - Rejected: "Image too large. Max 10MB.".<br>- No file written. | Blocked.       | Medium   |

### US-4.5: Soft Delete Errors

| ID            | Title                    | Pre-Condition                   | Steps                               | Expected Result                                         | Post-Condition | Priority |
| :------------ | :----------------------- | :------------------------------ | :---------------------------------- | :------------------------------------------------------ | :------------- | :------- |
| **NC-4.5-01** | **File Locked**          | - File open in Photoshop.       | 1. Delete mod.                      | - Error: "File in use".<br>- Abort move.                | File remains.  | High     |
| **NC-4.5-02** | **Trash Drive Full**     | - Drive full.                   | 1. Delete mod.                      | - Error: "Disk Full".<br>- Original preserved.          | Safe.          | High     |
| **NC-4.2-03** | **Invalid Paste (Text)** | - Text in clipboard (no image). | 1. Right-click → "Paste Thumbnail". | - Toast: "No image found in clipboard".<br>- No action. | No change.     | Medium   |
| **NC-4.1-03** | **Naming Conflict**      | - "ModA" folder exists.         | 1. Rename "ModB" → "ModA".          | - Error: "Folder already exists".<br>- Rename blocked.  | No rename.     | High     |

---

## 3. Edge Cases & Stability

| ID          | Title                      | Simulation Step                                       | Expected Handling                                                        | Priority |
| :---------- | :------------------------- | :---------------------------------------------------- | :----------------------------------------------------------------------- | :------- |
| **EC-4.01** | **Path > 260 Chars**       | 1. Nest folders deeply.                               | - Use `dunce` crate (UNC paths).<br>- Access works.                      | High     |
| **EC-4.02** | **10,000 Items in Folder** | 1. Generate 10k folders.                              | - TanStack Virtual active.<br>- Only ~20 DOM nodes.<br>- RAM stable.     | High     |
| **EC-4.03** | **Symlink Loops**          | 1. Symlink to self.                                   | - Detect loop.<br>- Show as file or block entry.                         | Medium   |
| **EC-4.04** | **Rapid Navigation**       | 1. Click Back/Forward 20x fast.                       | - Debounce operations.<br>- Final state matches breadcrumb.              | Medium   |
| **EC-4.05** | **Watcher Suppression**    | 1. Paste thumbnail (app action).<br>2. Watcher fires. | - Watcher suppressed during app I/O (TRD §3.5).<br>- No infinite loop.   | High     |
| **EC-4.06** | **Orphaned info.json**     | 1. `info.json` exists but mod folder empty.           | - Handled gracefully.<br>- Show empty state or cleanup.                  | Low      |
| **EC-4.07** | **Rapid Window Resizing**  | 1. Resize window aggressively 20x.                    | - Grid columns adjust dynamically.<br>- No layout break or misalignment. | Medium   |

---

## 4. Technical Metrics

| ID          | Metric              | Threshold    | Method                               |
| :---------- | :------------------ | :----------- | :----------------------------------- |
| **TM-4.01** | **Scroll FPS**      | **> 50 FPS** | Scroll 10k items (TanStack Virtual). |
| **TM-4.02** | **Thumb Cache Hit** | **< 100ms**  | L2 disk cache load time.             |
| **TM-4.03** | **Memory Usage**    | **< 200MB**  | 10k items loaded.                    |

---

## 5. Data Integrity

| ID          | Object                  | Logic                                                                                                       |
| :---------- | :---------------------- | :---------------------------------------------------------------------------------------------------------- |
| **DI-4.01** | **Custom Trash**        | Deleted items in `./app_data/trash/` MUST have metadata JSON for restore (original path, delete timestamp). |
| **DI-4.02** | **Thumbnail Format**    | All generated thumbnails stored as `.webp`, NOT `.png`.                                                     |
| **DI-4.03** | **info.json Lifecycle** | `info.json` created on first metadata edit. Read on panel open. Never deleted by app.                       |
