import type { FolderNameConflictCandidate } from '../../../lib/bindings';

export interface FolderConflictDraftState {
  drafts: Record<string, string>;
  keepPath: string | null;
  errors: Record<string, string>;
}

function defaultKeepPath(candidates: FolderNameConflictCandidate[]): string | null {
  return candidates.find((candidate) => candidate.is_enabled)?.path ?? candidates[0]?.path ?? null;
}

export function createFolderConflictDraftState(
  candidates: FolderNameConflictCandidate[],
): FolderConflictDraftState {
  return {
    drafts: Object.fromEntries(
      candidates.map((candidate) => [candidate.path, candidate.base_name]),
    ),
    keepPath: defaultKeepPath(candidates),
    errors: {},
  };
}

export function reconcileFolderConflictDraftState(
  candidates: FolderNameConflictCandidate[],
  currentDrafts: Record<string, string>,
  currentKeepPath: string | null,
  currentErrors: Record<string, string>,
): FolderConflictDraftState {
  const candidatePaths = new Set(candidates.map((candidate) => candidate.path));
  const keepPath =
    currentKeepPath && candidatePaths.has(currentKeepPath)
      ? currentKeepPath
      : defaultKeepPath(candidates);
  const keepChanged = keepPath !== currentKeepPath;

  return {
    drafts: Object.fromEntries(
      candidates.map((candidate) => [
        candidate.path,
        keepChanged && candidate.path === keepPath
          ? candidate.base_name
          : (currentDrafts[candidate.path] ?? candidate.base_name),
      ]),
    ),
    keepPath,
    errors: Object.fromEntries(
      Object.entries(currentErrors).filter(([path]) => candidatePaths.has(path)),
    ),
  };
}
