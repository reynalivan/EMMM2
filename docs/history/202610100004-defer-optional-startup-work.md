# Defer optional startup work

## Context

User approved priorities 3 and 5: scan Home keybindings in background with a
widget skeleton, and keep task-log retention off the startup critical path.

## Changes

- Wait for selected-game activation Ready before optional keybinding reads;
  scope cache/results by generation and hide stale responses during switches.
- Show a reduced-motion-compatible skeleton in the keybinding widget only;
  errors/unavailable sources settle visibly. Demo uses fixture reads without IPC.
- Move settled task retention into the existing background startup worker after
  active baseline settlement; preserve old open tasks and newer settled rows.
- Extend native restart E2E with visible F1 and post-baseline cleanup assertions.

## Impacted Files

- Dashboard: `src/pages/dashboard/hooks/useActiveKeybindings.ts`,
  `useActiveKeybindings.test.ts`, `useActiveKeybindings.demo.test.ts`;
  `src/pages/dashboard/{Dashboard.tsx,Dashboard.test.tsx}`;
  `src/pages/dashboard/components/DashboardActivity.tsx`.
- Locales: `src/shared/i18n/locales/{en,id,zh}/dashboard.json`.
- Backend: `src-tauri/src/modules/system/application/app/bootstrap.rs`;
  `src-tauri/src/modules/workspace/adapters/sqlite/task/testing.rs`.
- E2E: `tests/e2e/specs/startup-reopen.e2e.ts`.
- Documentation: `src-tauri/AGENT.md`,
  `docs/plans/startup-performance/{implementation_plan,validation}.md`, this file.

## Goal and Impact

Home remains usable while optional keybindings load. Cleanup cannot precede the
selected baseline or remove recovery tasks by age. No daemon, new dependency,
IPC/schema change or installed-app replacement. User's native-only E2E direction
was applied; the brief demo browser development server was stopped.

## Validation

34 frontend tests and migrated-schema task retention passed. Final native E2E
passed three restarts with visible keybinding and cleanup-order checks. Build,
strict Clippy and scoped lint checks passed; review's demo-loading regression was
fixed. The prior unrelated duplicate-whitelist full-suite failure remains noted
in `validation.md`; no end-to-end performance percentage claimed.
