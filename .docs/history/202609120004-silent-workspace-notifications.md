# Silent workspace notifications

## Context

Routine Workspace Switch actions showed both a reload-key toast and a selection
reconciliation toast, even though the UI state had already been updated.

## Changes

- Removed the applied-switch reload-key toast and its unused translations.
- Removed the automatic selection-reconciliation toast; the reconciled state is
  still dispatched to the workspace store.
- Kept dialogs for duplicate resolution, folder conflicts, rename confirmation,
  and files in use. Error toasts remain for failures that do not have a dialog.
- Added a regression assertion that an applied workspace switch stays silent.

## Validation

- `corepack pnpm exec vitest run src/features/workspace-runtime/actions/workspaceSwitchOps.test.ts src/features/workspace-runtime/hooks/useWorkspaceViewModel.contract.test.ts` — 25 passed
- `corepack pnpm exec prettier --check` on changed files
- `corepack pnpm build`
- `corepack pnpm tauri build --bundles nsis`
- `corepack pnpm i18n:lint` remains blocked by three existing hardcoded strings:
  `Ctrl+A`, `Ctrl+Shift+A`, and `% ·` in Match Wizard files not changed here.
