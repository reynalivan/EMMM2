---
description: Execute Red-Green-Refactor. Enforces TDD discipline and Code Standards.
---

**Pre-requisite:** `e:/Dev/EMMM2NEW/.agent/skills/tdd/SKILL.md`

1.  **🛑 INPUT VALIDATION**
    - **Check:** Target TC ID known (from `.docs/.testcase/tc-*.md`)?
    - **Action:** STOP if missing.

2.  **🔴 RED (Test First)**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/writing-unit-tests/SKILL.md`.
    - **Action:** Write failing test in `__tests__` or `tests.rs`.
    - **Verify:** Must fail on **Assertion**.

3.  **🟩 GREEN (Implementation)**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/backend-development/SKILL.md` (Rust Patterns).
    - **Constraint:** Write _minimum_ code to pass. YAGNI. Address source of truth (Filesystem vs DB).

4.  **🟦 REFACTOR (Clean Up)**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/code-refactoring/SKILL.md`.
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/code-simplifier/SKILL.md`.
    - **Action:** Run Test. Pass?

5.  **✅ VERIFICATION & COMMIT**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/verification-before-completion/SKILL.md`.
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/code-documentation/SKILL.md` (Add Docs).
    - **Action:** Run `cargo clippy -- -D warnings` to enforce code quality.
    - **Action:** Commit `feat/fix: <TC-ID> description`.

6.  **🔄 LOOP**
    - Proceed to next TC.
