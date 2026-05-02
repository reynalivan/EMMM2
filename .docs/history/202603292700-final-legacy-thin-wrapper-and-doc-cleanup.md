# Final legacy, thin wrapper, and doc cleanup

## Context

The runtime `mods` architecture was already stabilized, but active docs, a preview wrapper, and a few exported helpers still reflected the pre-`WorkspaceViewModel` shape.

## Changes

- Removed the dead preview selection helper and dead `DbEntryFull` alias.
- Inlined the one-line preview action wrapper into `PreviewPanel`.
- Removed the unused `SelectionRewriteEffect` export from workspace runtime state.
- Updated active docs and comments so they describe `WorkspaceViewModel`, shared runtime actions, and descriptor-driven refresh flow instead of old query/wrapper helpers.
- Added audit coverage to prevent the removed preview wrapper/helper and stale docs references from coming back.

## Impacted Files

- `.docs/flow.md` (modified)
- `.docs/requirements/req-06-objectlist-navigation.md` (modified)
- `.docs/requirements/req-08-smart-filters.md` (modified)
- `.docs/requirements/req-20-mod-toggle.md` (modified)
- `.docs/requirements/req-39-folder-collision.md` (modified)
- `src/features/collections/hooks/useCorridor.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/features/preview/PreviewPanel.tsx` (modified)
- `src/features/preview/hooks/usePreviewData.ts` (modified)
- `src/features/preview/hooks/usePreviewData.test.ts` (modified)
- `src/features/preview/hooks/usePreviewPanelActions.ts` (removed)
- `src/features/workspace-runtime/state/workspaceState.ts` (modified)
- `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (modified)
- `src/types/object.ts` (modified)

## Goal

Keep active documentation, exported types, and runtime-facing preview code aligned with the finalized workspace runtime architecture.

## Impact

- No runtime behavior change intended beyond removing an unnecessary preview wrapper surface.
- Repo audit now guards against reintroducing removed helpers and stale architecture references in active docs.
- One unrelated existing test failure remains in `EditObjectModal.test.tsx` around localized validation text expectations.

## Notes

- Historical notes in `.docs/history/*` were left untouched by design.
