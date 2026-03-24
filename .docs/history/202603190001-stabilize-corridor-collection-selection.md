# Stabilize Corridor Collection Selection

## Context

- Active collection in the topbar could fall back too easily during init/apply refresh.
- Collection row selection was local to the Collections page, so it did not stick across topbar actions, navigation, or restart.

## Changes

- Separated strict active corridor state from persisted UI selection per corridor.
- Persisted selected collection by `game_id + safe/unsafe` in app store and reused it in Collections page, save flows, and apply flows.
- Tightened topbar loading behavior to avoid falling back while the current corridor overview query is still pending.
- Extracted a shared backend helper for strict current-vs-target collection signature parity and added an apply-follow-up regression test.
- Fixed workspace draft toggle propagation to avoid parent state updates during child render.

## Impacted Files

- `src/lib/corridorSelection.ts` (added)
- `src/stores/useAppStore.ts` (modified)
- `src/stores/useAppStore.test.ts` (modified)
- `src/features/collections/CollectionsPage.tsx` (modified)
- `src/features/collections/CollectionsPage.test.tsx` (modified)
- `src/features/collections/components/CollectionWorkspace.tsx` (modified)
- `src/features/collections/components/SaveCollectionModal.tsx` (modified)
- `src/features/collections/components/ApplyCollectionModal.tsx` (modified)
- `src/features/collections/components/ApplyCollectionModal.test.tsx` (modified)
- `src/components/layout/top-bar/ContextControls.tsx` (modified)
- `src-tauri/src/services/collections/effective_state.rs` (modified)
- `src-tauri/tests/collections_service.rs` (modified)

## Goal

- Strict active collection remains backend-authoritative.
- Selected collection now stays attached to the current corridor across page transitions, apply/save actions, and restart.

## Impact

- Collections page and topbar now share the same selected-row state without changing corridor semantics.
- No schema or IPC contract changes.
- Backend strict active overview behavior is covered by an explicit follow-up-read regression after apply.
