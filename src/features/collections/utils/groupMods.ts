import type { CollectionMember } from '../../../types/collection';

export interface GroupedMod {
  id: string;
  name: string;
  type: string;
  mods: CollectionMember[];
  unsafeCount: number;
  is_enabled?: boolean;
}

interface BuildGroupedCollectionMembersOptions {
  mode: 'workspace' | 'preview';
  relevantObjectIds?: ReadonlySet<string>;
}

/**
 * Groups a flat list of V2 CollectionMembers into Object-based Groups.
 * Objects are extracted from `kind === 'object'` members.
 * Mods (`kind === 'mod' | 'nested'`) are sorted into these object groups based on their `object_id`.
 */
export function buildGroupedCollectionMembers(
  members: CollectionMember[],
  options?: BuildGroupedCollectionMembersOptions,
): GroupedMod[] {
  const groupsMap = new Map<string, GroupedMod>();
  const uncategorizedMods: CollectionMember[] = [];
  const uncategorizedUnsafeCount = 0;

  // 1. Initial pass: Create groups for all Object states
  members.forEach((member) => {
    if (member.kind === 'object') {
      const objId = member.object_id || member.path_key;
      groupsMap.set(objId, {
        id: objId,
        name: member.display_name || 'Unknown Object',
        type: 'Object', // In V2, we don't have object_type out of the box unless we do a DB join, just fallback
        mods: [],
        unsafeCount: 0,
        is_enabled: member.is_enabled,
      });
    }
  });

  // 2. Second pass: Assign mods to their exact Group, or create impromptu groups/uncategorized
  members.forEach((member) => {
    if (member.kind === 'mod' || member.kind === 'nested') {
      if (member.object_id) {
        if (!groupsMap.has(member.object_id)) {
          // Object state is missing from collection, but mod claims to belong to it
          groupsMap.set(member.object_id, {
            id: member.object_id,
            name: member.display_name || 'Unknown Object',
            type: 'Other',
            mods: [],
            unsafeCount: 0,
          });
        }
        const group = groupsMap.get(member.object_id)!;
        group.mods.push(member);
        // We lack `is_safe` in V2 CollectionMember easily accessible on frontend without cross-refing
        // We'll leave unsafeCount at 0 for now since SafeMode logic operates generically.
      } else {
        uncategorizedMods.push(member);
      }
    }
  });

  const mergedGroups = Array.from(groupsMap.values());

  if (uncategorizedMods.length > 0) {
    mergedGroups.push({
      id: 'uncategorized',
      name: 'Uncategorized',
      type: 'Other',
      mods: uncategorizedMods,
      unsafeCount: uncategorizedUnsafeCount,
      is_enabled: true,
    });
  }

  const relevantObjectIds = options?.relevantObjectIds;
  const filteredGroups =
    options?.mode === 'preview' && relevantObjectIds && relevantObjectIds.size > 0
      ? mergedGroups.filter(
          (group) =>
            relevantObjectIds.has(group.id) ||
            relevantObjectIds.has(group.name) ||
            group.id === 'uncategorized',
        )
      : mergedGroups;

  filteredGroups.sort((left, right) => {
    if (left.id === 'uncategorized') return 1;
    if (right.id === 'uncategorized') return -1;
    return left.name.localeCompare(right.name);
  });

  return filteredGroups;
}
