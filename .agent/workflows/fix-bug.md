---
description: Systematic Debugging. Enforces Reproduction and Root Cause Analysis.
---

1.  **🔍 UNDERSTAND**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/ask-questions-if-underspecified/SKILL.md`.
    -   **Action:** Gather logs, screenshots, and steps.

2.  **🧪 REPRODUCTION (The Gatekeeper)**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/writing-unit-tests/SKILL.md`.
    -   **Action:** Create `repro_bug_[ID].test.tsx`.
    -   **Rule:** If it doesn't fail, it's not reproduced. STOP.

3.  **🕵️ ANALYSIS & FIX**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/atomic-fs/SKILL.md` (If File Ops involved).
    -   **Action:** Analyze logs/trace.
    -   **Research:** `search_web` for obscure errors.

4.  **🛠️ IMPLEMENTATION**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/code-refactoring/SKILL.md` (Clean Fix).
    -   **Constraint:** Minimal atomic change.

5.  **✅ VERIFICATION**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/verification-before-completion/SKILL.md`.
    -   **Action:** Run Repro Test + Regression Suite.

6.  **💾 COMMIT**
    -   `fix(<scope>): <description> (Closes #ID)`
