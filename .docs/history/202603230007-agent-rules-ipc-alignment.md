# Agent Rules & Skills Aligned to Typed IPC

### Context

Agent documentation still referenced legacy `invoke` patterns after the codebase achieved 100% typed IPC.

### Changes

- Added explicit IPC Safety rule prohibiting raw `invoke()` in `code_standards.md`
- Updated `project_core.md` bridge hygiene to mandate Specta `bindings.ts`
- Replaced all mock patterns from `vi.mock('@tauri-apps/api/core')` to `vi.mock('../../lib/bindings')` in TDD/testing guides
- Updated Rust doc comment examples from `invoke('cmd')` to `commands.cmd(...)` with `#[specta::specta]`
- Updated E2E skill to reference typed IPC bridge

### Impacted Files

- `.agent/rules/code_standards.md` (modified)
- `.agent/rules/data_logic.md` (modified)
- `.agent/rules/dev_ops.md` (modified)
- `.agent/rules/project_core.md` (modified)
- `.agent/skills/code-documentation/references/rust_docs.md` (modified)
- `.agent/skills/tdd/references/react_tdd.md` (modified)
- `.agent/skills/writing-unit-tests/references/react_frontend.md` (modified)
- `.agent/skills/e2e-automation/SKILL.md` (modified)

### Goal

Ensure all agent guidance consistently directs toward the typed `commands` API, preventing regression to legacy patterns.

### Impact

- Future agent sessions will always generate typed IPC code
- Mock patterns in test scaffolding are now correct
- No breaking changes; documentation-only update
