---
description: Prepare a release. Enforces Changelog generation and Verification.
---

- **PLAN**:
  - Gate: Execute /verify-quality.
  - History: Use ./.agent/skills/changelog-generator/SKILL.md.
- **ACT**:
  - Build: `pnpm tauri build`. Check artifacts.
- **REFLECT**:
  - Release: Update CHANGELOG.md. Notify Nodes.
