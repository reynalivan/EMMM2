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
- **Frontend**: `src/features/` (domain slices), `src/components/` (atoms).
- **Limit**: 350 lines per file. Single Source of Truth; no logic duplication.

## 5. Decision Guard

- **Check**: Simplest? Side-effects? DRY? No duplication? State-consistent? All code used?
- **Tools**: Use `context7`, `fetch`, `deepwiki`, `exa` for research, `narsil-mcp`, `sequential-thinking` before coding.
- **i18n**: Namespace modularity + descriptive keys Include tooltips/aria.

## 3. Workflow

1. **PLAN**: Research context -> `implementation_plan.md` -> User Approval. No guessing.
2. **ACT**: Execute via `./.agent/workflows/` + `./.agent/skills/`.
3. **REFLECT**: Verify quality, eslint, build check, run test.
4. **Post-Implementation**: Run `.agent/rules/post_log.md` after every session.
