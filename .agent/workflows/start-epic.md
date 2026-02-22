---
description: Start a new Epic. Standardizes TDD planning, Gap Analysis, and Rule-Based Scaffolding.
---

1.  **🔍 CONTEXT & DISCOVERY**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/ask-questions-if-underspecified/SKILL.md` (Clarify Req).
    -   **Action:** Read `e:/Dev/EMMM2NEW/.docs/trd.md` (Scope).
    -   **Action:** Read `e:/Dev/EMMM2NEW/.agent/rules/project_context.md` (Arch).
    -   **Decision:** Split into Sub-Features if > 1 day work.

2.  **📋 PLANNING & ARCHITECTURE**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/writing-plans/SKILL.md` (Create Implementation Plan).
    -   **Tech Check:**
        -   Backend? Use `e:/Dev/EMMM2NEW/.agent/skills/backend-development/SKILL.md`.
        -   Commands? Use `e:/Dev/EMMM2NEW/.agent/skills/tauri-command/SKILL.md`.
        -   Files? Use `e:/Dev/EMMM2NEW/.agent/skills/atomic-fs/SKILL.md`.
        -   Complex Match? Use `e:/Dev/EMMM2NEW/.agent/skills/deep-matcher/SKILL.md`.

3.  **🧪 TEST CASE DESIGN (The Blueprint)**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/writing-unit-tests/SKILL.md`.
    -   **Action:** Audit `.docs/.testcase/`.
    -   **Mandate:** Ensure 100% coverage (Functional, Negative, Edge, Performance).

4.  **🏗️ ENVIRONMENT PREP**
    -   **Action:** Create `tests/fixtures/[Feature]/` for mock data.

5.  **🛑 USER REVIEW**
    -   **Stop:** Notify User. Do not code until Plan & TCs are approved.

6.  **🔄 EXECUTION (TDD LOOP)**
    -   **Strict Rule:** NO PRODUCTION CODE without a failing test.
    -   **Workflow:** For each Test Case in `.testcase/TC-Epic-*.md`:
        1.  **Execute:** `/tdd-cycle` (`e:/Dev/EMMM2NEW/.agent/workflows/tdd-cycle.md`).
        2.  **Reference:** `e:/Dev/EMMM2NEW/.agent/rules/tdd_workflow.md` (Iron Law).
        3.  **Reference:** `e:/Dev/EMMM2NEW/.agent/skills/tdd/references/rust_tdd.md` (Backend).
        4.  **Reference:** `e:/Dev/EMMM2NEW/.agent/skills/tdd/references/react_tdd.md` (Frontend).

7.  **✅ FINAL VERIFICATION**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/verification-before-completion/SKILL.md`.
    -   **Action:** Run full suite before claiming readiness to code.
