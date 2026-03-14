import type { CollectionPreviewMod } from '../../../types/collection';

export interface GroupedMod {
  id: string;
  name: string;
  type: string;
  mods: CollectionPreviewMod[];
  unsafeCount: number;
}

/**
 * Groups a flat list of CollectionPreviewMod by their parent object.
 * Used by CollectionWorkspace, ApplyCollectionModal, and any future views.
 */
export function groupMods(mods: CollectionPreviewMod[]): GroupedMod[] {
  const objectsMap = new Map<string, GroupedMod>();
  let hasUncategorized = false;
  const uncategorizedMods: CollectionPreviewMod[] = [];
  let uncategorizedUnsafeCount = 0;

  mods.forEach((mod) => {
    if (mod.object_name) {
      const groupKey = mod.object_name;

      if (!objectsMap.has(groupKey)) {
        objectsMap.set(groupKey, {
          id: mod.object_id || groupKey,
          name: mod.object_name,
          type: mod.object_type || 'Other',
          mods: [],
          unsafeCount: 0,
        });
      }

      const obj = objectsMap.get(groupKey)!;
      // Upgrade type/id if a later mod provides richer info
      if (obj.type === 'Other' && mod.object_type) {
        obj.type = mod.object_type;
      }
      if (obj.id === groupKey && mod.object_id) {
        obj.id = mod.object_id;
      }

      obj.mods.push(mod);
      if (!mod.is_safe) obj.unsafeCount += 1;
    } else {
      hasUncategorized = true;
      uncategorizedMods.push(mod);
      if (!mod.is_safe) uncategorizedUnsafeCount += 1;
    }
  });

  const groups = Array.from(objectsMap.values());
  if (hasUncategorized) {
    groups.push({
      id: 'uncategorized',
      name: 'Uncategorized',
      type: 'Other',
      mods: uncategorizedMods,
      unsafeCount: uncategorizedUnsafeCount,
    });
  }

  const typeOrder = ['Character', 'Weapon', 'UI', 'Other'];
  groups.sort((a, b) => {
    const idxA = typeOrder.indexOf(a.type);
    const idxB = typeOrder.indexOf(b.type);
    if (idxA !== -1 && idxB !== -1 && idxA !== idxB) return idxA - idxB;
    if (idxA !== -1 && idxB === -1) return -1;
    if (idxA === -1 && idxB !== -1) return 1;
    return a.name.localeCompare(b.name);
  });

  return groups;
}
