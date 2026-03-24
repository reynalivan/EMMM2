---
trigger: always_on
description: EMMM Core Architecture & Domain - High-level system design, domain objects, and tech stack constraints.
---

- FS Truth: Folder name 'DISABLED ' authoritative. DB is index cache.
- Tauri v2: Strict bridge hygiene. No direct DB/FS access from Frontend.
- Atomic: Ops MUST be all-or-nothing. Virtualized lists for 10k+ items.
- Core Stack: Rust (Backend) + React 19/TS 5+/Vite (Frontend). pnpm ONLY.
- DB Stack: sqlx (SQLite), notify (Watcher), tokio (Async), rayon (Parallel).
- UI Stack: daisyUI 5/Tailwind v4, zustand, react-query, react-virtual.
- Prohibited: lodash, moment, axios. Use native or date-fns/fetch.
- Game/ObjectList: DB-indexed Nav. Safe Mode zeroes counts instead of hiding.
- FolderGrid: Disk-direct view. Virtualized listing (no DB dep).
- Mod: Folder w/ .ini. Enabled if NO 'DISABLED ' prefix.
- Collection: Loadout snapshots applied via diffing + watcher suppression.
- Safe Mode: PIN gate transition (Snapshot -> Disable -> Restore).
- Watcher: notify-rs + debounce. Triggers DB Sync & GC.
- System Mod: Folder/Archive prefixed with `.` is immune to randomizer.
- Launch Engine: PowerShell RunAs Admin for loaders/games on Windows.
- Instance Guard: Single-instance lock required.
- Post-log rule executed, check rules on `.agent/rules/post_log.md`.
