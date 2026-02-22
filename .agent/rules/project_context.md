---
trigger: always_on
---

# EMMM2 — Project Context

**What it is**
EMMM2 is a **high-performance mod manager** for **3DMigoto-based mods** (shader/texture overrides via a DX11 wrapper workflow). :contentReference[oaicite:0]{index=0}  
Primary jobs: **scan/import mods**, **categorize**, **browse**, **preview/edit metadata**, **apply collections atomically**, and **keep users safe (Safe Mode)**.

## Core domain objects

- **Game**: one supported title -> Genshin (GIMI) / HSR (SRMI) / ZZZ (ZZMI) / WuWa (WWMI) / Arknight Endfield (EFMI).
- **Object**: mod category (Character / Weapon / UI), defined per-game via `schema.json`.
- **Mod Item**: a folder/file set on disk + metadata + thumbnail cache.
- **Collection (Preset)**: virtual loadout applied atomically + undo/snapshot.
- **Safe Mode**: privacy filter driven by `is_safe` flag (+ optional PIN gate).

## Main UI Architecture:

Default mental model: **3-panel app**

1. **Navigator (sidebar left)**: game switcher, object categories, filters.
2. **Explorer (center)**: folder grid/list w/ thumbnails, search, sort, bulk select, context menu ops.
3. **Preview (panel right)**: details, metadata editor, INI/text viewer, image gallery, quick actions.

## Non-negotiables (product constraints)

- **No data loss**: never hard delete; move to Trash; detect collisions before rename/move.
- **Atomic ops**: multi-step actions must be all-or-nothing (transactions/snapshots mindset).
- **Scale-first**: assume thousands of items; lists/grids must be virtualized. :contentReference[oaicite:1]{index=1}

## Feature summary

1 Onboarding: setup, discover games, persist config (Tauri store). :contentReference[oaicite:2]{index=2}  
2 Scanning/Deep Matcher: import + auto-categorize (name/content/AI/fuzzy).  
3 Navigation: object sidebar, filter, search.  
4 Folder Grid: thumbnails + caching + file watching.  
5 Core Ops: toggle/rename/standardize (safe moves).  
6 Preview: details + INI editor + gallery.  
7 Privacy: Safe/NSFW mode (+ optional PIN).  
8 Collections: loadouts + snapshot/undo.  
9 Dedup: storage optimization (hashing).  
12 Updates: app/data updater pipeline (Tauri updater). :contentReference[oaicite:3]{index=3}  
13 Dashboard: basic analytics/aggregation.

## Hard boundary (avoid out-of-scope): No “modding engine” features (inject/shader authoring).
