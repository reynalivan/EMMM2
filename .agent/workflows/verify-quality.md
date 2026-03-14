---
description: Zero-Tolerance Verification. Enforces Repair Standards and Code Integrity.
---

- **PLAN**:
  - Audit: ./.agent/skills/code-review/SKILL.md focus.
  - Analyze: Use narsil-mcp for security/leaks.
- **ACT**:
  - Backend: `cargo fmt`, `cargo clippy`, `cargo test`.
  - Frontend: `pnpm tsc`, `pnpm build`, `pnpm test`.
  - E2E: `npm run test:e2e` (./.agent/skills/e2e-automation/SKILL.md).
- **REFLECT**:
  - Report: Use ./.agent/skills/verification-before-completion/SKILL.md.
