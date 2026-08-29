import { describe, expect, it } from 'vitest';
import { reconcileFolderConflictQueue, selectNextFolderConflictGroup } from './folderConflictQueue';

const group = (groupId: string, candidatePaths: string[]) => ({
  group_id: groupId,
  identity: groupId,
  display_name: groupId,
  candidates: candidatePaths.map((path) => ({
    path,
    folder_name: path,
    base_name: path,
    is_enabled: true,
  })),
});

describe('folder conflict queue', () => {
  it('marks a disappeared group complete and selects the next unresolved group', () => {
    const previous = [
      group('one', ['one/a']),
      group('two', ['two/a']),
      group('three', ['three/a']),
    ];
    const next = [group('one', ['one/a']), group('three', ['three/a'])];

    expect(reconcileFolderConflictQueue(previous, next, [])).toEqual([
      { group_id: 'two', fingerprint: 'two:two/a:1', display_name: 'two' },
    ]);
    expect(selectNextFolderConflictGroup(previous, next, 'two')).toBe('three');
  });

  it('removes an old completion if the same conflict fingerprint reappears', () => {
    const current = [group('two', ['two/a'])];

    expect(
      reconcileFolderConflictQueue([], current, [
        { group_id: 'two', fingerprint: 'two:two/a:1', display_name: 'two' },
      ]),
    ).toEqual([]);
  });

  it('does not mark a group resolved when Trash only changes its remaining candidates', () => {
    const previous = [group('two', ['two/a', 'two/b', 'two/c'])];
    const current = [group('two', ['two/a', 'two/b'])];

    expect(reconcileFolderConflictQueue(previous, current, [])).toEqual([]);
    expect(selectNextFolderConflictGroup(previous, current, 'two')).toBe('two');
  });

  it('starts a fresh checklist when a new conflict episode begins', () => {
    const completed = [{ group_id: 'old', fingerprint: 'old:old/a:1', display_name: 'old' }];

    expect(reconcileFolderConflictQueue([], [group('new', ['new/a'])], completed)).toEqual([]);
  });
});
