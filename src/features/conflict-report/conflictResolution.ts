import type { ConflictInfo } from '@/entities/workspace';

export type ModDecision = 'keep' | 'disable';
export type ConflictDecisions = ReadonlyMap<string, ModDecision>;

export interface ConflictResolutionSummary {
  disablePaths: string[];
  resolvedCount: number;
  unresolvedCount: number;
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

export function chooseConflictWinner(
  decisions: ConflictDecisions,
  conflict: ConflictInfo,
  keepPath: string,
): Map<string, ModDecision> {
  if (!conflict.mod_paths.includes(keepPath)) {
    throw new Error('The selected winner is not part of this conflict');
  }

  const next = new Map(decisions);
  for (const path of conflict.mod_paths) {
    next.set(path, path === keepPath ? 'keep' : 'disable');
  }
  return next;
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
  const resolvedCount = conflicts.filter(
    (conflict) => conflict.mod_paths.filter((path) => !disabled.has(path)).length <= 1,
  ).length;

  return {
    disablePaths,
    resolvedCount,
    unresolvedCount: conflicts.length - resolvedCount,
  };
}
