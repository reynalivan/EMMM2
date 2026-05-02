# Remove inline object toggle switch from ObjectList

## Context

ObjectList still rendered an inline enable/disable switch even though object toggle actions are already available through the object context menu. The duplicate control was only needed on ObjectList and was no longer desired.

## Changes

- Removed the inline `WorkspaceSwitchControl` from object rows.
- Kept object status labeling and count display intact.
- Removed dead `ObjectList`/`ObjectListContent` props and handlers that only existed for the inline switcher.

## Impacted Files

- `src/features/object-list/ObjectList.tsx` (modified)
- `src/features/object-list/ObjectListContent.tsx` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `.docs/history/202603291940-objectlist-inline-switcher-removal.md` (added)

## Goal

Object enable/disable in ObjectList is now exposed only through the context menu, reducing duplicate controls without changing object actions elsewhere.

## Impact

- Scope is limited to ObjectList.
- FolderGrid is unchanged.
- No backend or IPC behavior changed.
