# Remove Active Mods Tags in Object List

## Context
User requested to remove the explicit labels/tags of active mods in the object list UI to reduce clutter, relying only on the numeric counts.

## Changes
- Removed obj.active_mod_paths rendering loop (pill tags) from ObjectRowItem metadata section.

## Impacted Files
- src/features/object-list/ObjectRowItem.tsx (modified)

## Goal
The object list now only shows total/enabled counts and standard metadata (weapon, rarity, etc.) without spelling out individual mod names.

## Impact
Cleaner UI, zero functional changes.