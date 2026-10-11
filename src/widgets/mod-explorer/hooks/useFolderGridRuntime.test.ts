import { act, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  EXPLORER_SEARCH_DEBOUNCE_MS,
  getPreviousExplorerSubPath,
  groupExplorerConflictNodes,
  useDebouncedExplorerSearchQuery,
} from './useFolderGridRuntime';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import type { FolderNameConflictGroup } from '@/shared/api/tauri/bindings';
import { pathBasename } from '@/shared/lib/pathKey';

function folderNode(path: string): WorkspaceExplorerNode {
  return {
    path,
    name: pathBasename(path),
    display_name: pathBasename(path),
    folder_name: pathBasename(path),
    node_type: 'ContainerFolder',
    node_kind: 'container',
    display_mode: 'container_folder',
    classification_reasons: [],
    id: null,
    owner_object_id: null,
    owner_object_folder_path: null,
    is_enabled: true,
    is_effectively_active: true,
    is_directory: true,
    thumbnail_path: null,
    modified_at: 0,
    size_bytes: 0,
    has_info_json: false,
    is_favorite: false,
    is_misplaced: false,
    is_safe: true,
    is_safety_classified: true,
    contains_safe_mods: false,
    contains_unsafe_mods: false,
    metadata: null,
    category: null,
    conflict_group_id: null,
    conflict_state: null,
    warnings: [],
    type_chip: null,
    ancestor_disabled: false,
    inactive_reason: null,
    warning_state: 'none',
    primary_warning: null,
    switch_state: 'enabled',
    switch_reason: null,
    switch_policy_key: 'mod',
    can_navigate: true,
    capabilities: {
      can_toggle: true,
      can_rename: true,
      can_delete: true,
      can_move: true,
      can_toggle_safe: true,
      can_sync: true,
      can_enable_only_this: true,
      can_pin: true,
      can_edit_metadata: true,
      can_reveal_in_explorer: true,
      can_move_category: true,
      can_open_in_explorer: true,
    },
  };
}

const conflict: FolderNameConflictGroup = {
  group_id: 'blue-conflict',
  identity: 'alice/blue',
  display_name: 'Blue',
  candidates: [
    'E:/Mods/Alice/Blue',
    'E:/Mods/Alice/DISABLED Blue',
    'E:/Mods/DISABLED Alice/Blue',
  ].map((path) => ({
    path,
    folder_name: pathBasename(path),
    base_name: 'Blue',
    is_enabled: !path.includes('DISABLED'),
  })),
};

afterEach(() => {
  vi.useRealTimers();
});

describe('groupExplorerConflictNodes', () => {
  it('groups only exact candidate siblings and keeps nested/other-parent rows separate', () => {
    const paths = [
      'E:/Mods/Alice/Blue',
      'E:/Mods/Alice/DISABLED Blue',
      'E:/Mods/Alice/Blue/Child',
      'E:/Mods/DISABLED Alice/Blue',
      'E:/Mods/Bob/Blue',
    ];
    const grouped = groupExplorerConflictNodes(paths.map(folderNode), [conflict]);

    expect(grouped.map((node) => node.path)).toEqual([paths[0], paths[2], paths[3], paths[4]]);
    expect(grouped[0].capabilities.can_toggle).toBe(false);
    expect(grouped[0].can_navigate).toBe(false);
    expect(grouped[1].conflict_group_id).toBeNull();
    expect(grouped[2].conflict_group_id).toBe('blue-conflict');
  });

  it('groups candidates from separate loaded pages without collapsing normal disabled folders', () => {
    const firstPage = [folderNode('E:/Mods/Alice/DISABLED Blue')];
    expect(groupExplorerConflictNodes(firstPage, [conflict])).toHaveLength(1);
    const secondPage = [
      folderNode('E:/Mods/Alice/Blue'),
      folderNode('E:/Mods/Alice/DISABLED Green'),
    ];
    const grouped = groupExplorerConflictNodes([...firstPage, ...secondPage], [conflict]);

    expect(grouped.map((node) => node.path)).toEqual([
      'E:/Mods/Alice/DISABLED Blue',
      'E:/Mods/Alice/DISABLED Green',
    ]);
    expect(grouped[0].name).toBe('Blue');
    expect(grouped[1].conflict_group_id).toBeNull();
  });

  it('matches physical path casing and separators without inferring conflicts from prefixes', () => {
    const folders = ['e:\\mods\\alice\\BLUE', 'E:/Mods/Alice/DISABLED Blue'].map(folderNode);
    expect(groupExplorerConflictNodes(folders, [conflict])).toHaveLength(1);
    expect(groupExplorerConflictNodes(folders, [])).toBe(folders);
  });
});

describe('getPreviousExplorerSubPath', () => {
  it('targets the direct parent view for a breadcrumb folder switcher', () => {
    expect(getPreviousExplorerSubPath('SkinSelectImpact/Aglaea')).toBe('SkinSelectImpact');
    expect(getPreviousExplorerSubPath('E:/Mods/SkinSelectImpact/Aglaea')).toBe(
      'E:/Mods/SkinSelectImpact',
    );
    expect(getPreviousExplorerSubPath('SkinSelectImpact')).toBeUndefined();
  });

  it('debounces only the backend explorer search value', () => {
    vi.useFakeTimers();
    const { result, rerender } = renderHook(
      ({ search }) => useDebouncedExplorerSearchQuery(search),
      { initialProps: { search: '' } },
    );

    rerender({ search: 'amber' });
    expect(result.current).toBe('');

    act(() => vi.advanceTimersByTime(EXPLORER_SEARCH_DEBOUNCE_MS - 1));
    expect(result.current).toBe('');

    act(() => vi.advanceTimersByTime(1));
    expect(result.current).toBe('amber');
  });
});
