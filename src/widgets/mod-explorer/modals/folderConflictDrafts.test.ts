import { describe, expect, it } from 'vitest';
import type { FolderNameConflictCandidate } from '../../../shared/api/tauri/bindings';
import {
  createFolderConflictDraftState,
  reconcileFolderConflictDraftState,
} from './folderConflictDrafts';

function createCandidate(
  path: string,
  overrides: Partial<FolderNameConflictCandidate> = {},
): FolderNameConflictCandidate {
  const folderName = path.split('/').pop() ?? path;

  return {
    path,
    folder_name: folderName,
    base_name: '2.Jacket',
    is_enabled: false,
    ...overrides,
  };
}

describe('folder conflict drafts', () => {
  it('generates unique suffixes for every non-kept duplicate', () => {
    const candidates = [
      createCandidate('C:/Mods/2.Jacket', { is_enabled: true }),
      createCandidate('C:/Mods/DISABLED 2.Jacket'),
      createCandidate('C:/Mods/DISABLED-2.Jacket'),
    ];

    const state = createFolderConflictDraftState(candidates);

    expect(state.drafts).toEqual({
      'C:/Mods/2.Jacket': '2.Jacket',
      'C:/Mods/DISABLED 2.Jacket': '2.Jacket-02',
      'C:/Mods/DISABLED-2.Jacket': '2.Jacket-03',
    });
    expect(state.actions).toEqual({
      'C:/Mods/DISABLED 2.Jacket': 'rename',
      'C:/Mods/DISABLED-2.Jacket': 'rename',
    });
  });

  it('keeps generated names unique when the user changes the kept folder', () => {
    const candidates = [
      createCandidate('C:/Mods/2.Jacket', { is_enabled: true }),
      createCandidate('C:/Mods/DISABLED 2.Jacket'),
      createCandidate('C:/Mods/DISABLED-2.Jacket'),
    ];
    const initial = createFolderConflictDraftState(candidates);

    const changed = reconcileFolderConflictDraftState(
      candidates,
      initial.drafts,
      initial.keepPath,
      {},
      'C:/Mods/DISABLED 2.Jacket',
    );

    expect(changed.keepPath).toBe('C:/Mods/DISABLED 2.Jacket');
    expect(changed.drafts).toEqual({
      'C:/Mods/2.Jacket': '2.Jacket-02',
      'C:/Mods/DISABLED 2.Jacket': '2.Jacket',
      'C:/Mods/DISABLED-2.Jacket': '2.Jacket-03',
    });
    expect(changed.actions).toEqual({
      'C:/Mods/2.Jacket': 'rename',
      'C:/Mods/DISABLED-2.Jacket': 'rename',
    });
  });

  it('preserves a pending Trash choice while reconciling the draft', () => {
    const candidates = [
      createCandidate('C:/Mods/2.Jacket', { is_enabled: true }),
      createCandidate('C:/Mods/DISABLED 2.Jacket'),
      createCandidate('C:/Mods/DISABLED-2.Jacket'),
    ];
    const initial = createFolderConflictDraftState(candidates);

    const reconciled = reconcileFolderConflictDraftState(
      candidates,
      initial.drafts,
      initial.keepPath,
      {},
      undefined,
      { 'C:/Mods/DISABLED 2.Jacket': 'trash' },
    );

    expect(reconciled.actions).toEqual({
      'C:/Mods/DISABLED 2.Jacket': 'trash',
      'C:/Mods/DISABLED-2.Jacket': 'rename',
    });
  });
});
