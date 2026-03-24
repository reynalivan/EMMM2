---
description: Systematic Debugging. Enforces Reproduction and Root Cause Analysis.
---

- **PLAN**:
  - Understand: Gather logs/steps (./.agent/skills/ask-questions-if-underspecified/SKILL.md).
  - RCA Gate: Identify Root Cause. Stop if reproduction fails.
  - Impact: Check AGENT.md axioms (Filesystem Truth, Atomic Ops).
- **ACT**:
  - Fix: Minimal atomic change (./.agent/skills/code-refactoring/SKILL.md).
  - TDD: Execute /tdd-cycle for logic changes.
- **REFLECT**:
  - Verify: Execute /verify-quality.
  - Commit: `fix(<scope>): <description> (Closes #ID)`
