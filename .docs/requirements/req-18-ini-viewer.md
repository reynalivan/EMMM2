# Epic 18: INI Viewer & Editor

## 1. Executive Summary

- **Problem Statement**: 3DMigoto mods use `*.ini` files as their functional core (shader injections, texture overrides) — without an in-app viewer and editor, power users must switch to a text editor and lose context when troubleshooting conflicts or tweaking values.
- **Proposed Solution**: An `IniEditorSection` inside the Preview Panel that lists all `.ini` files in the mod folder via `list_mod_ini_files`, displays the selected file with syntax highlighting (sections/keys/comments differentiated), and allows direct editing with an explicit Save (Ctrl+S) that writes to disk via `write_mod_ini` under `OperationLock`.
- **Success Criteria**:
  - `list_mod_ini_files` returns results in ≤ 100ms for a mod with ≤ 20 `.ini` files.
  - INI file content loads in the editor in ≤ 200ms for files up to 500KB.
  - Syntax highlighting correctly identifies sections (`[...]`), keys, values, comments (`;`) for ≥ 95% of valid 3DMigoto INI patterns in a 50-file benchmark.
  - Save (Ctrl+S or button) writes to disk in ≤ 300ms; `WatcherSuppression` prevents a spurious grid re-fetch.
  - No INI file is written if the editor content is unchanged from the loaded version ("dirty" detection).

---

## 2. User Experience & Functionality

### User Stories

#### US-18.1: Discover INI Files

As a user, I want to see which INI files exist in a mod folder, so that I can understand the mod's technical structure.

| ID        | Type        | Criteria                                                                                                                                                   |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-18.1.1 | ✅ Positive | Given a selected mod folder, when the INI section expands, then all `.ini` files found recursively within the mod root are listed in a dropdown in ≤ 100ms |
| AC-18.1.2 | ✅ Positive | Given multiple INI files, when I select one from the dropdown, then its content loads into the editor in ≤ 200ms                                           |
| AC-18.1.3 | ❌ Negative | Given a mod folder with no `.ini` files, then the INI section shows an empty state "No INI files found" — it does not crash or show an empty dropdown      |
| AC-18.1.4 | ✅ Positive | Given an `.ini` file saved with a UTF-8 BOM (U+FEFF), when read, then the BOM is silently stripped in memory, and is NOT re-added upon save                |
| AC-18.1.5 | ⚠️ Edge     | Given an `.ini` file encoded in Shift-JIS (common in old mods), then the backend auto-detects or falls back safely, displaying text correctly in the UI    |

---

#### US-18.2: Syntax Highlighting

As a user, I want the INI contents to be color-coded, so that I can quickly distinguish sections, keys, values, and comments.

| ID        | Type        | Criteria                                                                                                                                                                                     |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-18.2.1 | ✅ Positive | Given an INI file is loaded, then section headers (`[TextureOverrideAmbor]`) are highlighted in one color, key-value pairs (`hash = 123456`) in another, and comments (`;`) in a muted color |
| AC-18.2.2 | ✅ Positive | Given 3DMigoto-specific directives (e.g., `run = CommandList`, `$global_variable`), then they are tokenized and highlighted distinctly from plain values                                     |
| AC-18.2.3 | ⚠️ Edge     | Given a syntactically unusual INI (e.g., blank section header `[]`, duplicate keys), then the highlighter gracefully renders what it can — it does not throw or blank the entire editor      |

---

#### US-18.3: Direct Editing & Save

As a user, I want to edit the INI text in-place and save with Ctrl+S or a Save button, so that I can quickly tweak values without leaving the app.

| ID        | Type        | Criteria                                                                                                                                                                                                                       |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-18.3.1 | ✅ Positive | Given I edit any text in the editor, then a "Save" button appears and the editor area shows a "Unsaved changes" indicator                                                                                                      |
| AC-18.3.2 | ✅ Positive | Given unsaved changes, when I press Ctrl+S or click Save, then `write_mod_ini` is invoked; the file is overwritten on disk in ≤ 300ms; the "Unsaved" indicator disappears                                                      |
| AC-18.3.3 | ❌ Negative | Given the `.ini` file is locked by another process (the game is running), when Save is clicked, then `write_mod_ini` returns an IO error and a toast shows "Save failed: file locked" — the editor retains the unsaved content |
| AC-18.3.4 | ⚠️ Edge     | Given I switch to a different mod file while there are unsaved changes, then a "Discard changes?" confirmation dialog appears — the editor does not silently lose the user's edits                                             |
| AC-18.3.5 | ✅ Positive | Given the INI text contains `[Key...]` bindings or `$variable = value` patterns, a "Quick Actions" header parses these and provides dedicated UI inputs (cycle buttons, key inputs) that auto-update the raw text on change    |

---

### Non-Goals

- No INI AST validation (detecting semantic errors in 3DMigoto directives) — only syntax highlighting.
- No diff viewer / version history for INI edits.
- No automatic conflict detection between multiple mods' INI files (that is Epic 29 — Hash Conflict Checker).
- "Save" only supports whole-file overwrite — no partial line patch API exposed to the UI.

---

## 3. Technical Specifications

### Architecture Overview

```
IniEditorSection.tsx
  └── useIniFiles(folderPath) → invoke('list_mod_ini_files', { folderPath }) → IniFileEntry[]
      ├── Dropdown → selectedFile (IniFileEntry)
      └── useIniContent(folderPath, selectedFile.path)
              → invoke('read_mod_ini', { folderPath, fileName }) → string
          └── CodeEditor (CodeMirror 6 with custom INI tokenizer)
              ├── isDirty state (local, content !== loadedContent)
              └── onSave → invoke('write_mod_ini', { folderPath, fileName, content })

Backend:
  list_mod_ini_files(folder_path) → Vec<IniFileEntry { name, relative_path }>
    └── walkdir(folder_path, max_depth=5).filter(|e| ext == "ini")

  read_mod_ini(folder_path, file_name) → String
    └── fs::read_to_string(folder_path.join(file_name))

  write_mod_ini(folder_path, file_name, content) → ()
    └── acquire OperationLock → acquire WatcherSuppression(file_path)
        → fs::write(file_path, content) → drop lock
```

### Integration Points

| Component           | Detail                                                                                                       |
| ------------------- | ------------------------------------------------------------------------------------------------------------ |
| File List Query     | `useQuery(['iniFiles', folderPath], ...)` — re-runs whenever `folderPath` changes                            |
| File Content Query  | `useQuery(['iniContent', folderPath, selectedFilePath], ...)`                                                |
| Syntax Engine       | CodeMirror 6 with a custom `StreamLanguage` definition for 3DMigoto INI dialect                              |
| WatcherSuppression  | Applied for the specific `.ini` file path — prevents the file watcher from triggering a grid refresh on save |
| Read/Write Commands | `preview_cmds.rs` — `list_mod_ini_files`, `read_mod_ini`, `write_mod_ini`                                    |

### Security & Privacy

- **`file_name` parameter is validated** on the backend as a relative path using `starts_with(folder_path)` after join + canonicalize — prevents path traversal to write arbitrary files outside the mod folder.
- **`write_mod_ini` only writes inside `mods_path`** — same guard as all file operations.
- **WatcherSuppression** prevents the app's own INI save from triggering a folder re-fetch that could overwrite an in-progress edit.

---

## 4. Dependencies

- **Blocked by**: Epic 16 (Preview Panel — mounting context and `folderPath` prop), Epic 28 (File Watcher — WatcherSuppression API).
- **Blocks**: Epic 29 (Hash Conflict Checker — reads the same `.ini` files for hash extraction) — logically related but independent.
