# Fix Mods Manager Infinite Loading

## Context
The user reported an infinite loading loop in the Mods Manager (`FolderGrid`) after navigating from the dashboard. The backend was continuously scanning and listing mod folders (`[INFO] Listed 6 mod folders...`).

## Changes
- Identified that `useObjHandlersScan.ts`'s `handleBackgroundSync` was unstable because it depended on `isSyncing` state, recreating its reference on every start/stop.
- Replaced the `isSyncing` dependency guard with a stable `useRef(false)` (`isSyncingRef`), maintaining the same early-return protection without causing cascading reference changes.
- Fixed an `useEffect` in `ObjectList.tsx` that accidentally triggered on every `activeGame` object reference change (which rebuilt on every settings fetch), tying the dependency correctly to `activeGame?.id`.

## Impacted Files
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/object-list/ObjectList.tsx` (modified)

## Goal
To stabilize the Mod Manager's background directory indexing to run exactly once when the active game changes, preventing React Query invalidation loops.

## Impact
- Stops the infinite fetching loop of React Query and Tauri backend queries.
- Prevents UI unresponsiveness and rapid rerenders when navigating to the Explorer/Mods Manager.
- No breaking changes; purely an architectural React stabilization.
