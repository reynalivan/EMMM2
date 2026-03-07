# Epic 37: Archive Extraction Pipeline

## 1. Executive Summary

- **Problem Statement**: Mod creators package releases as ZIP, RAR, or 7Z archives with wildly varying internal structures — single wrapper folders, deeply nested author/game/character hierarchies, multi-mod packs, variant containers, and loose readme/image files mixed with mod assets. Ad-hoc extraction produces unusable nested folder structures (`MyMod/MyMod/MyMod/mesh.ib`), invalid mod imports, and orphaned archive files.
- **Proposed Solution**: A staged extraction pipeline that: (1) extracts to `{mods_dir}/.temp_extract/<uuid>`, (2) uses recursive `find_mod_roots` to discover the shallowest valid `.ini` folders, (3) classifies and routes based on Epic 11 folder types, (4) collects loose files, (5) moves to final destination with conflict resolution, and (6) backs up the source archive to `{source_dir}/.extracted/`.
- **Success Criteria**:
  - `analyze_archive_cmd` completes in ≤ 500ms for archives of any size (header-only read).
  - Smart extraction correctly identifies mod roots in ≥ 95% of test cases across all 9 documented scenarios.
  - Password prompt appears within ≤ 200ms of detecting `is_encrypted = true`.
  - Failed extraction (bad password, corruption, invalid content) cleans up all temp files within ≤ 500ms.
  - No `.ini` or asset files are left in `.temp_extract/` after any extraction (success or failure).
  - Multi-mod packs are correctly split into independent folders.

---

## 2. User Experience & Functionality

### User Stories

#### US-37.1: Pre-Extraction Analysis

As a system, I want to analyze an archive's contents before extracting, so that the UI can warn users about encryption or invalid content before any I/O.

| ID        | Type        | Criteria                                                                                                                                                                                              |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-37.1.1 | ✅ Positive | Given an archive path, when `analyze_archive_cmd` runs, then it returns `ArchiveAnalysis { format, file_count, has_ini, uncompressed_size, single_root_folder, is_encrypted }` within ≤ 500ms         |
| AC-37.1.2 | ✅ Positive | Given `is_encrypted = true` for some archives, then the frontend `ArchiveModal` groups them into "Password Protected" and prompts for per-archive passwords, while extracting non-encrypted first.    |
| AC-37.1.3 | ❌ Negative | Given a completely empty archive (0 files), then `analyze_archive_cmd` returns `{ file_count: 0, has_ini: false }` and the UI shows a warning "Archive contains no mod files" — no extraction attempt |
| AC-37.1.4 | ⚠️ Edge     | Given a multi-volume archive (`.part1.rar`, `.part2.rar`), then `analyze_archive_cmd` returns an error "Multi-volume archives not supported"                                                          |

---

#### US-37.2: Staged Extraction & Mod Root Discovery

As a user, I want the app to intelligently extract my mod without redundant nesting, correctly handle multi-mod packs, and reject invalid archives.

| ID        | Type        | Criteria                                                                                                                                                                                                                          |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-37.2.1 | ✅ Positive | Given an archive with valid `.ini` + assets at root (flat mod), when extracted, then the contents are placed in `{mods_dir}/{ArchiveName}/`                                                                                       |
| AC-37.2.2 | ✅ Positive | Given an archive with a single wrapper folder containing the mod (e.g., `ModName/merged.ini`), when extracted, then the wrapper is detected as the mod root and moved directly to `{mods_dir}/ModName/`                           |
| AC-37.2.3 | ✅ Positive | Given an archive with deep nesting (e.g., `Author/Game/Character/merged.ini` + loose `README.txt`), when extracted, then the deepest mod root `Character/` is moved to `{mods_dir}/Character/` and loose files are copied into it |
| AC-37.2.4 | ✅ Positive | Given an archive with a single mod that has variant subfolders (root `.ini` references `./VariantA/`, `./VariantB/`), when extracted, then the entire mod root is moved intact — subfolders are NOT treated as separate mods      |
| AC-37.2.5 | ✅ Positive | Given an archive with multiple independent mods (no root `.ini`, each subfolder has its own `.ini`), when extracted, then **each subfolder** is moved independently to `{mods_dir}/SubfolderName/`                                |
| AC-37.2.6 | ❌ Negative | Given an archive with no valid `.ini` file at any depth (only images/text), then extraction fails; temp dir is cleaned up; toast shows "Not a valid 3DMigoto mod archive (no valid .ini found)"                                   |
| AC-37.2.7 | ❌ Negative | Given a bad password, then extraction fails; partial temp dir is deleted within ≤ 500ms; toast shows "Password required to extract this archive"                                                                                  |
| AC-37.2.8 | ⚠️ Edge     | Given the target folder name already exists in `mods_dir`, then a counter suffix is appended: `ModName (2)`, `ModName (3)`, etc.                                                                                                  |
| AC-37.2.9 | ⚠️ Edge     | Given a bulk import of 3 archives dropped simultaneously, each is processed independently through the full pipeline — failures in one do not affect others                                                                        |

---

#### US-37.3: Source Archive Backup

As a user, I want my original archive file to be moved to a hidden `.extracted/` folder after successful extraction, so it doesn't clutter my Downloads folder and I can distinguish which mods I've already imported.

| ID        | Type        | Criteria                                                                                                                                      |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-37.3.1 | ✅ Positive | Given a successful extraction, then the source archive is moved to `{source_dir}/.extracted/{filename}` (same disk, instant rename)           |
| AC-37.3.2 | ✅ Positive | Given `mod.zip` already exists in `.extracted/`, then the backup is saved as `mod (2).zip`                                                    |
| AC-37.3.3 | ⚠️ Edge     | Given the source is on a read-only filesystem, then the backup move fails silently (logged as warning) — the extraction itself still succeeds |

---

#### US-37.4: Temp & Hidden Folder Exclusion

As a system, I want `.temp_extract/`, `.extracted/`, and other dot-folders to be hidden from the mod scanner and UI, so users never see internal system folders.

| ID        | Type        | Criteria                                                                                                                                                         |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-37.4.1 | ✅ Positive | Given a `.temp_extract/` folder exists in `mods_dir`, when `scan_mod_folders` runs, then it is excluded from results — never appears in ObjectList or FolderGrid |
| AC-37.4.2 | ✅ Positive | Given a `.extracted/` folder exists anywhere, it never appears as a mod candidate in any scanner output                                                          |

---

### Non-Goals

- No content preview inside the archive before extraction (beyond `analyze_archive_cmd` stats).
- No streaming partial extraction — full extract to temp, then move.
- No multi-volume archive support.
- No in-place editing of archived mods without extraction.

---

## 3. Technical Specifications

### Architecture Overview

```
analyze_archive_cmd(archive_path) → ArchiveAnalysis:
  Read archive headers (no full extraction)
  → format, file_count, has_ini, uncompressed_size, single_root_folder

extract_archive_cmd(archive_path, mods_dir, password?) → ExtractionResult:
  1. Acquire WatcherSuppression
  2. Disk space check (uncompressed_size + 50MB buffer)
  3. Extract to temp = {mods_dir}/.temp_extract/{uuid}
  4. On extraction error → delete_all(temp), return Err
  5. find_mod_roots(temp, max_depth=5):
     - Recurse into subfolders until valid .ini found
     - Valid .ini = has [TextureOverride*], [ShaderOverride*], or [Resource*] sections
     - Once found, STOP recursing deeper (subfolders are internal assets/variants)
  6. Classification & routing:
     ┌─ 0 mod roots → INVALID: delete temp, return Err("no valid .ini")
     ├─ 1 mod root == temp_root → FLAT MOD: wrap in {ArchiveName}/, move to mods_dir
     ├─ 1 mod root != temp_root → NESTED MOD: move mod root to mods_dir/{name}/
     │   └─ Collect loose files (txt/png/jpg) from wrapper layers → copy into mod folder
     └─ N mod roots → MULTI-MOD PACK: move each independently to mods_dir/{name}/
         └─ Loose files → copy into first mod folder
  7. Conflict resolution: if dest exists → append counter suffix "ModName (2)"
  8. Cleanup: delete remaining temp dir
  9. Source backup: move archive → {source_dir}/.extracted/{filename}
  10. Release WatcherSuppression
  11. Return ExtractionResult { dest_path, dest_paths, mod_count, files_extracted }
```

### Classification Logic (aligned with Epic 11)

| #   | Root `.ini`?            | Root assets? | Subfolders w/ `.ini`? | Epic 11 Type         | Action                               |
| --- | ----------------------- | ------------ | --------------------- | -------------------- | ------------------------------------ |
| A   | ✅ YES (valid sections) | ✅ ≥2        | Any                   | **ModPackRoot**      | Single mod → wrap/move to `mods_dir` |
| B   | ✅ YES                  | Any          | ✅ (via `filename=`)  | **VariantContainer** | Single mod w/ variants → move intact |
| C   | ❌ NO                   | ❌ NO        | ✅ Each has `.ini`    | **Multi-Mod Pack**   | Each subfolder → independent move    |
| D   | ❌ NO                   | ❌ NO        | ❌ NO                 | **Invalid**          | Delete temp → error toast            |

**Valid .ini rule**: A `.ini` file must contain at least one `[TextureOverride*]`, `[ShaderOverride*]`, or `[Resource*]` section header (AC-11.3.6). Config-only `.ini` files do NOT qualify.

### Mod File vs Loose File Classification

| Category    | Extensions                                                                               |
| ----------- | ---------------------------------------------------------------------------------------- |
| Mod files   | `.ini`, `.dds`, `.ib`, `.vb`, `.buf`, `.hlsl`                                            |
| Loose files | `.txt`, `.md`, `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.url`, `.html`, `.pdf` |

### Integration Points

| Component          | Detail                                                                                                                    |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| Archive Libraries  | `zip` crate (ZIP), `sevenz-rust` (7Z), `rar` crate (RAR)                                                                  |
| Temp Dir           | `{mods_dir}/.temp_extract/{uuid}` — cleaned up on success AND failure                                                     |
| Classify Module    | `services::mods::archive::classify` — `find_mod_roots`, `has_valid_mod_ini`, `collect_loose_files`, `resolve_unique_dest` |
| WatcherSuppression | Acquired before extraction starts; released after move complete                                                           |
| Scanner Exclusion  | `scan_mod_folders` skips all entries starting with `.` (dot-folder filter)                                                |
| Frontend           | `ArchiveModal` → `analyze_archive_cmd` → optional `PasswordInputModal` → `extract_archive_cmd`                            |
| Result Type        | `ExtractionResult { archive_name, dest_path, dest_paths, files_extracted, mod_count, success, error }`                    |

### Security & Privacy

- **`archive_path` and `mods_dir` validated** with `canonicalize()` — no path traversal into protected directories.
- **No subprocess / shell execution** — native Rust crates only; avoids command injection.
- **Temp dir is UUID-named** and scoped to `mods_dir/.temp_extract/` — no overlap with user directories.
- **Password is passed as `Option<String>` over IPC** — never logged; cleared from memory after extraction.
- **Disk space check** before extraction — prevents filling up user's drive.

---

## 4. Test Scenarios

| #   | Scenario                       | Input Structure                                                | Expected Output                                          |
| --- | ------------------------------ | -------------------------------------------------------------- | -------------------------------------------------------- |
| 1   | Flat mod (ini at root)         | `mod.zip/merged.ini + body.dds`                                | `mods_dir/mod/merged.ini + body.dds`                     |
| 2   | Single wrapper                 | `mod.zip/ModName/merged.ini`                                   | `mods_dir/ModName/merged.ini`                            |
| 3   | Author wrapper + loose files   | `mod.zip/README.txt + preview.png + Author/ModName/merged.ini` | `mods_dir/ModName/merged.ini + README.txt + preview.png` |
| 4   | Variant mod (orchestrator ini) | `mod.zip/ModName/merged.ini + ModName/VarA/ + ModName/VarB/`   | `mods_dir/ModName/` (intact with subfolders)             |
| 5   | Multi-mod pack                 | `pack.zip/ModA/merged.ini + ModB/merged.ini`                   | `mods_dir/ModA/` + `mods_dir/ModB/`                      |
| 6   | Deeply nested (3 levels)       | `mod.zip/Author/Game/Char/merged.ini`                          | `mods_dir/Char/merged.ini`                               |
| 7   | Invalid archive (no ini)       | `junk.zip/image.png + readme.txt`                              | Error: "Not a valid 3DMigoto mod archive", temp cleaned  |
| 8   | Name conflict                  | Dest `ModName/` already exists                                 | Creates `ModName (2)/`                                   |
| 9   | Bulk import (3 archives)       | Drop 3 archives at once                                        | Each processed independently with correct classification |

---

## 5. Dependencies

- **Blocked by**: Epic 28 (File Watcher — WatcherSuppression), Epic 11 (Folder Classification — classification rules), Epic 23 (Mod Import — extraction sits upstream of import metadata tracking).
- **Blocks**: Nothing — extraction sits at the top of the import pipeline.
