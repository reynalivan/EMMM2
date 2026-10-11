# Fix Settings and conflict contracts

## Context

The user authorized all seven audit findings and three verified cleanup candidates.
Preserve the shared workspace and actual custom dark themes.

## Changes

- Bind homepage saves to game/draft ownership; retain AI drafts across late updates.
- Prevent busy native conflict cancel; move winner coverage to the live mod-set API.
- Retain updater admission/progress beyond tabs with one failure-safe backend job.
- Align new defaults to Onyx; preserve stored/custom dark across discovery failures,
  retained query caches and uppercase JSON filenames. Show retry and actual selection.
- Remove three unused theme commands consistently via actual Specta export and five
  unused DaisyUI aliases; retain active validation/import/persistence helpers.

## Impacted Files

Modified frontend:

- `src/features/conflict-report/{ConflictModal.tsx,ConflictModal.test.tsx,conflictResolution.ts,conflictResolution.test.ts}`
- `src/pages/settings/components/tabs/{AITab.tsx,BrowserTab.tsx,GeneralTab.tsx,GeneralTab.test.tsx}`
- `src/pages/settings/hooks/{useAppUpdater.ts,useAppUpdater.test.ts}`
- `src/pages/settings/components/theme/DynamicThemeInjector.tsx`
- `src/entities/settings/api/{useCustomThemes.ts,useResolvedTheme.ts}`
- `src/shared/lib/themeOptions.ts`; `src/shared/ui/liquid/liquidTheme.ts`
- `src/shared/api/tauri/bindings.gen.ts`

Added tests:

- `src/pages/settings/components/tabs/{AITab.test.tsx,BrowserTab.test.tsx}`
- `src/pages/settings/components/theme/DynamicThemeInjector.test.tsx`
- `src/entities/settings/api/useResolvedTheme.test.tsx`; `src/shared/lib/themeOptions.test.ts`
- `tests/e2e/specs/settings-conflict-audit.e2e.ts`

Modified backend:

- `src-tauri/src/modules/system/adapters/tauri/theme_cmds.rs`
- `src-tauri/src/modules/updates/adapters/tauri/update_cmds.rs`
- `src-tauri/src/modules/settings/application/config/{models.rs,persistence.rs,tests/service_tests.rs}`
- `src-tauri/tests/config_service.rs`; `src-tauri/src/lib.rs`
- `src-tauri/permissions/app-commands.toml`; `src-tauri/AGENT.md`

Added plan/validation: `docs/plans/audit-next-fixes-20261011/{implementation_plan.md,validation.md}`.
Corrected prior audit reachability claim in `docs/plans/audit-next-20261011/report.md`;
this history added. Ignored evidence is in `.audit-output/audit-next-fixes-20261011/`.

## Goal and Impact

Correct intent/state ownership with fewer inactive APIs. No dependency, migration,
worktree, branch, commit, actual updater installation or credential write.

## Verification

89 distinct frontend and 26 targeted Rust tests passed after relevant RED cases.
App/E2E types, scoped lint/architecture, format, registry/export and strict
all-target readonly-audit Clippy passed; independent findings were repaired.
Six native cases passed across four initial cases and two corrected-navigation
reruns. Both fresh builds had 1,416 inputs/zero drift; owned processes, fixtures and
exact theme file cleaned. Busy cancellation and async interleavings are unit scope.
See validation for the initial selector failures and corrected UI reachability.
