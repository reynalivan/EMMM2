---
description: Zero-Tolerance Verification. Enforces Repair Standards and Code Integrity.
---

1.  **🕵️ AUTOMATED AUDIT**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/code-review/SKILL.md`.
    -   **Action:** `grep -rE "@ts-ignore|any|unwrap\(\)|todo!" .`
    -   **Action:** FAIL if found (unless strictly temporary).

2.  **🦀 BACKEND CHECK**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/writing-unit-tests/SKILL.md`.
    -   `cargo fmt -- --check`
    -   `cargo clippy -- -D warnings` (Strict)
    -   `cargo test`

3.  **⚛️ FRONTEND CHECK**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/vercel-react-best-practices/SKILL.md`.
    -   `pnpm tsc --noEmit` (Type Safety)
    -   `pnpm lint` (Style)
    -   `pnpm test run` (Logic)
    -   `pnpm build` (Bundle Integrity)

4.  **✅ FINAL GATE**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/verification-before-completion/SKILL.md`.
    -   **Report:** "Ready for Merge/Release."
