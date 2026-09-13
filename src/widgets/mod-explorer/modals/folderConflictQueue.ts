import type { FolderNameConflictGroup } from '../../../shared/api/tauri/bindings';

export interface CompletedFolderConflict {
  group_id: string;
  fingerprint: string;
  display_name: string;
}

export function folderConflictFingerprint(group: FolderNameConflictGroup): string {
  const candidates = group.candidates
    .map((candidate) => `${candidate.path}:${candidate.is_enabled ? 1 : 0}`)
    .sort()
    .join(',');
  return `${group.group_id}:${candidates}`;
}

export function reconcileFolderConflictQueue(
  previous: FolderNameConflictGroup[],
  current: FolderNameConflictGroup[],
  completed: CompletedFolderConflict[],
  resolvedGroupId: string | null = null,
): CompletedFolderConflict[] {
  if (previous.length === 0 && current.length > 0) {
    return [];
  }

  const currentFingerprints = new Set(current.map(folderConflictFingerprint));
  const currentIds = new Set(current.map((group) => group.group_id));
  const next = completed.filter(
    (entry) => !currentIds.has(entry.group_id) && !currentFingerprints.has(entry.fingerprint),
  );
  const completedFingerprints = new Set(next.map((entry) => entry.fingerprint));
  const addCompletedGroup = (group: FolderNameConflictGroup) => {
    const fingerprint = folderConflictFingerprint(group);
    if (
      currentIds.has(group.group_id) ||
      currentFingerprints.has(fingerprint) ||
      completedFingerprints.has(fingerprint)
    ) {
      return;
    }

    next.push({
      group_id: group.group_id,
      fingerprint,
      display_name: group.display_name,
    });
    completedFingerprints.add(fingerprint);
  };

  if (resolvedGroupId) {
    const resolvedGroup = previous.find((group) => group.group_id === resolvedGroupId);
    if (resolvedGroup) {
      addCompletedGroup(resolvedGroup);
    }
  }

  // A changed disk report is evidence of current filesystem state, not proof
  // that the user completed a resolution in this dialog. Only an explicit
  // successful action may mark a group complete.
  return next;
}

export function selectNextFolderConflictGroup(
  previous: FolderNameConflictGroup[],
  current: FolderNameConflictGroup[],
  selectedId: string | null,
): string | null {
  if (current.length === 0) {
    return null;
  }
  if (selectedId && current.some((group) => group.group_id === selectedId)) {
    return selectedId;
  }
  const previousIndex = previous.findIndex((group) => group.group_id === selectedId);
  if (previousIndex >= 0) {
    for (let index = previousIndex + 1; index < previous.length; index += 1) {
      const nextId = previous[index].group_id;
      if (current.some((group) => group.group_id === nextId)) {
        return nextId;
      }
    }
  }
  return current[0].group_id;
}
