# Test Case Scenarios: Epic 6 - Preview Panel & Detail View

**Objective:** Validate metadata editing, image gallery slider, INI editor (BOM-safe), unsaved changes guard, lazy loading, and clipboard image handling.

**Ref:** [epic6-previewpanel.md](file:///e:/Dev/EMMM2NEW/.docs/epic6-previewpanel.md) | TRD §3.3

---

## 1. Functional Test Cases (Positive)

### US-6.1: Metadata Display & Editing

| ID            | Title                   | Pre-Condition                   | Steps                                         | Expected Result                                                   | Post-Condition | Priority |
| :------------ | :---------------------- | :------------------------------ | :-------------------------------------------- | :---------------------------------------------------------------- | :------------- | :------- |
| **TC-6.1-01** | **Edit Description**    | - Panel open on mod.            | 1. Edit text field.<br>2. Wait 1s (debounce). | - Auto-saved to `info.json`.<br>- No explicit save button needed. | Persisted.     | High     |
| **TC-6.1-02** | **Display Author/Tags** | - `info.json` has author, tags. | 1. Open panel.                                | - Author and tags rendered.<br>- Edit inline.                     | Displayed.     | Medium   |

### US-6.2: Image Gallery (Slider)

| ID            | Title                | Pre-Condition         | Steps                                | Expected Result                                                                                               | Post-Condition    | Priority |
| :------------ | :------------------- | :-------------------- | :----------------------------------- | :------------------------------------------------------------------------------------------------------------ | :---------------- | :------- |
| **TC-6.2-01** | **Lazy Slider Load** | - Mod has 20 images.  | 1. Open slider.                      | - Only visible images loaded initially.<br>- Others load on scroll/swipe.                                     | Memory efficient. | High     |
| **TC-6.2-02** | **Clipboard Paste**  | - Image in clipboard. | 1. Focus gallery area.<br>2. Ctrl+V. | - Image added to mod folder.<br>- Converted to WebP (≤ 10MB raw, compressed to WebP).<br>- Gallery refreshes. | New image.        | High     |

### US-6.3: INI Editor (BOM-Safe)

| ID            | Title                | Pre-Condition                          | Steps                            | Expected Result                                                      | Post-Condition  | Priority |
| :------------ | :------------------- | :------------------------------------- | :------------------------------- | :------------------------------------------------------------------- | :-------------- | :------- |
| **TC-6.3-01** | **Parse Variables**  | - INI with `$swapvar=1`.               | 1. Open editor.                  | - UI shows input field with value "1".<br>- Variable name displayed. | Parsed.         | High     |
| **TC-6.3-02** | **Save with Backup** | - Changed `$swapvar` to `2`.           | 1. Click Save.                   | - File updated.<br>- Backup created at `filename.ini.backup`.        | Saved + backup. | High     |
| **TC-6.3-03** | **BOM Handling**     | - INI file has UTF-8 BOM (`EF BB BF`). | 1. Open.<br>2. Edit.<br>3. Save. | - BOM detected and preserved.<br>- File remains valid after save.    | BOM intact.     | High     |

### US-6.4: Unsaved Changes Guard

| ID            | Title                         | Pre-Condition          | Steps                     | Expected Result                                                                         | Post-Condition | Priority |
| :------------ | :---------------------------- | :--------------------- | :------------------------ | :-------------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-6.4-01** | **Navigate with Dirty State** | - INI edited, unsaved. | 1. Click a different mod. | - Modal: "Unsaved changes. Save / Discard / Cancel".<br>- Cancel → stay on current mod. | Guarded.       | High     |

---

## 2. Negative Test Cases (Error Handling)

### US-6.1: Metadata Errors

| ID            | Title                       | Pre-Condition               | Steps          | Expected Result                                              | Post-Condition | Priority |
| :------------ | :-------------------------- | :-------------------------- | :------------- | :----------------------------------------------------------- | :------------- | :------- |
| **NC-6.1-01** | **Write Permission Denied** | - `info.json` read-only.    | 1. Edit field. | - Error Toast: "Cannot save metadata".<br>- Retry option.    | No data loss.  | Medium   |
| **NC-6.1-02** | **Corrupt info.json**       | - `info.json` invalid JSON. | 1. Open panel. | - Warning: "Metadata corrupt".<br>- Offer to reset/recreate. | Recoverable.   | Medium   |

### US-6.3: INI Errors

| ID            | Title                      | Pre-Condition                   | Steps               | Expected Result                                                    | Post-Condition | Priority |
| :------------ | :------------------------- | :------------------------------ | :------------------ | :----------------------------------------------------------------- | :------------- | :------- |
| **NC-6.3-01** | **Malformed Syntax**       | - `[Section` (missing bracket). | 1. Parse.           | - Error shown with line number.<br>- Load RAW view instead of GUI. | Safe.          | High     |
| **NC-6.3-02** | **Missing INI File**       | - `d3dx.ini` deleted.           | 1. Open Config tab. | - Error: "Config file not found".<br>- Editor disabled.            | Handled.       | Medium   |
| **NC-6.3-03** | **Invalid Variable Value** | - `$var` expects Int.           | 1. Input "ABC".     | - Validation: "Must be number".<br>- Block Save.                   | Blocked.       | Medium   |

### US-6.2: Image Errors

| ID            | Title                 | Pre-Condition             | Steps     | Expected Result                                     | Post-Condition | Priority |
| :------------ | :-------------------- | :------------------------ | :-------- | :-------------------------------------------------- | :------------- | :------- |
| **NC-6.2-01** | **Large Image Paste** | - Clipboard image > 10MB. | 1. Paste. | - Rejected with toast: "Image too large. Max 10MB". | Size limited.  | Medium   |

---

## 3. Edge Cases & Stability

| ID          | Title                          | Simulation Step                                        | Expected Handling                                                               | Priority |
| :---------- | :----------------------------- | :----------------------------------------------------- | :------------------------------------------------------------------------------ | :------- |
| **EC-6.01** | **Shift-JIS / GBK INI**        | 1. Open CJK-encoded INI.                               | - Detect encoding.<br>- Display properly.<br>- Save back in SAME encoding.      | High     |
| **EC-6.02** | **Concurrent External Edit**   | 1. Edit in App.<br>2. Edit in Notepad.<br>3. Save App. | - Warn "File changed externally".<br>- OR Last Write Wins with backup.          | Medium   |
| **EC-6.03** | **Huge INI (5MB)**             | 1. Open 5MB INI.                                       | - Parse latency < 1s.<br>- Virtualize rows if needed.                           | Medium   |
| **EC-6.04** | **Rapid Variable Toggle**      | 1. Toggle `$var` 50x in 5s.                            | - Debounced save.<br>- Final state matches last click.                          | High     |
| **EC-6.05** | **BOM + Shift-JIS Combo**      | 1. INI with both BOM and Shift-JIS comments.           | - Handle mixed encoding gracefully.<br>- No data corruption on save.            | Medium   |
| **EC-6.06** | **Concurrency (INI + Toggle)** | 1. Edit INI in panel.<br>2. Toggle mod Enable/Disable. | - Operation lock (TRD §3.6) prevents corruption.<br>- INI save completes first. | High     |

---

## 4. Technical Metrics

| ID          | Metric              | Threshold   | Method                        |
| :---------- | :------------------ | :---------- | :---------------------------- |
| **TM-6.01** | **INI Parse Time**  | **< 20ms**  | 500-line INI file.            |
| **TM-6.02** | **Panel Render**    | **< 100ms** | Click mod → panel ready.      |
| **TM-6.03** | **Image Lazy Load** | **< 200ms** | First visible image rendered. |

---

## 5. Data Integrity

| ID          | Object               | Logic                                                                                     |
| :---------- | :------------------- | :---------------------------------------------------------------------------------------- |
| **DI-6.01** | **INI Backup**       | Saving `XY.ini` MUST create/overwrite `XY.ini.backup` with previous content BEFORE write. |
| **DI-6.02** | **BOM Preservation** | If original file had UTF-8 BOM, saved file must retain BOM.                               |
| **DI-6.03** | **info.json Schema** | Must follow portable schema: `{ author, description, tags, preset_name, source_url }`.    |
