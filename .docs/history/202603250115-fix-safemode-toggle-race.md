# Fix Safe Mode Toggle Blank Screen (Race Condition)

## Context
Clicking the guard icon in the topbar resulted in a blank page or the confirm/PIN modal not appearing. Root cause: `useSafeModeToggle` was called in **three places simultaneously** (ContextControls, GlobalActions, CollectionsPage), each mounting its own React `useState` `flowState`. When one instance called `setFlowState({ kind: 'confirm' })`, the other two instances remained `idle` and their modals stayed closed. Any re-render would reset to whichever instance had the latest state, causing flickering and blanks.

## Changes
- **`src/stores/useAppStore.ts`**: Added `safeModeFlow: SafeModeFlowState` and `setSafeModeFlow` to the Zustand store. All instances now read/write from one centralized state.
- **`src/features/collections/hooks/useSafeModeToggle.ts`**: Replaced `useState<FlowState>` with `useAppStore().safeModeFlow / setSafeModeFlow`. Removed local `FlowState` type (now lives in `useAppStore.ts` as `SafeModeFlowState`). Updated all callbacks and derived values accordingly.

## Impacted Files
- `src/stores/useAppStore.ts` (modified)
- `src/features/collections/hooks/useSafeModeToggle.ts` (modified)

## Goal
Safe mode toggle now reliably shows the confirm/PIN modal regardless of which UI element triggers it (topbar guard icon, topbar dropdown, or collections page tabs).

## Impact
- No behavioral change for single-instance usage — purely fixes the multi-instance race.
- `safeModeFlow` is NOT persisted in LocalStorage (it's ephemeral UI state, intentionally not in the `partialize` list).
