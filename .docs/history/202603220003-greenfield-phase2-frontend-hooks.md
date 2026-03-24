# Greenfield Phase 2 (Steps 2.1–2.4): Frontend Types, QueryKeys, and Hooks

## Context

Phase 2 of the greenfield redesign. Creates v2 TypeScript types, query key factories, and 16 domain hooks (replacing 9+ legacy hooks + scattered utility functions).

## Changes

- **v2 Types** (`types/v2.ts`): 9 interfaces mirroring Rust domain structs (V2CorridorSnapshot, V2SwitchResult, V2CollectionSummary, V2CollectionMember, V2CollectionPreview, V2ApplyResult, V2PinStatus) + 2 type aliases
- **v2 QueryKeys** (`v2QueryKeys.ts`): 3 factories (corridor, collection, pin) with `v2-` prefix
- **useCorridor**: Replaces useCorridorRuntimeSnapshot + useWorkspaceContext + resolveActiveCollection chain
- **useCorridorSwitch**: Replaces useAppStore.setSafeMode → invoke → invalidate chain
- **useV2Collections**: 6 hooks (list, preview, create, update, delete, apply, undo) replacing 9 legacy hooks
- **usePin**: 5 hooks (hasPin, status, verify, set, clear) replacing scattered PIN logic

## Impacted Files

- `src/types/v2.ts` (added)
- `src/features/collections/v2QueryKeys.ts` (added)
- `src/features/collections/hooks/v2/useCorridor.ts` (added)
- `src/features/collections/hooks/v2/useCorridorSwitch.ts` (added)
- `src/features/collections/hooks/v2/useV2Collections.ts` (added)
- `src/features/collections/hooks/v2/usePin.ts` (added)
- `src/features/collections/hooks/v2/index.ts` (added)

## Goal

Complete frontend query/mutation layer for v2 backend commands, enabling subsequent component refactoring (steps 2.5–2.8).

## Impact

- No breaking changes — v2 hooks coexist with legacy hooks
- `npx tsc --noEmit` passes
- 16 v2 hooks replace 9+ legacy hooks + 7 utility files
