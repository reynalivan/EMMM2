---
trigger: model_decision
description: DevOps: Quality & Performance - TDD workflows, performance budgets, and troubleshooting protocols.
---

- Fail Fast: No code without failing test or implementation_plan.md validation.
- Rust: #[test] (Unit), #[tokio::test] (Async), #[sqlx::test] (DB).
- FE: Vitest + RTL (Behavior-first). Mock `commands` from `bindings.ts`.
- E2E: WDIO + Tauri WebDriver. test/specs/. npm run test:e2e.
- Budgets: Startup <800ms. Scan <5s/GB. Interaction <100ms.
- Virtualization: MANDATORY for list/grid > 50 entries.
- Offload: Fuzzy search/hashing/CPU loops to Rust.
- Memory: Buffer large files; no full-file reads where avoidable.
- Troubleshooting: Analyze -> Research (Docs) -> Trace (narsil-mcp).
- Fail-Safe: Result<T, Error> (Rust); try-catch all `commands` calls (TS).
- Repair: Fix Schema/Structs, not symptomatic usage. No @ts-ignore/any.
