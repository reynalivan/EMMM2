# Epic 11: Folder Listing & Classification

## 1. Executive Summary

- **Proposed Solution**: A backend explorer projection owned by `WorkspaceViewModel` performs recursive classification per folder to distinguish between containers, terminal mod roots, and internal assets. The folder classifier is shared as the source of truth for listing and Disk Reconcile so container-only/internal asset folders are not blindly indexed as terminal mods. It employs a single-pass `fs::read_dir` strategy to identify 3DMigoto project structures, calculates folder sizes and modified timestamps, and normalizes display names by stripping "DISABLED " prefixes. There is no standalone frontend folder-list IPC; workspace UI consumes `WorkspaceViewModel.explorer.children`.
- **Success Criteria**:
  - [x] Explorer projection returns in ≤ 200ms for 500 top-level folders on an SSD.
  - [x] Correctly identifies `ModPackRoot` (has ini + mod sections).
  - [x] Strips `DISABLED ` prefix variants (`dis-`, `disable_`, `dis_`) from display names.
  - [x] Identifies "InternalAssets" (folders referenced by `filename=` in a parent mod's INI).
  - [x] Returns `classification_reasons` and **`warnings`** for every node.
  - [x] Detects `EnabledDisabledBothPresent` conflicts.
  - [x] **Path-based Inheritance**: Identifies if a folder is locked by a `DISABLED ` parent ancestor in $O(\text{depth})$.
  - Incremental classification skips cache-valid entries — re-scan time ≤ 20ms when ≤ 5% of entries have changed mtime/size.
  - `.ini` files without any `TextureOverride*`, `ShaderOverride*`, or `Resource*` sections are never falsely classified as `ModPackRoot`.
  - Malformed `info.json` isolates parse failure without breaking the rest of the directory listing.
  - Path traversal attempts (sub_path escaping `mods_path`) are blocked 100% of the time — verified by unit test.

---

## 2. User Experience & Functionality

### User Stories

#### US-11.1: List Folder Contents

As a user, I want the app to read my mods directory, so that I can see all my installed mods in the grid.

| ID        | Type        | Criteria                                                                                                                                                                                                                            |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-11.1.1 | ✅ Positive | Given an active game with a valid `mods_path`, when `WorkspaceViewModel` is requested with `explorer_sub_path = ""` (root level), then all immediate top-level object folder entries are returned as `explorer.children` in ≤ 200ms |
| AC-11.1.2 | ✅ Positive | Given `explorer_sub_path = "Characters/Albedo"`, when `WorkspaceViewModel` is requested, then only the mod folders immediately inside that path are returned — not recursive grandchildren                                          |
| AC-11.1.3 | ❌ Negative | Given a `mods_path` that no longer exists on disk, when `WorkspaceViewModel` is requested, then `source_state.status = "unavailable"` and the frontend shows a source unavailable banner — not a Rust panic                         |
| AC-11.1.4 | ⚠️ Edge     | Given a directory with ≥ 10,000 sub-folders, then explorer projection returns results without panic or OOM — using bounded memory; result may be paginated at ≥ 500 items                                                           |

---

#### US-11.2: Normalization & Classification of `DISABLED` Prefix

As a system, I want raw folder names to be parsed so `DISABLED ` prefix is stripped and reflected as `is_enabled = false`, so the UI shows clean display names to users.

| ID        | Type        | Criteria                                                                                                                                                                                                                               |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-11.2.1 | ✅ Positive | Given a folder named `DISABLED My Skin`, when listed, then `is_enabled = false` and `name = "My Skin"` in the returned `FolderEntry`                                                                                                   |
| AC-11.2.2 | ✅ Positive | Given a folder named `My Skin`, when listed, then `is_enabled = true` and `name = "My Skin"`                                                                                                                                           |
| AC-11.2.3 | ❌ Negative | Given a folder named `DISABLED DISABLED Skin`, the normalization engine strips all leading `DISABLED ` prefixes until none remain — resulting in `name = "Skin"` and `is_enabled = false` — no partial prefix left in the display name |
| AC-11.2.4 | ⚠️ Edge     | Given a folder named exactly `DISABLED` (empty name after prefix strip), the system falls back to `name = "(Unnamed Mod)"` placeholder — not an empty string or null                                                                   |

---

#### US-11.3: Recursive Folder Classification

As a system, I want to classify folders into their correct types so the UI can decide whether to show a navigable folder (ContainerFolder) or a terminal mod card (ModPackRoot/VariantContainer).

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-11.3.1 | ✅ Positive | Given a folder with no `.ini` or asset files at any depth ≤ 5, then it is classified as `ContainerFolder` — double-clicking navigates inside                                                                                                                            |
| AC-11.3.2 | ✅ Positive | Given a folder containing ≥ 1 valid mod `.ini` at folder root **AND** typical mod assets (`.buf`, `.ib`, `.dds`, `.vb`, `.hlsl`) above a threshold (≥ 2 asset files), then it is classified as `ModPackRoot`                                                            |
| AC-11.3.3 | ✅ Positive | Given a folder whose root `.ini` references multiple sub-folders via `filename=` paths, then it is classified as `VariantContainer` — its `variants[]` is populated from subfolders that each contain a valid mod `.ini`                                                |
| AC-11.3.4 | ✅ Positive | Given a subfolder named `Hat` that is referenced via `filename=./Hat/...` inside a parent's `Resource*` or `CustomShader*` section, then `Hat` is classified as `InternalAssets` — hidden from the grid (not returned in the listing)                                   |
| AC-11.3.5 | ✅ Positive | Given a folder with ≥ 5 sibling subfolders each containing their own valid mod `.ini`, it is classified as `VariantContainer` even without an orchestrator ini                                                                                                          |
| AC-11.3.6 | ❌ Negative | Given a folder containing a `.ini` file with **no** `TextureOverride*`, `ShaderOverride*`, or `Resource*` sections, then the folder is **not** classified as `ModPackRoot` on `.ini` presence alone — it remains `ContainerFolder`                                      |
| AC-11.3.7 | ❌ Negative | Given a symlink creating an infinite loop, the classifier stops at depth 5 and logs `CyclicalSymlink` error without freezing — the parent folder is still returned as `ContainerFolder`                                                                                 |
| AC-11.3.8 | ⚠️ Edge     | Given a folder that qualifies for both `ModPackRoot` and `VariantContainer` (has local `.ini` AND variant subdirs with `.ini`), then `ModPackRoot` takes priority — deterministic classification priority: `ModPackRoot > VariantContainer > ContainerFolder`           |
| AC-11.3.9 | ⚠️ Edge     | Given a `ModPackRoot` folder whose root `.ini` references `./Preset_A/` and `./Preset_B/` via `filename=`, both subfolders are listed as `InternalAssets` — they do NOT appear as `VariantContainer` children unless they each independently contain a valid mod `.ini` |

---

#### US-11.4: Incremental Classification Cache

As a system, I want folder classification to be cached and only recomputed when files change, so that repeated `WorkspaceViewModel.explorer.children` reads don't re-scan unchanged directories.

| ID        | Type        | Criteria                                                                                                                                                                                                                                      |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-11.4.1 | ✅ Positive | Given a second `WorkspaceViewModel` request for the same explorer path within the same session, when no entry has changed `mtime` or `size`, then classification results are served from the in-memory cache in ≤ 20ms — no `.ini` re-parsing |
| AC-11.4.2 | ✅ Positive | Given 1 out of 100 entries has a changed `mtime`, then only that entry is re-classified; the other 99 are served from cache                                                                                                                   |
| AC-11.4.3 | ❌ Negative | Given a folder is deleted externally between two `WorkspaceViewModel` requests, then Disk Reconcile repairs projection/cache freshness; the deleted folder does not appear in `explorer.children`                                             |
| AC-11.4.4 | ⚠️ Edge     | Given a `.ini` is modified inside a `ModPackRoot` (changing `filename=` references), then the cache key (mtime/size of the folder's content) changes — the folder is re-classified from scratch                                               |

---

#### US-11.5: Metadata Enrichment

As a system, I want each listed folder to carry its `info.json` fields and thumbnail path, so the UI can render rich mod cards without additional round-trips.

| ID        | Type        | Criteria                                                                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-11.1.1 | ✅ Positive | A folder is `ModPackRoot` if it contains a `.ini` file with valid 3DMigoto sections (`[TextureOverride...]`, etc.) AND has meaningful subfolders or assets.                                                                 |
| AC-11.1.2 | ✅ Positive | A folder is `FlatModRoot` if it is a `ModPackRoot` but its children are ONLY internal assets (referenced by INI).                                                                                                           |
| AC-11.1.3 | ✅ Positive | A folder is `VariantContainer` if it contains 3+ subfolders each containing a mod INI, or 2+ subfolders if a parent INI references them.                                                                                    |
| AC-11.1.4 | ✅ Positive | A folder is `InternalAssets` if it is referenced by a `filename=` directive in a parent INI.                                                                                                                                |
| AC-11.1.5 | ✅ Positive | A folder is `ContainerFolder` if it does not meet mod criteria (general categorization folder).                                                                                                                             |
| AC-11.5.1 | ✅ Positive | Given a folder containing `info.json`, when listed, then `author`, `description`, `version`, and `link` from the JSON are attached to the `FolderEntry` response                                                            |
| AC-11.5.2 | ✅ Positive | Given a folder containing `preview.png` or `preview.jpg`, when listed, then `thumbnail_path` is set to the absolute path — the frontend converts via `convertFileSrc()`                                                     |
| AC-11.5.3 | ❌ Negative | Given a folder with no `info.json`, then `metadata` fields are `null` in the response — the grid renders without author/description but does not crash                                                                      |
| AC-11.5.4 | ⚠️ Edge     | Given a malformed (invalid JSON) `info.json`, then the parse error is logged at `warn` level, the metadata fields are `null`, but the rest of the folder entry is still returned normally                                   |
| AC-11.5.5 | ✅ Positive | Given a folder contains a 0 KB root `.ini`, classification still determines its node type (`FlatModRoot`, `ModPackRoot`, or `VariantContainer`) where possible, while adding a warning entry describing the corrupt `.ini`. |
| AC-11.5.6 | ✅ Positive | Downstream consumers such as Collections Preview may persist and reuse the resolved terminal folder type + warning metadata from this classification layer instead of re-inferring it purely from raw paths.                |

---

### Non-Goals

- Folder listing never recurses deeper than 5 levels regardless of folder depth.
- No watching or polling in this command — that is Epic 28 (File Watcher).
- No thumbnail generation in this command — only path resolution; generation is Epic 41.
- `InternalAssets` classified folders are excluded from the returned listing but are never deleted from disk.
- Disk Reconcile uses the same terminal/container/internal decisions when building its projection; listing-only classifier forks are not allowed.
- No dynamic re-classification at runtime; the cache is invalidated per-entry by `mtime`/`size` change only.

---

## 3. Technical Specifications

### Architecture Overview

```
WorkspaceViewModel(input) → explorer.children: Vec<WorkspaceExplorerNode>
  ├── 1. Resolve absolute path: mods_path + canonicalize(sub_path)
  │         → Reject if resolved path escapes mods_path (traversal guard)
  ├── 2. fs::read_dir → rayon::par_iter over entries
  │   └── Per entry:
  │       ├── normalize_name(raw_name) → {name, is_enabled}
  │       ├── classify(entry_path, cache) → FolderType  [cache key: (path, mtime, size)]
  │       │     └── classify() rules (in priority order):
  │       │         1. has_valid_mod_ini() AND has_mod_assets() → ModPackRoot
  │       │         2. subfolder count ≥ 3 AND each has valid mod ini → VariantContainer
  │       │         3. root ini references ≥ 2 subfolders via filename= → VariantContainer
  │       │         4. none of above → ContainerFolder
  │       │     └── extract_referenced_subfolders(ini_text):
  │       │         parse Resource*, CustomShader* sections → collect filename= ./SubDir/... values
  │       │         → referenced children = InternalAssets (filtered from output)
  │       ├── enrich_metadata(entry_path) → Option<ModMetadata>  [info.json]
  │       └── resolve_thumbnail(entry_path) → Option<PathBuf>    [preview.png/jpg]
  └── 3. Filter out InternalAssets, return WorkspaceExplorerNode entries

has_valid_mod_ini(folder_path) → bool:
  for each *.ini in folder_path (root only):
    if ini_text contains [TextureOverride*] OR [ShaderOverride*] OR [Resource*] sections:
      return true
  return false

FolderEntry {
  folder_path: String,              // relative to mods_path
  name: String,                     // display name (DISABLED stripped)
  is_enabled: bool,
  ancestor_disabled_by: Option<String>, // name of first disabled parent segment
  folder_type: FolderType,          // ContainerFolder | ModPackRoot | VariantContainer | InternalAssets
  is_navigable: bool,               // derived: folder_type == ContainerFolder
  classification_reasons: Vec<String>, // debug/tooltip strings e.g. ["has-mod-ini", "has-assets"]
  warnings: Vec<String>,            // corruption/issue strings e.g. ["corrupt-ini-0kb"]
  variants: Vec<VariantEntry>,      // populated if VariantContainer
  referenced_subfolders: Vec<String>,  // subfolder names used as InternalAssets
  metadata: Option<ModMetadata>,
  thumbnail_path: Option<String>,
}
```

### Integration Points

| Component               | Detail                                                                                                                                |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Path Guard              | `std::fs::canonicalize(mods_path + sub_path)` → `starts_with(mods_path)` check — rejects traversal                                    |
| Classification Cache    | `Arc<RwLock<HashMap<(PathBuf, SystemTime, u64), FolderType>>>` — keyed by path + mtime + size                                         |
| INI Validity Check      | Scan only `TextureOverride*`, `ShaderOverride*`, `Resource*` section headers — line-by-line, no full parse                            |
| `referenced_subfolders` | Parsed from `filename=` values in `Resource*` and `CustomShader*` sections only                                                       |
| Parallelism             | `rayon::par_iter` for entry processing — `max_threads = Rayon default (num_cpus)`                                                     |
| React Query Key         | `workspaceKeys.viewModel(...)` — refreshed by `runtime-sync` descriptors from explicit mutations and Disk Reconcile results (Epic 28) |
| Thumbnail               | Path stored in `FolderEntry`; frontend converts with `convertFileSrc()` from `@tauri-apps/api`                                        |

### Security & Privacy

- **Directory traversal prevention**: `sub_path` is joined with `mods_path` and immediately `canonicalize()`d; the result is checked with `.starts_with(&mods_path_canonical)` — any path escaping the root is rejected with `PathEscapeError`.
- **Symlink depth limit**: Classifier stops recursion at depth 5 regardless of symlinks — prevents DoS via crafted link chains.
- **`info.json` is read-only** — listing never writes metadata; JSON parse errors are isolated per entry and logged, not propagated.
- **INI validation limited to section headers** — no arbitrary code evaluation; only string pattern matching for `[TextureOverride`, `[ShaderOverride`, `[Resource`.
- **Safe Mode**: If `safe_mode = true`, folders with `is_safe = false` in their linked `info.json` are excluded from the returned `Vec` — never reaching the frontend.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap — DB), Epic 02 (Game Management — `mods_path`), Epic 09 (Object Schema — classifier uses INI detection logic).
- **Blocks**: Epic 12 (Folder Grid UI — renders `WorkspaceViewModel.explorer.children`), Epic 28 (File Watcher / Disk Reconcile — refreshes WorkspaceViewModel through `runtime-sync`), Epic 41 (Thumbnail System — reads `thumbnail_path`).
