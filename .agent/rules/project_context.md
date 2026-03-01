---
trigger: always_on
---

# EMMM2 — Project Context

**What it is**
EMMM2 is a **high-performance, native mod manager** for **3DMigoto-based games** (Genshin Impact, HSR, ZZZ, WW, Arknights Endfield).
Primary jobs: **scan/import mods**, **categorize via Deep Matcher**, **browse**, **preview/edit metadata**, **apply collections atomically**, and **keep users safe (Safe Mode)**.

## Core Principles & 42 Requirements

The project spans **42 detailed Request Specifications (`req-01` to `req-43`)** located in `.docs/requirements/`.

- **The Filesystem is the Source of Truth:** A mod is `DISABLED` if and only if its physical folder name starts with `DISABLED ` (with a trailing space). The SQLite DB is just a high-speed index.
- **Atomic ops:** Multi-step actions (Bulk Toggle, Collections) must be all-or-nothing (transactions/snapshots mindset). Protective locks are required for I/O operations.
- **Scale-first:** Assume 10k+ items; lists/grids must be virtualized using `@tanstack/react-virtual`.
- **No data loss:** Never hard delete; move to Trash. Detect collisions before rename/move.

## Core domain objects

- **Game**: `games` table — tracks installed supported titles.
- **Object**: `objects` table — mod category (Character / Weapon / UI / Other) derived via per-game `schema.json`.
- **Mod Item**: `mods` table — physical folder containing `.ini` files + metadata (`info.json`) + thumbnail cache.
- **Collection**: `collections` table — virtual loadout applied atomically.
- **Safe Mode**: Frontend & Backend privacy filter driven by `is_safe` flag (+ optional PIN gate).

## Main UI Architecture:

Default mental model: **3-panel app**

1. **ObjectList (Navigation)**: Game switcher, grouped object categories, smart filters (`req-06`, `req-07`, `req-08`). Contains ObjectList items.
2. **FolderGrid (Explorer)**: Virtualized folder grid/list with thumbnails, instant worker search, bulk select, context menu ops (`req-11`, `req-12`). Contains FolderGrid items.
3. **Preview Panel**: `.ini` editor, image gallery, metadata JSON editor, quick utility actions (`req-16` to `req-19`).

## Hard boundary (avoid out-of-scope)

No “modding engine” features (i.e. no memory injection, no shader authoring tools).
