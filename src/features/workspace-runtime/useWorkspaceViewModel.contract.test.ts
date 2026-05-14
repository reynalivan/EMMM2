import { describe, expect, it } from 'vitest';
import {
  buildSelectionReconciledEvent,
  buildWorkspaceViewModelFilter,
  buildWorkspaceViewModelInput,
  workspaceKeys,
} from './useWorkspaceViewModel';

describe('useWorkspaceViewModel contract', () => {
  it('builds workspace filter from app runtime inputs', () => {
    const filter = buildWorkspaceViewModelFilter({
      gameId: 'game-1',
      safeMode: true,
      selectedObjectType: 'Character',
      objectMetaFilters: { element: ['Pyro'] },
      objectSortBy: 'name',
      objectStatusFilter: 'enabled',
    });

    expect(filter).toEqual({
      game_id: 'game-1',
      safe_mode: true,
      object_type: 'Character',
      search_query: null,
      meta_filters: { element: ['Pyro'] },
      sort_by: 'name',
      status_filter: 1,
    });
  });

  it('builds command input from filter and runtime selection', () => {
    const filter = buildWorkspaceViewModelFilter({
      gameId: 'game-1',
      safeMode: true,
      selectedObjectType: 'Character',
      objectMetaFilters: { element: ['Pyro'] },
      objectSortBy: 'name',
      objectStatusFilter: 'enabled',
    });

    const input = buildWorkspaceViewModelInput(filter, {
      selectedObjectFolderPath: 'Objects/Diluc',
      explorerSubPath: 'Objects/Diluc/Variants',
      selectedModPath: 'Objects/Diluc/Variants/mod.ini',
    });

    expect(input).toEqual({
      filter: {
        game_id: 'game-1',
        safe_mode: true,
        object_type: 'Character',
        search_query: null,
        meta_filters: { element: ['Pyro'] },
        sort_by: 'name',
        status_filter: 1,
      },
      selected_object_folder_path: 'Objects/Diluc',
      explorer_sub_path: 'Objects/Diluc/Variants',
      selected_mod_path: 'Objects/Diluc/Variants/mod.ini',
    });
  });

  it('uses workspace view-model query key that includes runtime location', () => {
    const queryKey = workspaceKeys.viewModel(
      {
        game_id: 'game-1',
        safe_mode: true,
        object_type: 'Character',
        search_query: null,
        meta_filters: { element: ['Pyro'] },
        sort_by: 'name',
        status_filter: 1,
      },
      'Objects/Diluc',
      'Objects/Diluc/Variants',
      'Objects/Diluc/Variants/mod.ini',
    );

    expect(queryKey).toEqual([
      'workspace',
      'mods',
      {
        game_id: 'game-1',
        safe_mode: true,
        object_type: 'Character',
        search_query: null,
        meta_filters: { element: ['Pyro'] },
        sort_by: 'name',
        status_filter: 1,
      },
      'Objects/Diluc',
      'Objects/Diluc/Variants',
      'Objects/Diluc/Variants/mod.ini',
    ]);
  });

  it('maps selection reconciliation status, reason, and affected paths to runtime event', () => {
    const event = buildSelectionReconciledEvent({
      selected_object_folder_path: null,
      explorer_sub_path: null,
      selected_mod_path: null,
      current_path: [],
      reconciliation_status: 'cleared',
      reconciliation_reason: 'source_unavailable',
      affected_paths: ['E:/Mods'],
    });

    expect(event).toEqual({
      type: 'SELECTION_RECONCILED',
      selectedObjectFolderPath: null,
      explorerSubPath: undefined,
      selectedModPath: null,
      currentPath: [],
      reconciliationStatus: 'cleared',
      reconciliationReason: 'source_unavailable',
      affectedPaths: ['E:/Mods'],
    });
  });
});
