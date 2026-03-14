---
description: Start a major Feature/Requirement (Req). Standardizes TDD planning, Gap Analysis, and Rule-Based Scaffolding.
---

- **PLAN**:
  - Discovery: Clarify Req via trd.md.
  - Blueprint: Ensure Test Cases exist (.docs/.testcase/tc-*.md).
  - Design: Create Plan (./.agent/skills/writing-plans/SKILL.md).
- **ACT**:
  - Prep: Create tests/fixtures/ for mock data.
  - Loop: Execute /tdd-cycle for each TC. NO PROD CODE WITHOUT FAILING TEST.
- **REFLECT**:
  - Verify: Execute /verify-quality full suite.
  - Sync: Update task.md and walkthrough.md.
