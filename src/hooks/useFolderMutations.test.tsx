import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '../stores/useAppStore';
import { workspaceKeys } from '../features/workspace-runtime/useWorkspaceViewModel';
import type {
  WorkspaceCapabilities,
  WorkspaceExplorerNode,
  WorkspaceViewModel,
} from '../types/workspace';
import { useBulkToggle } from './useFolderMutations';

const bulkToggleMods = vi.fn();

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('../lib/bindings', () => ({
  commands: {
    bulkToggleMods: (...args: unknown[]) => bulkToggleMods(...args),
  },
}));

vi.mock('../stores/useToastStore', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

const capabilities: WorkspaceCapabilities = {
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
};

function createNode(path: string): WorkspaceExplorerNode {
  return {
    node_type: 'FlatModRoot',
    classification_reasons: [],
    id: 'mod-1',
    owner_object_id: 'obj-1',
    owner_object_folder_path: 'ALBEDO',
    name: 'Variant',
    folder_name: 'Variant',
    path,
    is_enabled: true,
    is_directory: true,
    thumbnail_path: null,
    modified_at: 0,
    size_bytes: 0,
    has_info_json: false,
    is_favorite: false,
    is_misplaced: false,
    is_safe: true,
    metadata: null,
    category: null,
    conflict_group_id: null,
    conflict_state: null,
    pin_hash: null,
    warnings: [],
    node_kind: 'terminal_mod',
    display_mode: 'flat_mod',
    type_chip: 'flat_mod',
    display_name: 'Variant',
    is_effectively_active: true,
    ancestor_disabled: false,
    inactive_reason: null,
    warning_state: 'none',
    primary_warning: null,
    switch_state: 'enabled',
    switch_reason: null,
    switch_policy_key: 'mod',
    capabilities,
    can_navigate: false,
  };
}

function createWorkspace(path: string): WorkspaceViewModel {
  const node = createNode(path);
  return {
    objects: [],
    explorer: {
      self_node_type: null,
      self_node_kind: 'container',
      self_display_mode: 'container_folder',
      self_type_chip: null,
      self_is_mod: false,
      self_is_enabled: true,
      self_is_effectively_active: true,
      self_owner_object_id: null,
      self_owner_object_folder_path: null,
      self_classification_reasons: [],
      children: [node],
      conflicts: [],
      ancestor_disabled_by: null,
      ancestor_disabled_path: null,
      inactive_reason: null,
    },
    preview: {
      selected_path: path,
      selected_node: node,
      is_flat_mod_root: true,
      display_title: 'Variant',
      display_subtitle: null,
      mod_info_summary: null,
      ini_summary: null,
      image_summary: null,
      warning_summary: {
        state: 'none',
        messages: [],
      },
    },
    selection: {
      selected_object_folder_path: 'ALBEDO',
      explorer_sub_path: 'ALBEDO',
      selected_mod_path: path,
      current_path: ['ALBEDO'],
      reconciliation_status: 'unchanged',
      reconciliation_reason: null,
      affected_paths: [],
    },
    runtime: {
      game_id: 'game-1',
      safe_mode: false,
      source_state: {
        status: 'available',
        message: null,
      },
    },
  };
}

function createWrapper(queryClient: QueryClient) {
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

describe('useBulkToggle', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    vi.clearAllMocks();
    queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    queryClient.clear();
    useAppStore.setState({
      gridSelection: new Set(['E:\\Mods\\ALBEDO\\Variant']),
      selectedModPath: 'E:\\Mods\\ALBEDO\\Variant',
      selectedObjectFolderPath: 'ALBEDO',
      explorerSubPath: 'ALBEDO',
      currentPath: ['ALBEDO'],
    });
  });

  it('keeps selected folder and cached preview on the toggled path', async () => {
    const oldPath = 'E:/Mods/ALBEDO/Variant';
    const newPath = 'E:/Mods/ALBEDO/DISABLED Variant';
    const workspaceKey = workspaceKeys.viewModel(
      {
        game_id: 'game-1',
        safe_mode: false,
        object_type: null,
        search_query: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      },
      'ALBEDO',
      'ALBEDO',
      oldPath,
    );
    queryClient.setQueryData(workspaceKey, createWorkspace(oldPath));
    bulkToggleMods.mockResolvedValue({
      success: [newPath],
      failures: [],
      collection_impact: {
        affected_collection_count: 0,
        affected_collection_names: [],
        rewritten_paths: [],
        missing_paths: [],
      },
      path_rewrites: [{ old_path: oldPath, new_path: newPath }],
    });

    const { result } = renderHook(() => useBulkToggle(), {
      wrapper: createWrapper(queryClient),
    });

    await act(async () => {
      await result.current.mutateAsync({
        gameId: 'game-1',
        paths: ['E:\\Mods\\ALBEDO\\Variant'],
        enable: false,
      });
    });

    expect(bulkToggleMods).toHaveBeenCalledWith({
      gameId: 'game-1',
      paths: ['E:\\Mods\\ALBEDO\\Variant'],
      enable: false,
    });
    await waitFor(() => {
      expect(useAppStore.getState().selectedModPath).toBe(newPath);
    });
    expect(useAppStore.getState().gridSelection.has(newPath)).toBe(true);
    expect(useAppStore.getState().gridSelection.has('E:\\Mods\\ALBEDO\\Variant')).toBe(false);
    expect(queryClient.getQueryData<WorkspaceViewModel>(workspaceKey)?.preview.selected_path).toBe(
      newPath,
    );
  });
});
