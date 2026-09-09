import { describe, expect, it } from 'vitest';
import type { FolderNameConflictCandidate } from '../../../shared/api/tauri/bindings';
import { validateFolderConflictDrafts } from './folderConflictValidation';

const candidates: FolderNameConflictCandidate[] = [
  {
    path: 'C:/Mods/Alice/Blue',
    folder_name: 'Blue',
    base_name: 'Blue',
    is_enabled: true,
  },
  {
    path: 'C:/Mods/Alice/DISABLED Blue',
    folder_name: 'DISABLED Blue',
    base_name: 'Blue',
    is_enabled: false,
  },
];

describe('validateFolderConflictDrafts', () => {
  it('rejects a status prefix in the editable base name', () => {
    expect(
      validateFolderConflictDrafts(candidates, {
        [candidates[0].path]: 'DISABLED Blue One',
        [candidates[1].path]: 'Blue Two',
      }),
    ).toEqual({ [candidates[0].path]: 'disabled_prefix' });
  });

  it('detects case-only duplicate identities with locale-invariant casing', () => {
    expect(
      validateFolderConflictDrafts(candidates, {
        [candidates[0].path]: 'FILE',
        [candidates[1].path]: 'file',
      }),
    ).toEqual({
      [candidates[0].path]: 'duplicate',
      [candidates[1].path]: 'duplicate',
    });
  });

  it('rejects keeping the original name for a folder being renamed', () => {
    const distinctCandidates: FolderNameConflictCandidate[] = [
      { ...candidates[0], base_name: 'Blue' },
      { ...candidates[1], base_name: 'Green', folder_name: 'DISABLED Green' },
    ];

    expect(
      validateFolderConflictDrafts(
        distinctCandidates,
        {
          [distinctCandidates[0].path]: 'Blue',
          [distinctCandidates[1].path]: 'Green',
        },
        distinctCandidates[1].path,
      ),
    ).toEqual({ [distinctCandidates[0].path]: 'unchanged' });
  });
});
