# Epic 23: Mod Import Pipeline

## 1. Executive Summary

- **Problem Statement**: Users download mod archives (`.zip`, `.rar`, `.7z`) from sites like GameBanana or NexusMods — without an in-app import pipeline, they must manually extract, rename, and place folders in the correct Object directory.
- **Proposed Solution**: A global file-drop handler that intercepts drags onto the app window, extracts archives to a temp area, normalizes folder structure (strips extra wrapper folders), runs the Deep Matcher to auto-categorize, moves the result to the correct Object directory, then emits a batch summary event.
- **Success Criteria**:
  - Full-screen "Drop to Install" overlay appears within ≤ 200ms of a drag entering the window.
  - Archive extraction and placement complete in ≤ 3s for a 50MB zip with ≤ 100 files.
  - Auto-categorization via Deep Matcher assigns the correct Object in ≥ 70% of test archives (benchmark: 50 named zips).
  - Failed categorization places the mod in "Uncategorized" — no silent discard and no crash.
  - Progress events stream at ≥ 1 update/s for multi-archive imports.

---

## 2. User Experience & Functionality

### User Stories

#### US-23.1: Drag and Drop Ingest

As a user, I want to drag a `.zip` mod file from my desktop directly into EMMM2, so that the app installs and categorizes it automatically.

| ID        | Type        | Criteria                                                                                                                                                                                                                |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-23.1.1 | ✅ Positive | Given the app window is open, when I drag any file over it, then a full-screen "Drop to Install" overlay renders within ≤ 200ms with supported format icons (`.zip`, `.rar`, `.7z`, folder)                             |
| AC-23.1.2 | ✅ Positive | Given I drop a single `.zip` file, then a progress bar shows extraction status; on completion the new folder appears in the grid's current object or in "Uncategorized"                                                 |
| AC-23.1.3 | ✅ Positive | Given I drop multiple archives at once, then each is processed sequentially with progress events streaming per archive; a batch summary toast shows "N imported / M failed" on completion                               |
| AC-23.1.4 | ❌ Negative | Given I drop a file with an unsupported extension (`.pdf`, `.exe`), then the overlay shows "Unsupported format" and the file is ignored — no extraction, no error crash                                                 |
| AC-23.1.5 | ⚠️ Edge     | Given I drop a `.zip` with a malformed or password-protected archive, then the extraction fails gracefully; the partially extracted temp folder is cleaned up; a toast shows "Import failed: {archive_name} — {reason}" |

---

#### US-23.2: Archive Analysis & Auto-Categorization

As a system, I want to analyze the archive name and contents, so that I can place the extracted mod into the correct Object bucket automatically.

| ID        | Type        | Criteria                                                                                                                                                                                                                                          |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-23.2.1 | ✅ Positive | Given a file named `Keqing_Neon_Skin_v2.zip`, when imported, then the Deep Matcher tokenizes the filename, resolves "Keqing" to the canonical Object, and places the extracted folder in the `Characters/Keqing/` directory                       |
| AC-23.2.2 | ✅ Positive | Given the archive contains an extra wrapper folder (e.g., unzip produces `Keqing_Neon_Skin_v2/Keqing_Neon_Skin/` with the actual mod inside), then the normalizer detects and strips the wrapper — the final placed folder is the actual mod root |
| AC-23.2.3 | ❌ Negative | Given the Deep Matcher cannot find a strong match (score below threshold), then the mod is placed in the "Uncategorized" object and marked with `auto_category = false` — never silently dropped                                                  |
| AC-23.2.4 | ⚠️ Edge     | Given the archive extracts to a path that already exists in the target object directory (collision), then the import pauses and a `ConflictResolveDialog` appears for that item — other archives in the batch continue                            |

---

### Non-Goals

- No mod update/version tracking in this phase — each import is treated as a new distinct mod.
- No `.rar5` or `.tar.gz` support — only `.zip`, `.rar` (v4), and `.7z`.
- No automatic enabling of newly imported mods — they are imported as disabled (no `DISABLED ` prefix; just placed as-is; `is_enabled = true` since the folder has no prefix but is a fresh unplayed mod).
- No network fetch/download — only accepts local files.

---

## 3. Technical Specifications

### Architecture Overview

```
Frontend:
  useFileDrop.ts → listen('tauri://file-drop', paths) → setDroppedFiles(paths)
    → show full-screen overlay
    → invoke('import_mods_from_paths', { game_id, paths })
    → listen('import:progress', { current, total, name, status })
    → on complete: invalidateQueries(['folders', gameId]) + ['objects', gameId]

Backend:
  import_mods_from_paths(game_id, paths: Vec<PathBuf>) → ImportBatchResult:
    for path in paths:
      1. Detect format (zip/rar/7z by extension + magic bytes)
      2. Extract to {temp_dir}/{uuid}/ (zip crate or sevenz_rust)
      3. Normalize: if extracted has single top-level folder wrapping mod, unwrap it
      4. Classify inner structure (Epic 11 classifier) → confirm ModPackRoot
      5. Run DeepMatcher.analyze(folder_name, ini_tokens) → Option<ObjectId>
      6. target_dir = if object_found: mods_path/category/object/ else mods_path/Uncategorized/
      7. Check collision → ConflictError if target exists
      8. fs::rename(temp/uuid/, target_dir/extracted_name/)
      9. emit('import:progress', {current, total, name, status: Ok | Err})
    return ImportBatchResult { success_count, failed: Vec<ImportError> }
```

### Integration Points

| Component       | Detail                                                                   |
| --------------- | ------------------------------------------------------------------------ |
| File Drop       | `tauri://file-drop` event → `useFileDrop.ts`                             |
| Extraction      | `compress-tools` crate (handles `.zip`, `.rar`, `.7z`)                   |
| Deep Matcher    | `services/scanner/deep_matcher::analyze(...)` (Epic 26)                  |
| Progress Events | `Window::emit('import:progress', ...)` per archive                       |
| Temp Cleanup    | `temp_dir` always cleaned on success or error (scoped with `scopeguard`) |
| Collision       | `ConflictResolveDialog.tsx` for per-archive conflicts                    |

### Security & Privacy

- **Archive paths are validated** before extraction — only files within the allowed OS temp dir are written during extraction.
- **Magic byte validation** supplements extension check — a `.zip` file renamed as `.rar` is still detected correctly and processed safely.
- **No shell execution** for archive extraction — only Rust native crate APIs are used; no `7z.exe` subprocess.
- **Temp dir scoped cleanup** — `scopeguard::defer!` ensures temp extracts are removed even on panic.

---

## 4. Dependencies

- **Blocked by**: Epic 09 (Object Schema + Master DB — Deep Matcher uses aliases), Epic 26 (Deep Matcher — auto-categorization logic), Epic 24 (Conflict Resolution — per-archive conflict dialog).
- **Blocks**: Nothing — import is an entry point into the mod lifecycle, not a prerequisite.
