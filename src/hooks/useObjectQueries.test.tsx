import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import {
  objectKeys,
  patchObjectEnabledCount,
  patchObjectRootSwitchState,
  runObjectBatchMutation,
} from './objectQueryCache';
import { useCategoryCounts } from './useObjectQueries';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useAppStore } from '../stores/useAppStore';
import React from 'react';
import type { ObjectSummary } from '../types/object';
import { workspaceKeys } from '../features/workspace-runtime/useWorkspaceViewModel';
import type { WorkspaceViewModel } from '../types/workspace';

vi.unmock('@tanstack/react-query');

// Mock dependecies
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

vi.mock('../lib/services/objectService', () => ({
  getCategoryCounts: vi.fn().mockImplementation((_gameId: string, safeMode: boolean) => {
    if (safeMode) {
      return Promise.resolve([{ category: 'Character', count: 5 }]);
    }
    return Promise.resolve([{ category: 'Character', count: 10 }]);
  }),
}));

vi.mock('./useActiveGame', () => ({
  useActiveGame: () => ({
    activeGame: {
      id: 'genshin',
    },
  }),
}));

const queryClient = new QueryClient();
const wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
);

describe('useCategoryCounts (TC-30 Privacy & Safe Mode)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    queryClient.clear();
  });

  // TC-30-003: Verify object list counts decrement appropriately when entering safe mode.
  it('TC-30-003: Fetches filtered counts based on safeMode state', async () => {
    // 1. Render hook with safeMode = false
    act(() => {
      useAppStore.setState({ safeMode: false });
    });

    const { result, rerender } = renderHook(() => useCategoryCounts(), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual([{ category: 'Character', count: 10 }]);

    // 2. Enable safeMode (simulating lock)
    act(() => {
      useAppStore.setState({ safeMode: true });
    });

    // Clear query client to force refetch or rely on different query keys
    queryClient.clear();
    rerender();

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    // The mock backend returns 5 when safeMode is true, simulating exact DB subtraction
    expect(result.current.data).toEqual([{ category: 'Character', count: 5 }]);
  });

  it('optimistically patches enabled_count and clamps to valid bounds', () => {
    const objectList: ObjectSummary[] = [
      {
        id: 'obj-1',
        name: 'Alpha',
        folder_path: 'Alpha',
        object_type: 'Character',
        sub_category: null,
        status: 1,
        created_at: null,
        mod_count: 3,
        enabled_count: 1,
        thumbnail_path: null,
        is_pinned: false,
        is_auto_sync: false,
        is_object_disabled: false,
        has_naming_conflict: false,
        metadata: '{}',
        tags: '[]',
        hash_db: null,
        custom_skins: null,
        matched_entry_key: null,
        matched_alias_name: null,
        matched_confidence: null,
        matched_reason: null,
        matched_source: null,
        active_mod_paths: null,
      },
    ];

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
      objectList,
    );

    patchObjectEnabledCount(queryClient, 'obj-1', 5);
    let patched = queryClient.getQueryData<ObjectSummary[]>(objectKeys.lists());
    expect(patched).toBeUndefined();

    patched = queryClient.getQueryData<ObjectSummary[]>(
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
    expect(patched?.[0].enabled_count).toBe(3);

    patchObjectEnabledCount(queryClient, 'obj-1', -10);
    patched = queryClient.getQueryData<ObjectSummary[]>(
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
    expect(patched?.[0].enabled_count).toBe(0);
  });

  it('optimistically patches object root switch state in object and workspace caches', () => {
    const filter = {
      game_id: 'genshin',
      safe_mode: false,
      object_type: null,
      search_query: null,
      meta_filters: null,
      sort_by: null,
      status_filter: null,
    };
    const object: ObjectSummary = {
      id: 'obj-1',
      name: 'Alpha',
      folder_path: 'Alpha',
      object_type: 'Character',
      sub_category: null,
      status: 1,
      created_at: null,
      mod_count: 3,
      enabled_count: 2,
      thumbnail_path: null,
      is_pinned: false,
      is_auto_sync: false,
      is_object_disabled: false,
      has_naming_conflict: false,
      metadata: '{}',
      tags: '[]',
      hash_db: null,
      custom_skins: null,
      matched_entry_key: null,
      matched_alias_name: null,
      matched_confidence: null,
      matched_reason: null,
      matched_source: null,
      active_mod_paths: null,
    };
    const workspace: WorkspaceViewModel = {
      objects: [
        {
          ...object,
          node_kind: 'object',
          display_mode: 'unknown',
          type_chip: null,
          display_name: 'Alpha',
          is_effectively_active: true,
          inactive_reason: null,
          warning_state: 'none',
          primary_warning: null,
          switch_state: 'enabled',
          switch_reason: null,
          switch_policy_key: 'object',
          capabilities: {
            can_toggle: true,
            can_rename: true,
            can_delete: true,
            can_move: false,
            can_toggle_safe: false,
            can_sync: true,
            can_enable_only_this: false,
            can_pin: true,
            can_edit_metadata: true,
            can_reveal_in_explorer: true,
            can_move_category: true,
            can_open_in_explorer: true,
          },
        },
      ],
      explorer: {
        self_node_type: null,
        self_node_kind: 'container',
        self_display_mode: 'unknown',
        self_type_chip: null,
        self_is_mod: false,
        self_is_enabled: true,
        self_is_effectively_active: true,
        self_owner_object_id: null,
        self_owner_object_folder_path: null,
        self_classification_reasons: [],
        children: [],
        conflicts: [],
        ancestor_disabled_by: null,
        ancestor_disabled_path: null,
        inactive_reason: null,
      },
      preview: {
        selected_path: null,
        selected_node: null,
        is_flat_mod_root: false,
        display_title: null,
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
        selected_object_folder_path: 'Alpha',
        explorer_sub_path: 'Alpha',
        selected_mod_path: null,
        current_path: ['Alpha'],
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

    queryClient.setQueryData(objectKeys.list(filter), [object]);
    queryClient.setQueryData(workspaceKeys.viewModel(filter, 'Alpha', 'Alpha', null), workspace);

    patchObjectRootSwitchState(queryClient, {
      objectId: 'obj-1',
      folderPath: 'DISABLED Alpha',
      enabled: false,
    });

    const list = queryClient.getQueryData<ObjectSummary[]>(objectKeys.list(filter));
    const patchedWorkspace = queryClient.getQueryData<WorkspaceViewModel>(
      workspaceKeys.viewModel(filter, 'Alpha', 'Alpha', null),
    );

    expect(list?.[0]).toMatchObject({
      folder_path: 'DISABLED Alpha',
      is_object_disabled: true,
      enabled_count: 0,
    });
    expect(patchedWorkspace?.objects[0]).toMatchObject({
      folder_path: 'DISABLED Alpha',
      is_object_disabled: true,
      enabled_count: 0,
      switch_state: 'disabled',
      is_effectively_active: false,
    });
  });

  it('runObjectBatchMutation restores cached rows on mutation failure', async () => {
    const queryKey = objectKeys.list({
      game_id: 'genshin',
      safe_mode: false,
      object_type: null,
      search_query: null,
      meta_filters: null,
      sort_by: null,
      status_filter: null,
    });
    const objectList: ObjectSummary[] = [
      {
        id: 'obj-1',
        name: 'Alpha',
        folder_path: 'Alpha',
        object_type: 'Character',
        sub_category: null,
        status: 1,
        created_at: null,
        mod_count: 3,
        enabled_count: 1,
        thumbnail_path: null,
        is_pinned: false,
        is_auto_sync: false,
        is_object_disabled: false,
        has_naming_conflict: false,
        metadata: '{}',
        tags: '[]',
        hash_db: null,
        custom_skins: null,
        matched_entry_key: null,
        matched_alias_name: null,
        matched_confidence: null,
        matched_reason: null,
        matched_source: null,
        active_mod_paths: null,
      },
    ];

    queryClient.setQueryData(queryKey, objectList);

    await expect(
      runObjectBatchMutation({
        queryClient,
        applyOptimisticUpdate: (object) => ({ ...object, is_pinned: true }),
        mutation: async () => {
          throw new Error('boom');
        },
      }),
    ).rejects.toThrow('boom');

    const restored = queryClient.getQueryData<ObjectSummary[]>(queryKey);
    expect(restored?.[0].is_pinned).toBe(false);
  });
});
