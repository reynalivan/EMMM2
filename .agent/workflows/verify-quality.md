---
description: Zero-Tolerance Verification. Enforces Repair Standards and Code Integrity.
---

1.  **🕵️ AUTOMATED AUDIT**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/code-review/SKILL.md`.
    - **Action:** Use `grep_search` to find `@ts-ignore`, `any`, `unwrap()`, or `todo!` in the codebase.
    - **Tool:** Use `mcp_narsil-mcp_check_type_errors` and `mcp_narsil-mcp_scan_security` for deep Rust/TS static analysis.
    - **Action:** FAIL if found (unless strictly temporary and documented).

2.  **🦀 BACKEND CHECK**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/writing-unit-tests/SKILL.md`.
    - `cargo fmt -- --check`
    - `cargo clippy -- -D warnings` (Strict)
    - `cargo test`

3.  **⚛️ FRONTEND CHECK**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/vercel-react-best-practices/SKILL.md`.
    - `pnpm tsc --noEmit` (Type Safety)
    - `pnpm lint` (Style)
    - `pnpm test run` (Logic)
    - `pnpm build` (Bundle Integrity)

4.  **🌐 END-TO-END CHECK (Optional but Recommended)**
    - **Action:** Build binary `npm run tauri build` (if not built).
    - **Action:** Execute `npm run test:e2e` to verify WebDriver integration flows.
    - **Tool:** Leverage `playwright` or `puppeteer` MCP server for visual/DOM E2E validation if deep UI bugs are suspected.

5.  **✅ FINAL GATE**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/verification-before-completion/SKILL.md`.
    - **Report:** "Ready for Merge/Release."

6.  **🧠 KNOWLEDGE SYNC**
    - **Tool:** Use `mcp_supermemory_addMemory` or `mcp_memory_create_entities` to log any new architectural patterns or major bug resolutions discovered during this cycle.
