import type { ConflictInfo } from '@/entities/workspace';

export type ModDecision = 'keep' | 'disable';
export type ConflictDecisions = ReadonlyMap<string, ModDecision>;

export interface ConflictResolutionSummary {
  disablePaths: string[];
  resolvedCount: number;
  unresolvedCount: number;
}

export interface ConflictModSet {
  key: string;
  modPaths: string[];
  conflicts: ConflictInfo[];
}

function sortPaths(paths: string[]): string[] {
  return [...new Set(paths)].sort((left, right) => left.localeCompare(right));
}

/**
 * The runtime detector reports one item per resource or shader hash. The user
 * acts on mod folders, so combine hashes that involve the same mod locations.
 */
export function groupConflictsByModSet(conflicts: ConflictInfo[]): ConflictModSet[] {
  const groups = new Map<string, ConflictModSet>();

  for (const conflict of conflicts) {
    const modPaths = sortPaths(conflict.mod_paths);
    const key = JSON.stringify(modPaths);
    const existing = groups.get(key);
    if (existing) {
      existing.conflicts.push(conflict);
    } else {
      groups.set(key, { key, modPaths, conflicts: [conflict] });
    }
  }

  return [...groups.values()]
    .map((group) => ({
      ...group,
      conflicts: [...group.conflicts].sort(
        (left, right) => left.kind.localeCompare(right.kind) || left.hash.localeCompare(right.hash),
      ),
    }))
    .sort((left, right) => left.key.localeCompare(right.key));
}

export function buildConflictKey(conflict: ConflictInfo): string {
  const stages = conflict.evidence
    .map((evidence) => evidence.shader_stage)
    .filter((stage): stage is string => stage !== null)
    .sort();
  return [
    conflict.kind,
    conflict.hash,
    stages.join(','),
    [...conflict.mod_paths].sort().join(','),
  ].join('|');
}

export function setModDecision(
  decisions: ConflictDecisions,
  modPath: string,
  decision: ModDecision,
): Map<string, ModDecision> {
  if (!modPath.trim()) {
    throw new Error('A mod path is required');
  }
  const next = new Map(decisions);
  next.set(modPath, decision);
  return next;
}

function chooseWinnerFromPaths(
  decisions: ConflictDecisions,
  modPaths: string[],
  keepPath: string,
): Map<string, ModDecision> {
  if (!modPaths.includes(keepPath)) {
    throw new Error('The selected winner is not part of this conflict');
  }

  const next = new Map(decisions);
  for (const path of modPaths) {
    next.set(path, path === keepPath ? 'keep' : 'disable');
  }
  return next;
}

export function chooseConflictWinner(
  decisions: ConflictDecisions,
  conflict: ConflictInfo,
  keepPath: string,
): Map<string, ModDecision> {
  return chooseWinnerFromPaths(decisions, conflict.mod_paths, keepPath);
}

export function chooseConflictModSetWinner(
  decisions: ConflictDecisions,
  conflictSet: ConflictModSet,
  keepPath: string,
): Map<string, ModDecision> {
  return chooseWinnerFromPaths(decisions, conflictSet.modPaths, keepPath);
}

export function summarizeConflictResolution(
  conflicts: ConflictInfo[],
  decisions: ConflictDecisions,
): ConflictResolutionSummary {
  const activePaths = new Set(conflicts.flatMap((conflict) => conflict.mod_paths));
  const disablePaths = [...decisions]
    .filter(([path, decision]) => activePaths.has(path) && decision === 'disable')
    .map(([path]) => path)
    .sort();
  const disabled = new Set(disablePaths);
  const conflictSets = groupConflictsByModSet(conflicts);
  const resolvedCount = conflictSets.filter(
    (conflictSet) => conflictSet.modPaths.filter((path) => !disabled.has(path)).length <= 1,
  ).length;

  return {
    disablePaths,
    resolvedCount,
    unresolvedCount: conflictSets.length - resolvedCount,
  };
}
