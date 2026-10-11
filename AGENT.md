# EMMM AI Agent Guide (AGENT.md)

EMMM: Premium Mod Orchestrator (3DMigoto: Genshin, HSR, ZZZ, WuWa, Endfield).

## 1. Core Axioms

- **FS Truth**: Folder prefix `DISABLED ` is SSoT; SQLite is a cache.
- **Atomics**: Bulk operations MUST be transactional + `OperationLock` guarded.
- **Soft Delete**: Move to `./app_data/trash/`; never hard delete.
- **Scale**: Virtualize grids/lists > 50 items (@tanstack/react-virtual); 10k+ capacity.
- **Compute**: Heavy logic (SQL/Hashing/Parsing) -> Rust `tauri::command`.

## 2. Compliance (Zero-Tolerance)

- **Zero-Literal**: Frontend MUST use semantic tokens (no hex/Tailwind color scales).
- **Zero-Hardcode**: 100% i18n (EN/ID/ZH) via `react-i18next`. No literals in JSX/TSX.
- **No Truncation**: Full file outputs only; NO placeholders (`//...`).
- **Post-Implementation**: Run `.agent/rules/post_log.md` after every session.

## 4. Architecture Standards

- **Backend**: `src-tauri/src/` (Tauri v2, Rust, SQLite). Mandatory DAL separation.
- **Tauri IPC**: Follow `docs/knowledge/tauri-command-registration.md`; every frontend command must be registered, permission-allowlisted, generated, and covered by the registry test.
- **Tauri releases**: Follow `docs/knowledge/tauri-updater-release.md`; keep release versions synchronized, generate signed updater artifacts, and never expose or replace the updater private key.
- **Frontend**: `src/features/` (domain slices), `src/components/` (atoms).
- **Modularity**: Keep modules cohesive and easy to navigate; split large files when responsibility, state, or control flow becomes hard to reason about, never by a mechanical line limit. Single Source of Truth; no logic duplication.

## 5. Decision Guard

- **Check**: Simplest? Side-effects? DRY? No duplication? State-consistent? All code used?
- **Tools**: Use `context7`, `fetch`, `deepwiki`, `exa` for research, `narsil-mcp`, `sequential-thinking` before coding.
- **i18n**: Namespace modularity + descriptive keys Include tooltips/aria.

## 6. Windows App Verification and Demo Mode

- Default application verification on Windows is native E2E: WebdriverIO + `tauri-driver` + WebView2 against the freshly built isolated EMMM executable. Read `.agent/skills/e2e-automation/SKILL.md` and `.agent/workflows/e2e-testing.md` before app E2E work.
- Do not start a browser demo or Playwright browser session for ordinary app fixes, startup/performance checks, IPC, watcher, storage, import, or recovery verification. If native E2E is blocked, report the blocker; do not substitute browser results.
- Browser demo work requires an explicit user request for demo/fixture/browser review. Read `docs/frontend-demo.md`, then use only `pnpm dev:demo`; its in-memory fixtures must never reach user data or native services.
- Reuse the existing app, shell, routes, pages, modals, and controls. Never add alternate demo UI.
- Use typed bindings. Missing demo handlers reject rather than calling Tauri.
- Verify application behavior in the native Windows app, including loading/error states, keyboard interaction and real IPC/disk effects. Browser fixture results cannot certify native behavior. Documentation-only edits need link/format checks, not an app rebuild.

## 3. Workflow

1. **HISTORY**: Read the 3-4 latest files from `docs/history/` to understand recent context, patterns, and avoid regressions.
2. **PLAN**: Research context -> `implementation_plan.md` -> User Approval. No guessing.
3. **ACT**: Execute via `./.agent/workflows/` + `./.agent/skills/`.
4. **REFLECT**: Verify quality, eslint, build check, run test.
5. **Post-Implementation**: Run `.agent/rules/post_log.md` after every session.
