# FolderGrid — Navigable Folders vs Mod Packs Requirements

## 0) User Story

**As a** user, **I want to** explore subfolders freely with breadcrumbs and sorting, **so that** I have Windows Explorer–like control, **while** the UI prevents me from accidentally diving into noisy mod-internal folders.

FolderGrid shows folders under the currently selected **ObjectList** item and must:

- Support **double-click navigation** for real folders
- Provide **breadcrumb navigation**
- Provide **sorting** (Name, Modified, Size optional)
- Keep **Enabled/Disabled** grouping consistent with ObjectList rules

---

## 1) Key Outcome

FolderGrid must display **two kinds of items**:

1.  **Navigable folders** (safe to browse)
2.  **Mod Packs** (non-navigable by default; treat as “leaf” packs)

Users can still force-access pack internals via context menu: **Open content mods (Advanced)**.

---

## 2) Terminology

- **Node**: a folder entry displayed in FolderGrid.
- **Navigable**: double-click enters the folder.
- **Mod Pack**: a folder that represents a mod root or orchestrator; internals are not meant for browsing.
- **Variant**: alternate sub-pack folders that are mutually exclusive (e.g., “no shoes”).
- **Referenced subfolder**: a child folder referenced by `filename=...` inside a root `.ini`.

---

## 3) Data Model

Each folder node must have:

- `path`
- `display_name`
- `status`: `Enabled | Disabled` (same rule set as ObjectList)
- `node_type`: one of
  - `ContainerFolder` (navigable)
  - `ModPackRoot` (non-navigable)
  - `VariantContainer` (non-navigable)
  - `InternalAssets` (non-navigable)

- `is_navigable`: boolean derived from `node_type == ContainerFolder`
- `classification_reasons[]`: short strings for debug/UX tooltips

Optional fields:

- `pack_id` (stable internal id)
- `variant_group_id`
- `variants[]`: list of variant folder names/paths (for VariantContainer)
- `referenced_subfolders[]`

---

## 4) UI & Interaction Requirements

### FG-1 Layout

- Render two visual groups (within Enabled/Disabled):
  - **Folders** (ContainerFolder)
  - **Mod Packs** (ModPackRoot + VariantContainer)

### FG-2 Navigation

- **Double-click**:
  - `ContainerFolder` → enter folder (breadcrumb updates)
  - `ModPackRoot` → open Details/Preview (no navigation)
  - `VariantContainer` → open Variant Picker (no navigation)
  - `InternalAssets` → no navigation

### FG-3 Context Menu

All nodes:

- `Open in Explorer`

Mod packs only:

- `Enable / Disable`
- `Open details`
- `**Open content mods (Advanced)**`
  - Opens the pack’s internal file view (read-only by default)
  - Breadcrumb must indicate “Advanced” mode

Variant containers:

- `Choose variant…` (opens Variant Picker)
- Optional: `Next variant` / `Previous variant`

### FG-4 Visual Language

- `ContainerFolder`: normal folder icon
- `ModPackRoot`: package/box icon + badge `MOD PACK`
- `VariantContainer`: package icon + badge `VARIANTS`
- `InternalAssets`: dimmed folder icon + badge `INTERNAL`

### FG-5 Sorting

- Sorting applies within each visual group:
  - Name (A–Z)
  - Modified
  - Size (optional)

---

## 5) Classification Rules (Robust ModPack Detection)

Classification must be computed **offline** during scan/indexing (Epic2 pipeline) and cached.

### 5.1 Identify “valid 3DMigoto mod ini”

A `.ini` counts as a **mod ini** if it matches at least one:

- Contains `TextureOverride*` or `ShaderOverride*` sections
- Contains `Resource*` sections that define external assets via `filename=`

This avoids false positives from random `.ini` config files.

### 5.2 Extract `referenced_subfolders` from ini

Parse all `filename=` values inside:

- `Resource*` sections (e.g., Buffer/Texture)
- `CustomShader*` sections (if present)

If `filename` points to `./<Subfolder>/...` or `<Subfolder>/...`, add `<Subfolder>` to `referenced_subfolders`.

### 5.3 Classify `ModPackRoot`

A folder becomes `ModPackRoot` if:

- It contains ≥ 1 valid mod ini **at folder root**, AND
- At least one of:
  - Root contains typical mod assets (e.g., `.buf/.ib/.dds/.hlsl`) above a small threshold, OR
  - The root ini has one or more `filename=` references (dependencies) OR
  - Contains pack markers like `info.json` + preview image + root mod ini

### 5.4 Classify `VariantContainer`

A folder becomes `VariantContainer` if any:

- Root contains an orchestrator ini (e.g., `merged.ini` or large root ini) that references multiple subfolders via `filename=`
- It contains **many** sibling subfolders that look like variants (default ≥ 5)
- Multiple subfolders each contain their own valid mod ini

### 5.5 Classify child folders under packs

If a folder is `ModPackRoot` or `VariantContainer`:

- Any child folder listed in `referenced_subfolders` becomes `InternalAssets` by default.
- Any child folder that contains a valid mod ini becomes a **Variant** candidate:
  - If there are multiple such candidates, the parent is `VariantContainer` and candidates populate `variants[]`.

### 5.6 Classify `ContainerFolder`

If none of the pack rules match, classify as `ContainerFolder`.

---

## 6) Variant Picker Requirements

If a node is `VariantContainer`:

- Show a Variant Picker listing `variants[]`
- Activating a variant must:
  - Enable that variant
  - Disable other variants in the same group
  - Leave the rest of the preset selection unchanged

- Variants must respect Safe Mode filtering (if enabled)

---

## 7) Performance Requirements

- No filesystem deep scan on every click.
- Folder classification must be cached and updated incrementally:
  - Re-scan only folders/files whose signature changed (mtime/size/hash).

- Parsing `.ini` files must be limited to relevant sections (`Resource*`, `TextureOverride*`, `ShaderOverride*`).

---

## 8) Acceptance Criteria (Testable)

### AC-FG1 Container navigation

Given a `ContainerFolder`, when user double-clicks it, FolderGrid enters that folder and breadcrumbs update.

### AC-FG2 ModPack leaf behavior

Given a folder classified as `ModPackRoot`, when user double-clicks it, it does not navigate; it opens Details/Preview.

### AC-FG3 Advanced forced access

Given a `ModPackRoot`, when user selects `Open content mods (Advanced)`, EMM2 opens the internal file view (or Explorer) and indicates Advanced mode.

### AC-FG4 Variant container behavior

Given a `VariantContainer`, when user double-clicks it, EMM2 opens the Variant Picker (no folder navigation).

### AC-FG5 Dependency subfolder lock

Given a root ini references `./Hat/...` via `filename=`, the `Hat` folder is not shown as a normal navigable folder (either hidden or marked `INTERNAL`) and is only accessible via Advanced.

### AC-FG6 False positive protection

Given a folder contains an `.ini` that is not a valid mod ini (no overrides/resources), EMM2 must not classify it as a Mod Pack solely due to `.ini` presence.

### AC-FG7 Stable sorting

Given mixed nodes, sorting applies within groups and does not intermix `Folders` and `Mod Packs`.

### AC-FG8 Enabled/Disabled grouping consistency

Given ObjectList marks an item Disabled, FolderGrid must place its nodes under Disabled using the same rule set.

---

## 9) Out of Scope

- Full internal file editor inside FolderGrid (Advanced view can be read-only)
- Auto-remapping/mod patching

---

## 10) Notes

- This classification intentionally prioritizes “do not confuse the user” over raw filesystem freedom.
- Advanced access exists for power users to inspect pack internals when needed.
