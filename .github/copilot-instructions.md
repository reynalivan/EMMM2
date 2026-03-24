# EMMM Copilot Instructions

EMMM: Premium Mod Orchestrator (3DMigoto Ecosystem).

## 1. Axioms (Absolute Truths)

1. FS Truth: Folder prefix `DISABLED ` is the ONLY source of truth. SQLite is a cache.
2. Atomics: Bulk operations (Toggles, Collections) MUST be transactional. Guard I/O with `OperationLock`.
3. Soft Delete: Never hard delete user data. Move to `./app_data/trash/`.
4. Scale First: Virtualize all grids/lists > 50 items (@tanstack/react-virtual). 60fps target.
5. Rust Compute: Offload scanning, hashing, and parsing to Rust `tauri::command`. Keep FE UI thread free.

## 2. Tech Stack Consistency

- Runtime: Tauri v2 (Edge WebView2).
- Backend: Rust (Tokio, SQLx/SQLite, Notify v7). Repositories/DAL separation.
- Frontend: React 19, TS 5, Zustand, TanStack Query, daisyUI 5, Tailwind 4.
- Pathing: Use dynamic root `./` ONLY. No absolute paths.

## 3. Directory Standards

- Backend Logic: `./src-tauri/src/services/`.
- Backend Bridge: `./src-tauri/src/commands/` (Validation/Dispatch only).
- Frontend Slices: `./src/features/` (UI + Logic + Hooks).
- Automated Ops: `./.agent/skills/` and `./.agent/workflows/`.

## 4. Developer Trajectories

- Plan: Research context first -> fresh implementation_plan.md -> User approval.
- Act: Execute via workflows. Zero-truncation policy (Full file or valid UD only).
- Verify: Failed test or manual verification FIRST. SSoT code-refactoring only.

## 5. Operation Standards

- Mod Status: `DISABLED ` prefix (Capital D, trailing space) = Disabled.
- Thumbnails: `preview_custom.*` > `preview*` > first image. WebP cached.
- INI Parser: Custom line-based parser required (3DMigoto non-standard syntax).
- Virtualization: MANDATORY for large data sets.
  > Refer to `.docs/trd.md` and `.agent/rules/` for deep technical alignment.
- Post-log rule executed, check rules on `.agent/rules/post_log.md`.
