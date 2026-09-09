import type { FolderNameConflictCandidate } from '../../../shared/api/tauri/bindings';

export type FolderConflictCandidateAction = 'rename' | 'trash';

export interface FolderConflictDraftState {
  drafts: Record<string, string>;
  keepPath: string | null;
  errors: Record<string, string>;
  actions: Record<string, FolderConflictCandidateAction>;
}

function normalizeName(name: string): string {
  return name.toLowerCase();
}

function defaultKeepPath(candidates: FolderNameConflictCandidate[]): string | null {
  return candidates.find((candidate) => candidate.is_enabled)?.path ?? candidates[0]?.path ?? null;
}

function nextAvailableName(baseName: string, usedNames: Set<string>): string {
  let suffix = 2;
  let candidate = `${baseName}-${String(suffix).padStart(2, '0')}`;

  while (usedNames.has(normalizeName(candidate))) {
    suffix += 1;
    candidate = `${baseName}-${String(suffix).padStart(2, '0')}`;
  }

  return candidate;
}

function buildDefaultDrafts(
  candidates: FolderNameConflictCandidate[],
  keepPath: string | null,
): Record<string, string> {
  const keepCandidate = candidates.find((candidate) => candidate.path === keepPath);
  const usedNames = new Set<string>();
  const drafts: Record<string, string> = {};

  if (keepCandidate) {
    drafts[keepCandidate.path] = keepCandidate.base_name;
    usedNames.add(normalizeName(keepCandidate.base_name));
  }

  for (const candidate of candidates) {
    if (candidate.path === keepPath) continue;

    const baseName = candidate.base_name;
    const normalizedBaseName = normalizeName(baseName);
    const draft = usedNames.has(normalizedBaseName)
      ? nextAvailableName(baseName, usedNames)
      : baseName;

    drafts[candidate.path] = draft;
    usedNames.add(normalizeName(draft));
  }

  return drafts;
}

function buildDefaultActions(
  candidates: FolderNameConflictCandidate[],
  keepPath: string | null,
): Record<string, FolderConflictCandidateAction> {
  return Object.fromEntries(
    candidates
      .filter((candidate) => candidate.path !== keepPath)
      .map((candidate) => [candidate.path, 'rename' as const]),
  );
}

export function createFolderConflictDraftState(
  candidates: FolderNameConflictCandidate[],
): FolderConflictDraftState {
  const keepPath = defaultKeepPath(candidates);

  return {
    drafts: buildDefaultDrafts(candidates, keepPath),
    keepPath,
    errors: {},
    actions: buildDefaultActions(candidates, keepPath),
  };
}

export function reconcileFolderConflictDraftState(
  candidates: FolderNameConflictCandidate[],
  currentDrafts: Record<string, string>,
  currentKeepPath: string | null,
  currentErrors: Record<string, string>,
  requestedKeepPath?: string | null,
  currentActions: Record<string, FolderConflictCandidateAction> = {},
): FolderConflictDraftState {
  const candidatePaths = new Set(candidates.map((candidate) => candidate.path));
  const keepPath =
    requestedKeepPath && candidatePaths.has(requestedKeepPath)
      ? requestedKeepPath
      : currentKeepPath && candidatePaths.has(currentKeepPath)
        ? currentKeepPath
        : defaultKeepPath(candidates);
  const keepChanged = keepPath !== currentKeepPath;
  const defaultDrafts = buildDefaultDrafts(candidates, keepPath);
  const defaultActions = buildDefaultActions(candidates, keepPath);
  const previousKeepCandidate = candidates.find((candidate) => candidate.path === currentKeepPath);

  return {
    drafts: Object.fromEntries(
      candidates.map((candidate) => [
        candidate.path,
        candidate.path === keepPath
          ? candidate.base_name
          : keepChanged &&
              candidate.path === previousKeepCandidate?.path &&
              currentDrafts[candidate.path] === previousKeepCandidate.base_name
            ? defaultDrafts[candidate.path]
            : (currentDrafts[candidate.path] ??
              defaultDrafts[candidate.path] ??
              candidate.base_name),
      ]),
    ),
    keepPath,
    errors: Object.fromEntries(
      Object.entries(currentErrors).filter(([path]) => candidatePaths.has(path)),
    ),
    actions: Object.fromEntries(
      candidates
        .filter((candidate) => candidate.path !== keepPath)
        .map((candidate) => [
          candidate.path,
          currentActions[candidate.path] ?? defaultActions[candidate.path] ?? 'rename',
        ]),
    ),
  };
}
