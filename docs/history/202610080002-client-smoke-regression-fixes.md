# Fix client smoke regressions

## Context

Isolated client QA reproduced stale settings revisions, lost selection, editor input races and unhandled rename conflict feedback.

## Changes

- Publish authoritative activation/settings snapshots monotonically; cancel superseded reads.
- Validate duplicate physical roots in the form resolver; restore native trust-dialog Escape/focus and editable browser addresses.
- Preserve explicit selection on refresh; apply dirty-editor transitions atomically and rewrite queued paths with disk renames.
- Keep grid shortcuts out of text controls; merge metadata refresh/ACKs without replacing newer edits.
- Consume already-reported rename failures at the UI boundary, keeping success effects separate.
- Keep opposite bulk-toggle intents across path rewrites only when the complete selected filesystem identities match; retain context and all-matching snapshot guards.
- Bridge completed-toggle/listing lag with one transient committed receipt and the existing identity-validated command; retire it when fresh candidates or changed context supersede it.
- Map receipt paths with existing rewrite semantics and physical IDs, covering mixed UI/backend path formats without basename matching.

## Impacted Files

Modified unless marked added; grouped braces enumerate filenames.

- `src-tauri/AGENT.md`, `src-tauri/src/modules/reconciliation/application/disk_reconcile/types.rs`, `src-tauri/src/modules/settings/adapters/tauri/settings_cmds.rs`
- `src/shared/api/tauri/bindings.gen.ts` (regenerated), `src/shared/lib/isEditableKeyboardTarget.ts` (added)
- `src/app/store/appStore/{gameSlice,workspaceRuntimeSlice}.ts`, `src/app/store/useAppStore.test.ts`
- `src/entities/settings/index.ts`, `src/entities/settings/api/{settingsQuery,useSettings}.ts`, `{settingsQuery.test.ts,useSettings.regression.test.tsx}` (added)
- `src/features/workspace-runtime/state/{workspaceEvents,workspaceState,workspaceRuntimeReducer,workspaceRuntimeTransitions,workspaceStoreBridge,workspaceStoreSelectors,workspaceReducer.test}.ts`
- `src/features/mod-runtime/actions/{useSharedModActions,useSharedModActions.test}.ts`
- `src/pages/settings/{modals/GameFormModal.tsx,modals/GameFormModal.test.tsx,components/TrustInformationDialog.tsx,hooks/useSettings.test.ts}`, `components/TrustInformationDialog.test.tsx` (added)
- `src/pages/browser/components/{BrowserToolbar,BrowserToolbar.test}.tsx`
- `src/widgets/mod-explorer/{FolderGrid,FolderGrid.test}.tsx`, `hooks/{useFolderGrid,useFolderGrid.test,useFolderGridBulk,useFolderGridBulk.test,useFolderGridSelection,useFolderNavigation,useFolderNavigation.test,useWorkspaceExplorerSelection}.ts`, `hooks/useWorkspaceExplorerSelection.test.tsx`
- `src/widgets/mod-preview/hooks/useMetadataDraft.ts`, `useMetadataDraft.test.ts` (added)
- `tests/e2e/specs/{client-browser-smoke,client-workspace-smoke}.e2e.ts` (audit files updated)
- `docs/plans/client-smoke-20261007/{implementation_plan,fix-results}.md`, `fix-native-evidence.json` (added), this history (added)

## Goal

Keep validated disk operations responsive while refreshes, pending navigation and asynchronous saves preserve the user's latest intent.

## Impact

Activation acknowledgement adds the settings snapshot to generated IPC bindings. No extra hydration IPC, dependency, queue or schema migration. Optional runtime/indexing work is not added to the switch critical path.

## Validation

1,203 frontend tests passed (1 skipped); native Settings 44/44, workspace 20/20 and direct Browser input 1/1 passed. Types, lint, architecture, builds and diff checks passed. Details/boundaries: `docs/plans/client-smoke-20261007/fix-results.md`. Concurrent Mod Inbox changes are preserved, not attributed to this fix.
