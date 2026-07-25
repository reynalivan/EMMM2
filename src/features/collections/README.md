# Collections Feature

This feature module manages Mod Collections, Corridor state, Safe Mode, and PIN security.

## Feature Boundaries & Architectural Overlap

The `collections` feature folder contains UI components and hooks for four distinct backend systems that are tightly cohesive in the user experience:

1. **Collections**: Named snapshots of mod states (`CollectionSummary`, `CollectionPreview`).
2. **Corridor**: The projected virtual filesystem state representing what the game engine actually sees (`CorridorSnapshot`).
3. **Safe Mode**: The mechanism to switch the Corridor between the "SAFE" (all disabled) and "UNSAFE" (mods active) states.
4. **PIN Security**: The authentication gate required to switch out of Safe Mode.

### Why are they together?
While the backend separates these concerns into `collection_service`, `disk_reconcile`, and `pin_service`, the frontend UI heavily intertwines them. For example:
- The `CollectionsPage` displays the current `Corridor` state as the first row.
- `useSafeModeToggle` orchestrates checking the `usePin` status before initiating a `useCorridorSwitch`.
- Applying a Collection requires knowing the current Corridor state to compute the correct differential.

### Export Barrel
Consumers outside of this feature (e.g., `ContextControls`, `PrivacyTab`) should import hooks from `src/features/collections/hooks/index.ts` to avoid reaching deep into internal files.
