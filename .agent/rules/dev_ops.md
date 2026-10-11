---
trigger: model_decision
description: DevOps: Quality & Performance - TDD workflows, performance budgets, and troubleshooting protocols.
---

- Fail Fast: No code without failing test or implementation_plan.md validation.
- Rust: #[test] (Unit), #[tokio::test] (Async), #[sqlx::test] (DB).
- FE: Vitest + RTL (Behavior-first). Mock `commands` from `bindings.ts`.
- E2E: Native Windows app through WDIO + `tauri-driver` + WebView2. Specs live in `tests/e2e/specs/`; use `wdio.conf.ts` and `.agent/skills/e2e-automation/SKILL.md`.
- Budgets: Startup <800ms. Scan <5s/GB. Interaction <100ms.
- Virtualization: MANDATORY for list/grid > 50 entries.
- Offload: Fuzzy search/hashing/CPU loops to Rust.
- Memory: Buffer large files; no full-file reads where avoidable.
- Troubleshooting: Analyze -> Research (Docs) -> Trace (narsil-mcp).
- Fail-Safe: Result<T, Error> (Rust); try-catch all `commands` calls (TS).
- Repair: Fix Schema/Structs, not symptomatic usage. No @ts-ignore/any.

## Windows Native E2E and Demo Boundaries

- Default to native Windows E2E for application UI and behavior, startup/reopen, performance, IPC, watcher, filesystem, import and recovery. Unit/component tests supplement native checks; choose focused specs appropriate to the change.
- Do not launch browser tooling or `pnpm dev:demo` for ordinary application verification. A blocked native build/driver is a reported limitation, not permission to switch to browser QA.
- Start `pnpm dev:demo` only when the user explicitly requests demo/fixture/browser review. The mode is for the Vite development server only.
- Keep the production build path free of demo fixture modules. `pnpm exec vite build --mode demo` must fail with the development-only guard.
- After changing fixture content or adapters, run focused tests, TypeScript typecheck, the production build, and `git diff --check`.
- For explicitly requested browser demo work, exercise only the affected fixture screens and report fixture-only coverage. Do not claim native IPC, persistence, process restart or watcher correctness from these results.
- Follow `docs/frontend-demo.md` for the supported fixture boundary and the full screen checklist.
