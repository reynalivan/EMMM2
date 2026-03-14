# EMMM2 AI Agent Axioms (AGENT.md)

EMMM2: Premium Mod Orchestrator (3DMigoto Ecosystem: Genshin, HSR, ZZZ, WuWa, Endfield).

## 1. Core Principles (Axioms)

1. FS Truth: SQLite is a cache. Folder prefix `DISABLED ` is the ONLY source of truth for mod state.
2. Atomics: Bulk operations MUST be transactional (all-or-nothing). Guard heavy I/O with `OperationLock`.
3. Soft Delete: Never hard delete. Move to `./app_data/trash/`. Detect collisions before move.
4. Scale First: Virtualize all grids/lists > 50 items (@tanstack/react-virtual). 10k+ capacity.
5. Rust Compute: Offload logic (scanning, hashing, parsing) to Rust `tauri::command`. Keep FE UI thread free.

## 2. Tech Stack Consistency

- Backend: Tauri v2, Rust (tokio, sqlx, notify v7), SQLite.
- Frontend: React 19, TS 5, Zustand, TanStack Query, daisyUI 5, Tailwind 4.

## 3. Agent Execution Trajectory (Plan-Act-Reflect)

For all non-trivial tasks, force the following sequence:

- PLAN: Research context first. Create implementation_plan.md. Get User Approval. No guessing.
- ACT: Execute via ./.agent/workflows/ using ./.agent/skills/. No logic truncation (Zero-Truncation Policy).
- REFLECT: Verify via /verify-quality. Log patterns to supermemory.

## 4. Directory Standards

- Backend: ./src-tauri/src/ (commands, database, services). Repositories/DAL separation mandatory.
- Frontend: ./src/features/ (domain slices), ./src/components/ (atomic elements).
- Standards: 350 line limit per file. SSoT only. No duplicate logic across layers.

## 5. Zero Nonsense Policy

- Directive Mode: Concise technical sentences. No fluff, no conversational fillers.
- Tooling: Use narsil-mcp (Trace flow/security) and exa (Docs) before implementation.
- Safety: Full file outputs or valid unified diffs only. No placeholders (// ...).
