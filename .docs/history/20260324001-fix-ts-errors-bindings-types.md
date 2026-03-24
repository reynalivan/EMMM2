# Fix TypeScript Compilation Errors — Bindings, Types, and Components

## Context

Continued from previous session: `pnpm exec tsc --noEmit` was reporting 40+ errors across the frontend codebase. Goal was to reach zero errors.

## Changes

### Bindings & API (`src/lib/bindings.ts`)

- `readModIni` return type: `string` → `unknown` (actual backend returns IniDocument structure)
- `abortExtractionCmd` param: required → optional (`_params?`) to allow 0-arg calls
- `clearOldThumbnails` return type: `void` → `string` (returns a summary message)
- `suggestRandomMods` return type: `unknown[]` → `Record<string, unknown>[]`
- `checkMetadataUpdate` mock: added missing `updated: false` field
- All commands already fixed in prior session still intact

### Types (`src/types/scanner.ts`, `src/types/settings.ts`)

- `MetadataSyncResult`: added `updated: boolean` and `version?: string` fields (used by UpdateTab)
- `SafeModeConfig`: `enabled?: boolean` added (used by AppStore and PrivacyTab)
- `AppSettings`: added `active_game_id`, `auto_close_launcher` explicit fields

### Frontend Components & Hooks

- **`usePreviewPanelState.ts`**: `onSave` now returns `draft` after `mutateAsync` to satisfy `Promise<MetadataDraftValues>`; `iniDocuments.document` cast to `IniDocumentLike | null | undefined`
- **`RandomizerModal.tsx`**: `suggestRandomMods` result cast via `as unknown as RandomModProposal[]`
- **`useAppStore.ts`**: `safe_mode.enabled` accessed with `?? false` to coerce `boolean | undefined` → `boolean`; fixed duplicate closing brace from prior edit
- **`PrivacyTab.tsx`**: `force_exclusive_mode` coerced via `!!` for `checked` prop

### Tests

- **`dedupService.test.ts`**: `DupScanEvent.event` `'Progress'` → `'progress'` (lowercase)
- **`useDedup.test.ts`**: same `'Progress'` → `'progress'` fix

## Impacted Files

- `src/lib/bindings.ts` (modified)
- `src/types/scanner.ts` (modified)
- `src/types/settings.ts` (modified)
- `src/features/preview/hooks/usePreviewPanelState.ts` (modified)
- `src/features/randomizer/RandomizerModal.tsx` (modified)
- `src/stores/useAppStore.ts` (modified)
- `src/features/settings/tabs/PrivacyTab.tsx` (modified)
- `src/lib/services/dedupService.test.ts` (modified)
- `src/features/scanner/hooks/useDedup.test.ts` (modified)

## Goal

`pnpm exec tsc --noEmit` exits with code 0 and zero errors.

## Impact

- Frontend type-checks cleanly
- No runtime behavioral changes — all fixes are type-level narrowing or mock stubs
- `MetadataSyncResult.updated` and `.version` must be satisfied by backend response; if not, runtime errors may surface in UpdateTab sync flow
