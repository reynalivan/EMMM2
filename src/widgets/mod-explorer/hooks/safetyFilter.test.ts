import { describe, expect, it } from 'vitest';
import type { WorkspaceExplorerNode } from '@/entities/workspace/model/workspace';
import { filterFoldersBySafety } from './safetyFilter';

function folder(path: string, isSafe: boolean, classified = true): WorkspaceExplorerNode {
  return {
    path,
    is_safe: isSafe,
    is_safety_classified: classified,
    contains_safe_mods: classified && isSafe,
    contains_unsafe_mods: classified && !isSafe,
  } as WorkspaceExplorerNode;
}

describe('filterFoldersBySafety', () => {
  const folders = [folder('Safe', true), folder('Unsafe', false), folder('Unknown', true, false)];

  it('shows every folder by default', () => {
    expect(filterFoldersBySafety(folders, 'all')).toEqual(folders);
  });

  it('filters safe and unsafe folders without mutating the source', () => {
    expect(filterFoldersBySafety(folders, 'safe').map((entry) => entry.path)).toEqual(['Safe']);
    expect(filterFoldersBySafety(folders, 'unsafe').map((entry) => entry.path)).toEqual(['Unsafe']);
    expect(folders).toHaveLength(3);
  });

  it('keeps mixed parent folders navigable in both filtered views', () => {
    const mixedParent = {
      ...folder('Mixed', false, false),
      contains_safe_mods: true,
      contains_unsafe_mods: true,
    };

    expect(filterFoldersBySafety([mixedParent], 'safe')).toEqual([mixedParent]);
    expect(filterFoldersBySafety([mixedParent], 'unsafe')).toEqual([mixedParent]);
  });
});
