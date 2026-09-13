import { describe, expect, it } from 'vitest';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import { filterFoldersBySafety } from './safetyFilter';

function folder(
  path: string,
  isSafe: boolean,
  classified = true,
  nodeKind: WorkspaceExplorerNode['node_kind'] = 'terminal_mod',
): WorkspaceExplorerNode {
  return {
    path,
    is_safe: isSafe,
    is_safety_classified: classified,
    contains_safe_mods: classified && isSafe,
    contains_unsafe_mods: classified && !isSafe,
    node_kind: nodeKind,
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

  it('keeps neutral navigation folders visible while filtering terminal mods', () => {
    const container = folder('Container', false, false, 'container');
    const inactiveBranch = folder('Inactive', false, false, 'inactive_branch');

    expect(
      filterFoldersBySafety([...folders, container, inactiveBranch], 'safe').map(
        (entry) => entry.path,
      ),
    ).toEqual(['Safe', 'Container', 'Inactive']);
    expect(
      filterFoldersBySafety([...folders, container, inactiveBranch], 'unsafe').map(
        (entry) => entry.path,
      ),
    ).toEqual(['Unsafe', 'Container', 'Inactive']);
  });
});
