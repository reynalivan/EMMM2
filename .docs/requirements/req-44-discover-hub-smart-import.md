# Epic 44: Discover Hub + In-App Browser + Auto Smart Import

## 1. Executive Summary

- **Problem Statement**: Users currently need to leave the app to search for and download mods (e.g., from GameBanana) via external browsers. Afterward, they must manually coordinate imports or rely on global Downloads folder watchers, which can be unreliable or intrusive.
- **Proposed Solution**: Introduce a "Discover Hub" and a fully integrated, multi-tab "In-App Browser". Downloads are intercepted to a controlled `BrowserDownloadsRoot`. Completed files appear in a **Download Manager** where users can single-select or bulk-select files for **Auto-Organize Import** (choosing the target game before the pipeline runs). The browser opens on a configurable homepage (default: Google Search).
- **Success Criteria**:
  - Clicking "Download" on a Discover Hub mod opens the profile URL in a new in-app browser tab.
  - All downloads route to `AppData/EMM2/BrowserDownloads/` automatically (no Windows Downloads folder).
  - Completed downloads appear in the Download Manager with `Finished` status and checkboxes for selection.
  - User can single-select or bulk-select finished files and trigger "Import Selected" with a game picker dialog.
  - The browser is configurable — homepage URL defaults to `https://www.google.com` and can be changed.
  - The browser blocks non-HTTP(S) schemes and isolates remote pages from EMMM2 IPC.

---

## 2. Definitions

| Term                     | Definition                                                                                                                |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| **Discover Hub**         | The in-app mod search page pre-loaded with GameBanana, inside the in-app browser.                                         |
| **In-App Browser**       | Multi-tab Tauri WebView2 browser window managed by EMMM2. NOT an iframe.                                                  |
| **Browser Homepage**     | The URL loaded when a new tab is created or the browser is first opened. Default: `https://www.google.com`. Configurable. |
| **Tab**                  | One instance of a WebView2 window managed by the Tab Manager UI.                                                          |
| **BrowserDownloadsRoot** | `AppData/EMM2/BrowserDownloads/` — the controlled staging folder for all in-app downloads.                                |
| **Download Manager**     | A panel (drawer / sidebar / modal) listing all download items: in-progress and finished.                                  |
| **Download Session**     | Context created when user clicks "Download" from Discover Hub to correlate the file.                                      |
| **Import Job**           | A single unit of the Smart Import pipeline (staging → extract → match → place).                                           |
| **Auto-Organize**        | The post-download action: user selects a target Game → pipeline places the mod in the correct workspace.                  |

---

## 3. Scope & Non-Goals

**In Scope:**

- **Discover Hub (GameBanana)**: Search, browse, filter, card list, light detail view. Primary action: "Download".
- **In-App Browser**: Multi-tab (WebView2) — opens any HTTP/HTTPS URL. Intercepts and redirects downloads.
- **Browser Homepage**: Configurable URL. Default: `https://www.google.com`. Shown on every new tab.
- **Download Manager**: Status panel showing download items. Supports single-select and multi-select with bulk import.
- **Game Picker (Auto-Organize)**: Before importing, user selects which game's workspace to target.
- **Auto Smart Import**: Event-triggered pipeline (staging → extract → deep matcher → place as DISABLED).
- **Remote Security**: Remote web pages have **zero IPC access** to EMMM2 commands.

**Non-Goals (This Phase):**

- Direct downloads via GameBanana API (bypassing the browser).
- Embedding external websites in the main app window (iframe/embedded web).
- Importing from a locally selected file outside of BrowserDownloadsRoot.

---

## 4. UX Flows (End-to-End)

### Flow A — Discover Hub Download

1. User opens "Discover Hub" → browser opens to GameBanana (default tab).
2. User finds a mod → clicks "Download" card action.
3. EMMM2 creates a **Download Session** (`source=gamebanana`) and opens a new tab to the mod's `profile_url`.
4. User clicks the real download button on the website.
5. WebView2 `DownloadEvent::Requested` fires → EMMM2 redirects the destination to `BrowserDownloadsRoot/<path>`.
6. Download runs. Progress shown in the **Download Manager** (badge on browser icon + slide-in panel).
7. WebView2 `DownloadEvent::Finished` fires → Item in Download Manager changes to `✅ Finished`.
8. Toast: "Download complete — ready to import".
9. User opens Download Manager → selects the file (or bulk-selects multiple) → clicks **"Import Selected"**.
10. **Game Picker Dialog** appears → user picks target game.
11. Smart Import pipeline runs: staging → extract → match → place → `DISABLED`.
12. Mod appears in workspace.

### Flow B — Free URL Browse & Download

1. User clicks "+" for a new tab. Tab opens to the **Browser Homepage** (default: Google Search).
2. User navigates freely to any HTTPS site and downloads a file.
3. EMMM2 intercepts → routes to `BrowserDownloadsRoot` (no session → marked `adhoc`).
4. File appears in Download Manager as `Finished` (or `Adhoc`).
5. User selects and imports — same flow as Flow A from step 9.

### Flow C — Auto-Import (Optional, when enabled)

1. Same as Flow A/B, but when **"Auto Import after download"** setting is `ON`:
2. On `Finished` event → Game Picker dialog auto-pops (if game is ambiguous).
3. If only **one game is configured** → skip picker, auto-assign.
4. Pipeline runs automatically; user only sees the toast result.

---

## 5. In-App Browser Requirements

### 5.1 Browser Window & Tab Manager

**Tab Bar (Mandatory):**

- Display list of open tabs with favicon, title (page title or hostname fallback), and close (`×`) button.
- `+` button to open a new tab → loads **Browser Homepage**.
- Active tab is visually highlighted.

**Per-Tab UI Controls (Mandatory):**

- Address bar (URL input/display).
- **Back** / **Forward** / **Reload** and **Stop** (during load) buttons.
- **Open in external browser** button (opens current tab URL in OS default browser).
- Loading indicator (strip or spinner).
- **Download badge**: shows count of in-progress downloads in this session.

**Keyboard Shortcuts:**

| Shortcut         | Action                   |
| ---------------- | ------------------------ |
| `Ctrl+T`         | New tab (opens Homepage) |
| `Ctrl+W`         | Close current tab        |
| `Ctrl+Tab`       | Switch to next tab       |
| `Ctrl+Shift+Tab` | Switch to previous tab   |
| `Ctrl+L`         | Focus address bar        |
| `Alt+Left`       | Back                     |
| `Alt+Right`      | Forward                  |
| `F5`             | Reload                   |
| `Esc`            | Stop loading             |

### 5.2 Browser Homepage (Configurable)

- **Default URL**: `https://www.google.com`
- Every new `+` tab loads this URL automatically.
- User can change it in Settings → Browser → Homepage URL.
- Input must be a valid `http://` or `https://` URL. Invalid inputs are rejected with inline error.
- A "Reset to default" button restores `https://www.google.com`.
- The first tab on Discover Hub uses `https://gamebanana.com` as the URL, not the homepage.

### 5.3 URL Rules (Strict, No Gray Area)

- **Accepted**: `http://` and `https://` only.
- **Auto-prepend**: Input without scheme → prepend `https://`.
- **Rejected**: `file://`, `tauri://`, `data:`, or any other non-HTTP(S) scheme → show inline error, block navigation.

### 5.4 New Window Handling

- If a website triggers `window.open()` or a link targets `_blank`:
  - Open as a **new managed tab** in the same browser window.
  - All security rules (scheme restrictions, IPC isolation, download redirection) apply to the new tab.

---

## 6. Download Manager Requirements

The Download Manager is a **persistent slide-in panel** accessible from the browser toolbar. It lists all download items for the current session and all past unimported ones.

### 6.1 Download Item Fields

Each item in the Download Manager displays:

| Field         | Description                                                                     |
| ------------- | ------------------------------------------------------------------------------- |
| **Filename**  | Safe, sanitized filename.                                                       |
| **Source**    | `gamebanana` (with mod title if linked to session) or `adhoc` (URL domain).     |
| **Status**    | One of: `Downloading`, `Finished`, `Failed`, `Canceled`, `Imported`.            |
| **Progress**  | Progress bar (if engine provides bytes received / total). Hidden on `Finished`. |
| **Size**      | Final file size on finish.                                                      |
| **Timestamp** | Start time.                                                                     |
| **Actions**   | Context-sensitive buttons (see below).                                          |

### 6.2 Actions per Item

| Status        | Available Actions                         |
| ------------- | ----------------------------------------- |
| `Downloading` | Cancel                                    |
| `Finished`    | Import (single), Open file location       |
| `Failed`      | Retry, Open file location, Delete         |
| `Canceled`    | Retry, Delete partial                     |
| `Imported`    | Open mod in workspace, Open file location |

### 6.3 Single-Select Import

- User clicks the **"Import"** button on a single `Finished` item.
- **Game Picker Dialog** appears (see §7) if multiple games are configured.
- Import Job is created and tracked in the Import Queue.

### 6.4 Multi-Select (Bulk) Import

**Selection Mechanics:**

- Each item has a **checkbox** (shown on hover or when selection mode is active).
- "Select all" checkbox in the header selects all `Finished` items.
- Only `Finished` items are selectable; other statuses are greyed out.
- Counter badge: "3 selected".

**Bulk Action Toolbar (appears when ≥1 item selected):**

- **"Import Selected (N)"** — primary action.
- **"Deselect All"** — clears selection.
- **"Delete Selected"** — remove finished archive files from disk (with confirmation).

**Bulk Import Behavior:**

- Clicking "Import Selected" opens the **Game Picker Dialog ONCE** for the entire batch.
- All selected files are queued as separate Import Jobs sharing the same target game.
- The Import Queue panel opens automatically.

### 6.5 Download Manager State Persistence

- The panel state (open/closed) persists across sessions in local app state.
- Items remain in the list until explicitly cleared or marked `Imported`.
- User can manually clear `Imported` items via "Clear imported" at the top.

---

## 7. Game Picker (Auto-Organize) Requirements

The Game Picker Dialog appears before any import job is submitted when the target game is ambiguous.

### 7.1 Dialog Content

- **Title**: "Organize to which game?"
- **Game list**: All configured games from the `games` table, displaying:
  - Game icon / thumbnail.
  - Game name.
  - Mod count (e.g., "124 mods").
- **Selection**: Radio-like selection (one game per import batch).
- **Remember choice**: Checkbox "Remember for this session" — skips the dialog for subsequent imports during the same app session.
- **Buttons**: "Import" (primary, confirms selection), "Cancel".

### 7.2 Auto-skip Logic

| Condition                                                                     | Behavior                                   |
| ----------------------------------------------------------------------------- | ------------------------------------------ |
| Only 1 game configured                                                        | Skip dialog; auto-assign to the only game. |
| "Remember for this session" was checked                                       | Skip dialog; use remembered game.          |
| Import triggered from Discover Hub with Download Session that has a `game_id` | Skip dialog; use that game.                |
| Otherwise                                                                     | Always show dialog.                        |

### 7.3 Game Picker + Bulk Import

- When bulk-importing N files, one dialog appears for the whole batch.
- All N Import Jobs use the same target game.

---

## 8. Download Handling Requirements (BrowserDownloadsRoot)

### 8.1 Path & Structure

- Root: `AppData/EMM2/BrowserDownloads/`
- Layout:
  - `YYYY-MM/<session_id>/<safe_filename>.<ext>` — session-linked download.
  - `YYYY-MM/adhoc/<timestamp>_<safe_filename>.<ext>` — no session / free browse.

**`safe_filename` sanitization rules:**

- Strip Windows illegal chars: `< > : " / \ | ? *`.
- Trim leading/trailing whitespace.
- Max 120 characters (truncate if longer, preserve extension).
- On filename collision → append `_(n)` before the extension.

### 8.2 Allowed Extensions (Allowlist)

Default: `.zip`, `.7z`, `.rar`, `.tar`, `.gz`.
Configurable in Settings → Browser → Import Extensions.
Files with disallowed extensions are **not** intercepted — browser handles them normally (or blocks them).

### 8.3 Download Event State Machine

```
Requested → InProgress → Finished
                        ↘ Failed
           ↘ Canceled
```

- `Requested`: `DownloadEvent::Requested` fires; EMMM2 sets destination.
- `InProgress`: Progress updates streamed (bytes received / total).
- `Finished`: `DownloadEvent::Finished` fires with `success=true`.
- `Failed`: `DownloadEvent::Finished` fires with `success=false`, or timeout.
- `Canceled`: User pressed Cancel; partial file cleaned up.

### 8.4 Partial / Temp File Safety

- Import is **never** triggered on partial files.
- Import is triggered **only** on `Finished` event with `success=true`.
- Fallback watcher (§9.2) also enforces: file size stable for 2s AND readable.

---

## 9. Auto Import Trigger

### 9.1 Primary Trigger (Mandatory)

On `DownloadEvent::Finished` with `success=true`:

- If **Auto Import setting is OFF** → update Download Manager item to `Finished`, await user action.
- If **Auto Import setting is ON**:
  - If game is auto-assignable (§7.2) → enqueue Import Job immediately.
  - Else → show Game Picker, then enqueue.

### 9.2 Fallback Trigger (Recommended)

A targeted file watcher on `BrowserDownloadsRoot` (NOT the OS Downloads folder):

- For manual copy/paste into the folder or edge cases where the `Finished` event is missed.
- Rule: file size is stable for 2s AND file can be opened for read → treat as `Finished`.
- Only applies to files matching the allowed extensions allowlist.

---

## 10. Smart Import Pipeline Requirements

### 10.1 Staging (Atomic)

1. Generate `import_id` (UUID).
2. Copy archive from `BrowserDownloadsRoot` → `AppData/EMM2/staging/<import_id>/archive.<ext>`.
3. Extract to `.../staging/<import_id>/extracted/`.
4. **Do not touch Mods workspace** until the commit step.

### 10.2 Extract & Validate

- Validate that the archive can be opened and is not corrupt/password-protected.
- On failure: mark job `Failed: InvalidArchive`. UI shows: "Open file location", "Retry", "Delete file".

### 10.3 Deep Matcher

Runs the Epic 26 pipeline against the extracted content.

| Output field      | Type   | Description                           |
| ----------------- | ------ | ------------------------------------- |
| `object_category` | String | Character / Weapon / UI / Other       |
| `object_id`       | UUID?  | Matched object (nullable if no match) |
| `confidence`      | f32    | 0.0–1.0                               |
| `reason_summary`  | String | Short human-readable reason           |

Threshold: `confidence >= 0.70` → `AutoMatched`. Below → `NeedsReview`.

### 10.4 Place to Workspace

- Target path: `<game_mods_root>/<ObjectCategory>/<NormalizedModName>/`.
- Folder name is prefixed with `DISABLED ` to set the EMMM2-standard disabled state.
- Metadata written to `info.json` (or EMMM2 metadata store):
  - `source`: `"gamebanana"` | `"adhoc"`.
  - `source_url`: Mod profile URL.
  - `submission_id`: GameBanana submission ID (if known).
  - `imported_at`: ISO 8601 timestamp.
  - `archive_hash`: SHA-256 of the original archive (for deduplication).
  - `target_game_id`: FK to the game selected in Game Picker.

### 10.5 Deduplication

If `archive_hash` matches an existing import:

- **With UI**: Flag as duplicate. Offer "Use existing", "Replace", "Keep both".
- **Auto mode** (no UI / bulk import): Keep both, append suffix `(<n>)` to folder name.

### 10.6 Needs Review UI

If `NeedsReview` (confidence < 0.70):

- Suspend the Import Job.
- Show modal with: candidate list (category + confidence + reason), manual category picker, "Skip" option.
- On confirm → continue with Place step.

---

## 11. Download Session Correlation

### 11.1 Create Session (Flow A only)

On Discover Hub "Download" click:

```json
{
  "session_id": "uuid",
  "source": "gamebanana",
  "submission_id": "123456",
  "mod_title": "Keqing Neon Skin v2",
  "profile_url": "https://gamebanana.com/mods/123456",
  "game_id": "genshin_impact",
  "created_at": "2026-03-01T15:00:00Z",
  "expected_keywords": ["keqing", "neon", "skin"],
  "status": "awaiting_download"
}
```

### 11.2 Link Download → Session

On `DownloadEvent::Requested`:

- If the request originates from a tab associated with a session → link directly.
- Else → link to the most recent session within the last 30 minutes, or mark `adhoc`.

---

## 12. UI Surfaces (Summary)

| Surface                    | Description                                                                                                        |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **Discover Hub**           | Search, filters (game, category, sort), card grid with thumbnail, "Open" + "Download" CTAs.                        |
| **Browser Window**         | Tab bar, address bar, nav controls, URL/homepage input, download badge icon.                                       |
| **Download Manager Panel** | Slide-in panel. Lists all items with status, progress, checkboxes for multi-select, "Import Selected" bulk action. |
| **Game Picker Dialog**     | Modal to select target game before import. Auto-skipped for single game or remembered choice.                      |
| **Import Queue**           | Live list of Import Jobs: `Queued` → `Extracting` → `Matching` → `NeedsReview` → `Done` / `Failed`.                |
| **Needs Review Modal**     | Appears for low-confidence matches. Shows candidates + manual override.                                            |
| **Import Result Toast**    | On `Done`: summary + "Open mod folder", "Edit metadata". Does **not** auto-enable the mod.                         |

---

## 13. Security Requirements

- **IPC Isolation**: Remote browser tabs **must not** initialize Tauri IPC. They cannot invoke any EMMM2 command.
- **Scheme block**: `on_navigation` in Rust enforces HTTP/HTTPS only. All other schemes are cancelled with inline error.
- **No script injection**: EMMM2 must not inject any JavaScript into remote pages.
- **Download extension filter**: Files with extensions not on the allowlist pass through to browser default behavior (or are blocked).
- **Cookie/Cache**: Follows WebView2 default behavior. Settings option: "Clear browser data" (cookies, cache, history).

---

## 14. Settings (Browser & Import)

| Setting                         | Default                              | Notes                                                              |
| ------------------------------- | ------------------------------------ | ------------------------------------------------------------------ |
| Browser Homepage URL            | `https://www.google.com`             | Must be valid HTTPS. "Reset to default" available.                 |
| Auto Import after download      | `ON`                                 | Triggers pipeline immediately on `Finished`.                       |
| Skip Game Picker if single game | `ON`                                 | Auto-assigns if only one game configured.                          |
| Allowed Import Extensions       | `.zip`, `.7z`, `.rar`, `.tar`, `.gz` | Configurable list.                                                 |
| BrowserDownloads Retention      | 30 days                              | "Clear now" button available.                                      |
| BrowserDownloadsRoot Path       | `AppData/EMM2/BrowserDownloads/`     | Override allowed. Must validate writability.                       |
| Clear Browser Data              | —                                    | Button to wipe cookies, cache, and history for the in-app browser. |

---

## 15. Acceptance Criteria (Testable)

| #        | Criteria                                                                                                                                         |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-44.01 | Clicking "Download" in Discover Hub opens a new browser tab pointing to the GameBanana mod profile URL.                                          |
| AC-44.02 | A `.zip` downloaded from any in-app browser tab is saved to `BrowserDownloadsRoot`, not the Windows Downloads folder.                            |
| AC-44.03 | The `Finished` download item appears in the Download Manager with status `✅ Finished` and an enabled "Import" button.                           |
| AC-44.04 | Single-selecting a `Finished` item and clicking "Import" shows the Game Picker dialog (if multiple games exist).                                 |
| AC-44.05 | Multi-selecting 3 `Finished` items and clicking "Import Selected" shows the Game Picker once and creates 3 Import Jobs for the same target game. |
| AC-44.06 | When only 1 game is configured, the Game Picker is skipped and import proceeds automatically.                                                    |
| AC-44.07 | A new `+` tab loads the configured Browser Homepage (default: `https://www.google.com`).                                                         |
| AC-44.08 | Changing the Browser Homepage URL in Settings to `https://gamebanana.com` → next new tab loads that URL.                                         |
| AC-44.09 | Entering `file:///C:/Windows` in the address bar shows an inline error and blocks navigation.                                                    |
| AC-44.10 | Successfully imported mods appear in the target game's workspace with status `DISABLED`.                                                         |
| AC-44.11 | Opening ≥5 tabs, switching between them, and closing all — no crash occurs.                                                                      |
| AC-44.12 | Corrupted archive → Import Job status is `Failed: InvalidArchive`. UI offers "Retry" and "Open file location".                                   |
| AC-44.13 | Low-confidence match (< 0.70) → Import Job is suspended; Needs Review modal appears with candidate list.                                         |
| AC-44.14 | Remote web page inside a browser tab cannot invoke any EMMM2 Tauri command (IPC is isolated).                                                    |
| AC-44.15 | Duplicate archive (same `archive_hash`) → flagged as duplicate; auto-mode keeps both with suffix `(2)`.                                          |

---

## 16. Dependencies

- **Depends On**:
  - Epic 26 (Deep Matcher — classification pipeline).
  - Epic 23 (Mod Import Pipeline — staging & archive extraction logic).
  - Epic 02 (Game Management — `games` table used by Game Picker).
- **Blocks**: Future advanced API-based scraping / direct GameBanana API integration.
