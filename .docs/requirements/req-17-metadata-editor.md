# Epic 17: Metadata Editor

## 1. Executive Summary

- **Problem Statement**: Mod metadata (author, version, description, tags, links) is stored in an `info.json` file per mod folder — users have no in-app way to view or edit it, forcing them to open files manually in a text editor.
- **Proposed Solution**: An auto-saving metadata section inside the Preview Panel that reads and writes `info.json` via `read_mod_info` / `update_mod_info` commands, with per-field debounced blur-save, a visual "Saved" indicator, and inline validation for URL fields.
- **Success Criteria**:
  - `read_mod_info` response populates the form in ≤ 100ms (from IPC call to field render).
  - Auto-save on `onBlur` persists to `info.json` within ≤ 200ms on SSD.
  - A missing `info.json` is handled gracefully — all fields show empty/default state; a new file is created only on the first edit.
  - Corrupted `info.json` shows an error banner per field — does not crash the Preview Panel.
  - URL field "Open Link" button is disabled for any string that fails `URL` constructor validation — 0 invalid opens.

---

## 2. User Experience & Functionality

### User Stories

#### US-17.1: View Mod Info

As a user, I want to see a mod's author, version, description, and tags in the Preview Panel, so that I know what it is without opening files manually.

| ID        | Type        | Criteria                                                                                                                                                                                            |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-17.1.1 | ✅ Positive | Given a selected mod folder with a valid `info.json`, when the MetadataSection mounts, then all fields (name, author, version, description, tags, link) are populated from the file within ≤ 100ms  |
| AC-17.1.2 | ✅ Positive | Given a mod folder with no `info.json`, then all fields render with empty/default placeholders — no error; the file is created only when the user first edits a field                               |
| AC-17.1.3 | ❌ Negative | Given a malformed (invalid JSON) `info.json`, then an inline error banner "Metadata file is corrupted — editing will overwrite it" is shown; the user can still save a new clean version by editing |

---

#### US-17.2: Auto-Saving Edits

As a user, I want my metadata edits to save automatically when I blur a field, so that I never lose changes and don't need a save button.

| ID        | Type        | Criteria                                                                                                                                                                                                           |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-17.2.1 | ✅ Positive | Given I edit any field and click away (`onBlur`), then `update_mod_info(folderPath, {field: value})` is invoked; on success, a "Saved ✓" indicator pulses for 1.5s beside the field                                |
| AC-17.2.2 | ✅ Positive | Given the "Saved" indicator has shown, when I switch to a different mod without any new edits, then no extra save call is made                                                                                     |
| AC-17.2.3 | ❌ Negative | Given an `info.json` write fails (read-only file, disk full), then the field reverts to its previous value and an error toast "Save failed: {reason}" is shown                                                     |
| AC-17.2.4 | ⚠️ Edge     | Given I rapidly cycle between two mods while a save is in-flight for the first, then the in-flight save completes for the correct `folderPath` — the second mod's fields are not overwritten with the first's data |

---

#### US-17.3: Link Validation & Opening

As a user, I want to provide and open an external mod link (GameBanana, NexusMods) directly from the panel, so that I can check for updates without searching.

| ID        | Type        | Criteria                                                                                                                                                       |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-17.3.1 | ✅ Positive | Given a valid `https://` URL in the Link field, when I click the "Open Link" icon, then the OS default browser opens to that URL via `tauri::api::shell::open` |
| AC-17.3.2 | ❌ Negative | Given the Link field contains a non-URL string (e.g., `"see readme"`), then the "Open Link" button is disabled (grayed) — no shell open is attempted           |
| AC-17.3.3 | ⚠️ Edge     | Given a `javascript:` or `file:` protocol URL, then the URL validation rejects it — only `https://` and `http://` are allowed; the button stays disabled       |

---

### Non-Goals

- No rich text (Markdown) rendering in the description field — plain text only.
- No image upload via the Metadata Editor — that is Epic 19 (Image Gallery).
- No multi-select metadata batch edit — editing applies to one mod at a time.
- Tags are stored as a `Vec<String>` in `info.json` — no taxonomy or validation against a predefined tag list.

---

## 3. Technical Specifications

### Architecture Overview

```
MetadataSection.tsx
  └── useModInfo(folderPath) → invoke('read_mod_info', { folderPath }) → ModInfo
      ├── EditableField (name, author, version) → onBlur → useUpdateModInfo.mutate
      ├── TextArea (description) → onBlur → useUpdateModInfo.mutate
      ├── TagsInput (tags[]) → onChange debounced 500ms → useUpdateModInfo.mutate
      └── URLField (link) → URL validation → onBlur save + OpenLinkButton

Backend:
  read_mod_info(folder_path) → ModInfo (or empty defaults if info.json missing)
  update_mod_info(folder_path, updates: PartialModInfo) → ModInfo
    └── acquire OperationLock → acquire WatcherSuppression(folder_path)
        → read existing info.json | create new {} object
        → merge updates field-by-field
        → serde_json::to_string_pretty → fs::write(info.json)
        → drop lock → return updated ModInfo
```

### Integration Points

| Component          | Detail                                                                                                        |
| ------------------ | ------------------------------------------------------------------------------------------------------------- |
| Read Hook          | `useQuery(['modInfo', folderPath], () => invoke('read_mod_info', { folderPath }))`                            |
| Write Hook         | `useMutation(invoke('update_mod_info', ...))` with `onSuccess: setQueryData(['modInfo', folderPath], result)` |
| WatcherSuppression | Applied for `folder_path/info.json` — prevents file watcher from triggering grid re-fetch on metadata save    |
| Shell Open         | `invoke('open_url', { url })` → Rust `tauri::api::shell::open(url)` after URL allowlist check                 |

### Security & Privacy

- **URL allowlist**: Only `https://` and `http://` schemes are passed to `shell::open` — `javascript:`, `file:`, and `data:` schemes are rejected on both frontend (disabled button) and backend (allowlist check).
- **`info.json` is only written inside `mods_path`** — `folder_path` is validated with `canonicalize()` + `starts_with(mods_path)` before any write.
- **WatcherSuppression** on `info.json` prevents metadata saves from triggering spurious folder re-fetches.

---

## 4. Dependencies

- **Blocked by**: Epic 16 (Preview Panel — mounting container and `folderPath` prop), Epic 28 (File Watcher — WatcherSuppression API).
- **Blocks**: Nothing — leaf component of the Preview Panel sub-tree.
