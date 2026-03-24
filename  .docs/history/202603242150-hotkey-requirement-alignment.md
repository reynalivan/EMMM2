# Requirement Alignment: Hotkey Variant Cycling

## Context

Initial implementation of Epic 42 (In-Game Hotkeys) prioritized character-based detection and simplified the hotkey set by excluding variant cycling. However, structural restoration of the hotkey system during the Storage Optimizer phase enabled the re-integration of these actions for a more complete user experience.

## Changes

### Requirement Specifications

- **req-42-ingame-hotkeys.md**:
  - Removed variant cycling exclusion from "Non-Goals".
  - Explicitly added Variant Cycle to the supported hotkey list alongside Safe Mode and Preset switching.

## Impacted Files

- `.docs/requirements/req-42-ingame-hotkeys.md` (modified)

## Goal

Establish functional parity between the implemented hotkey subsystem and the project's technical requirements.

## Impact

- **Consistency**: The documentation now accurately reflects the backend's ability to dispatch `NextVariantFolder` and `PrevVariantFolder` actions.
- **Traceability**: Future optimizations to variant selection logic are now officially supported by the requirement spec.
