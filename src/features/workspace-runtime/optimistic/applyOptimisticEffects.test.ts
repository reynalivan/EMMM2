import { describe, expect, it, beforeEach } from 'vitest';
import { QueryClient } from '@tanstack/react-query';
import { applyRuntimeEffects } from './applyOptimisticEffects';
import {
  buildObjectCountDeltaDescriptor,
  buildPathInvalidationDescriptor,
  buildPathRewriteDescriptor,
  buildQueryInvalidationDescriptor,
  buildQueryRemovalDescriptor,
} from './descriptorBuilders';
import { mergeRuntimeEffectDescriptors } from './descriptor';
import { objectKeys } from '../../../hooks/objectQueryCache';
import { useAppStore } from '../../../stores/useAppStore';
import { thumbnailKeys } from '../../../hooks/useThumbnail';
import { detailsKeys } from '../../preview/hooks/usePreviewData';
import { workspaceKeys } from '../useWorkspaceViewModel';
import type {
  WorkspaceCapabilities,
  WorkspaceExplorerNode,
  WorkspaceViewModel,
} from '../../../types/workspace';

const enabledCapabilities: WorkspaceCapabilities = {
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

function createWorkspaceExplorerNode(path: string): WorkspaceExplorerNode {
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
    capabilities: enabledCapabilities,
    can_navigate: false,
  };
}

function createWorkspaceViewModel(path: string): WorkspaceViewModel {
  const node = createWorkspaceExplorerNode(path);
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
      game_id: 'genshin',
      safe_mode: false,
      source_state: {
        status: 'available',
        message: null,
      },
    },
  };
}

describe('applyRuntimeEffects', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient();
    queryClient.clear();
    useAppStore.setState({
      selectedObjectFolderPath: 'ALBEDO',
      explorerSubPath: 'ALBEDO/Variants',
      currentPath: ['ALBEDO', 'Variants'],
      selectedModPath: 'E:/Mods/ALBEDO/Variants/mod.ini',
    });
  });

  it('rewrites runtime selection paths', () => {
    applyRuntimeEffects(
      queryClient,
      buildPathRewriteDescriptor('E:/Mods/ALBEDO/Variants', 'E:/Mods/ALBEDO/Presets', []),
    );

    const state = useAppStore.getState();
    expect(state.explorerSubPath).toBe('ALBEDO/Presets');
    expect(state.currentPath).toEqual(['ALBEDO', 'Presets']);
    expect(state.selectedModPath).toBe('E:/Mods/ALBEDO/Presets/mod.ini');
  });

  it('rewrites cached workspace view-model preview and selection paths', () => {
    const oldPath = 'E:/Mods/ALBEDO/Variant';
    const newPath = 'E:/Mods/ALBEDO/DISABLED Variant';
    const workspaceKey = workspaceKeys.viewModel(
      {
        game_id: 'genshin',
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
    queryClient.setQueryData(workspaceKey, createWorkspaceViewModel(oldPath));

    applyRuntimeEffects(queryClient, buildPathRewriteDescriptor(oldPath, newPath, []));

    const workspace = queryClient.getQueryData<WorkspaceViewModel>(workspaceKey);
    expect(workspace?.selection.selected_mod_path).toBe(newPath);
    expect(workspace?.preview.selected_path).toBe(newPath);
    expect(workspace?.explorer.children[0]?.path).toBe(newPath);
    expect(
      workspace?.preview.selected_node && 'path' in workspace.preview.selected_node
        ? workspace.preview.selected_node.path
        : null,
    ).toBe(newPath);
  });

  it('replaces grid selection using normalized path separators', () => {
    useAppStore.setState({
      gridSelection: new Set(['E:\\Mods\\ALBEDO\\Variant']),
      selectedModPath: 'E:\\Mods\\ALBEDO\\Variant',
    });

    useAppStore
      .getState()
      .replaceGridSelection('E:/Mods/ALBEDO/Variant', 'E:/Mods/ALBEDO/DISABLED Variant');

    const state = useAppStore.getState();
    expect(state.gridSelection.has('E:/Mods/ALBEDO/DISABLED Variant')).toBe(true);
    expect(state.gridSelection.has('E:\\Mods\\ALBEDO\\Variant')).toBe(false);
    expect(state.selectedModPath).toBe('E:/Mods/ALBEDO/DISABLED Variant');
  });

  it('patches workspace object enabled count deterministically', () => {
    queryClient.setQueryData(
      objectKeys.list({
        game_id: 'genshin',
        safe_mode: false,
        object_type: null,
        search_query: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      }),
      [
        {
          id: 'obj-1',
          name: 'Alpha',
          folder_path: 'Alpha',
          object_type: 'Character',
          sub_category: null,
          status: 'active',
          metadata: '{}',
          tags: '[]',
          hash_db: null,
          custom_skins: null,
          thumbnail_path: null,
          is_auto_sync: false,
          mod_count: 4,
          enabled_count: 1,
          is_pinned: false,
          has_naming_conflict: false,
          matched_alias_name: null,
          is_object_disabled: false,
        },
      ],
    );

    applyRuntimeEffects(queryClient, buildObjectCountDeltaDescriptor('obj-1', 2, []));

    const list = queryClient.getQueryData<
      {
        id: string;
        enabled_count: number;
      }[]
    >(
      objectKeys.list({
        game_id: 'genshin',
        safe_mode: false,
        object_type: null,
        search_query: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      }),
    );

    expect(list?.[0]?.enabled_count).toBe(3);
  });

  it('removes thumbnail queries and invalidates detail queries from descriptor effects', () => {
    const thumbnailKey = thumbnailKeys.folder('E:/Mods/ALBEDO');
    const previewKey = detailsKeys.previewImages('E:/Mods/ALBEDO');
    queryClient.setQueryData(thumbnailKey, 'thumb');
    queryClient.setQueryData(previewKey, ['preview.png']);

    applyRuntimeEffects(
      queryClient,
      mergeRuntimeEffectDescriptors(
        buildQueryRemovalDescriptor([thumbnailKey], []),
        buildQueryInvalidationDescriptor([previewKey], []),
      ),
    );

    expect(queryClient.getQueryData(thumbnailKey)).toBeUndefined();
    expect(queryClient.getQueryState(previewKey)?.isInvalidated).toBe(true);
  });

  it('clears stale runtime selection when a path is invalidated', () => {
    applyRuntimeEffects(
      queryClient,
      buildPathInvalidationDescriptor('E:/Mods/ALBEDO/Variants', []),
    );

    const state = useAppStore.getState();
    expect(state.selectedModPath).toBeNull();
    expect(state.explorerSubPath).toBe('ALBEDO/Variants');
    expect(state.currentPath).toEqual(['ALBEDO', 'Variants']);
  });
});
