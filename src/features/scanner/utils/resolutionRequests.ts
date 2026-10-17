import type { DupScanGroup, DuplicateSelection, ResolutionRequest } from '@/entities/workspace';
import { canonicalPathKey } from '@/shared/lib/pathKey';

export class DuplicateResolutionConflictError extends Error {
  constructor() {
    super('Duplicate choices would delete a kept folder or delete a folder more than once');
    this.name = 'DuplicateResolutionConflictError';
  }
}

/**
 * The duplicate report lets the user decide per group, but the backend resolves
 * per pair (`keepA` trashes folderB, `ignore` whitelists the pair). This expands
 * one group decision into the pairs the backend expects.
 *
 * - Keep T  → one `keepA` pair (T, other) for every other member, trashing them.
 * - Ignore  → one `ignore` pair for every unordered member combination, so the
 *             whole group stays whitelisted rather than only one edge of it.
 */
export function buildResolutionRequests(
  selections: Map<string, DuplicateSelection>,
  groups: DupScanGroup[],
): ResolutionRequest[] {
  const requests: ResolutionRequest[] = [];

  for (const group of groups) {
    const selection = selections.get(group.groupId);
    if (!selection) {
      continue;
    }

    const members = group.members.map((member) => member.folderPath);

    if (selection.type === 'Keep') {
      const kept = selection.targetPath;
      if (!members.includes(kept)) {
        continue;
      }

      for (const member of members) {
        if (member === kept) {
          continue;
        }

        requests.push({
          groupId: group.groupId,
          action: 'keepA',
          folderA: kept,
          folderB: member,
        });
      }
      continue;
    }

    for (let i = 0; i < members.length; i += 1) {
      for (let j = i + 1; j < members.length; j += 1) {
        requests.push({
          groupId: group.groupId,
          action: 'ignore',
          folderA: members[i],
          folderB: members[j],
        });
      }
    }
  }

  const keepers = new Set<string>();
  const losers = new Set<string>();
  for (const request of requests) {
    if (request.action !== 'keepA') continue;
    const keeper = canonicalPathKey(request.folderA) ?? request.folderA;
    const loser = canonicalPathKey(request.folderB) ?? request.folderB;
    if (losers.has(loser)) throw new DuplicateResolutionConflictError();
    keepers.add(keeper);
    losers.add(loser);
  }
  if ([...losers].some((loser) => keepers.has(loser))) {
    throw new DuplicateResolutionConflictError();
  }
  return requests;
}
