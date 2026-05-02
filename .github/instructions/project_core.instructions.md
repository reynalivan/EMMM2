---
trigger: always_on
description: EMMM Core Architecture & Domain - High-level system design, domain objects, and tech stack constraints.
---

- FS Truth: Folder name 'DISABLED ' authoritative. DB is index cache.
- Tauri v2: Strict bridge hygiene. No direct DB/FS access from Frontend. ALL IPC via typed `commands` from Specta `bindings.ts`; raw `invoke()` PROHIBITED.
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
- Command Guard: EVERY backend command (`#[tauri::command]`) MUST be registered in `src-tauri/src/lib.rs` and whitelisted in `src-tauri/permissions/app-commands.toml` before use. Missing permissions cause runtime "Command not found" errors.
- Instance Guard: Single-instance lock required.
- Index vs Auto-Organize: USE `quickImport` (sends empty MasterDB) to ONLY index disk folders to DB silently. DO NOT use `scanPreview` or `syncDatabase` for silent indexing, as they trigger Auto-Organize (Deep Matcher mapping).
- Post-log rule executed, check rules on `.agent/rules/post_log.md`.
