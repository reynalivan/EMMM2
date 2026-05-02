# EMMM Copilot Instructions (Synced with AGENT.md)

EMMM: Premium Mod Orchestrator (3DMigoto: Genshin, HSR, ZZZ, WuWa, Endfield).

## 1. Core Axioms (Absolute Truths)

- **FS Truth**: Folder prefix `DISABLED ` is the ONLY source of truth. SQLite is a cache.
- **Atomics**: Bulk operations (Toggles, Collections) MUST be transactional + `OperationLock` guarded.
- **Soft Delete**: Move to `./app_data/trash/`; never hard delete user data.
- **Scale**: Virtualize all grids/lists > 50 items (@tanstack/react-virtual); 60fps target.
- **Compute**: Heavy logic (Scanning/Hashing/Parsing) -> Rust `tauri::command`. Keep UI thread free.

## 2. Compliance (Zero-Tolerance)

- **Zero-Literal**: Frontend MUST use semantic tokens (no hex or literal Tailwind color scales like `slate-500`).
- **Zero-Hardcode**: 100% i18n (EN/ID/ZH) via `react-i18next`. No literal strings in JSX/TSX.
- **Zero-Truncation**: Full file outputs or valid Unified Diffs only; FORBIDDEN to use placeholders (`//...`).
- **IPC Safety**: ALL frontend→backend calls MUST use typed `commands` from `lib/bindings.ts`. Raw `invoke()` is PROHIBITED.
- **Post-Implementation**: Mandatory logging via `.agent/rules/post_log.md` after every session.

## 3. Architecture & Tech Stack

- **Runtime**: Tauri v2, Rust (Tokio, SQLx), React 19, TS 5, Zustand, TanStack Query, daisyUI 5, Tailwind 4.
- **Structure**: Backend (`src-tauri/src/`); Frontend (`src/features/` for slices, `src/components/` for atoms).
- **Limit**: Max 350 lines per file. Refactor heavy logic to hooks/services.
- **Pathing**: Use dynamic root `./` ONLY. No absolute paths in project code.

## 4. Decision Guard & Workflow

- **Check**: Is it the simplest path? Side-effects analyzed? DRY? No logic duplication?
- **Tools**: Use `context7` (Docs), `narsil-mcp` (Trace/Security), and `sequential-thinking` before implementation.
- **i18n**: Use modular namespaces + descriptive keys. Include tooltips and aria-labels in localization.
- **History**: Read 3-4 latest files from `.docs/history/` before planning to understand recent implementation context.
- **Plan**: Research context -> `implementation_plan.md` -> User Approval.
- **Act**: Execute via `./.agent/workflows/` and `./.agent/skills/`.
- **Verify**: Verification FIRST (Failed test or manual). Execute /verify-quality.

## 5. Operation Standards

- **Mod Status**: `DISABLED ` prefix (Capital D, trailing space) = Disabled.
- **INI Parser**: Custom line-based parser required (3DMigoto non-standard syntax).
- **Thumbnails**: `preview_custom.*` > `preview*` > first image. WebP cached.
- **Permissions**: New commands MUST be whitelisted in `src-tauri/permissions/app-commands.toml`.
