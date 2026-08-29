import type { FolderNameConflictCandidate } from '../../../shared/api/tauri/bindings';

export type FolderConflictValidationCode =
  'empty' | 'invalid' | 'reserved' | 'disabled_prefix' | 'duplicate';

const RESERVED_NAME = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/i;
const INVALID_NAME = /[\\/:*?"<>|]/;
const DISABLED_PREFIX = /^disabled[\s_-]*/i;

function validateName(name: string): FolderConflictValidationCode | null {
  if (!name || name.trim() !== name) return 'empty';
  if (INVALID_NAME.test(name) || /[. ]$/.test(name)) return 'invalid';
  if (RESERVED_NAME.test(name)) return 'reserved';
  if (DISABLED_PREFIX.test(name)) return 'disabled_prefix';
  return null;
}

export function validateFolderConflictDrafts(
  candidates: FolderNameConflictCandidate[],
  drafts: Record<string, string>,
): Record<string, FolderConflictValidationCode> {
  const errors: Record<string, FolderConflictValidationCode> = {};
  const seen = new Map<string, string>();

  for (const candidate of candidates) {
    const value = drafts[candidate.path] ?? '';
    const validation = validateName(value);
    if (validation) errors[candidate.path] = validation;
    const identity = value.toLowerCase();
    const previous = seen.get(identity);
    if (previous) {
      errors[previous] = 'duplicate';
      errors[candidate.path] = 'duplicate';
    }
    seen.set(identity, candidate.path);
  }

  return errors;
}
