import { beforeEach, describe, expect, it } from 'vitest';
import {
  buildSelectionReconciledEvent,
  recordInternalWorkspacePathRewrites,
  resetWorkspaceSelectionReconciliationGuardsForTest,
  shouldRunSelectionReconciliationEffect,
  shouldShowSelectionReconciliationToast,
  buildWorkspaceViewModelFilter,
  buildWorkspaceViewModelInput,
  shouldApplySelectionReconciledEvent,
  workspaceKeys,
} from './useWorkspaceViewModel';

describe('useWorkspaceViewModel contract', () => {
  beforeEach(() => {
    resetWorkspaceSelectionReconciliationGuardsForTest();
  });

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

  it('ignores stale unchanged read-model selection after a runtime path rewrite', () => {
    const shouldApply = shouldApplySelectionReconciledEvent(
      {
        selectedObjectFolderPath: 'ALBEDO',
        explorerSubPath: 'ALBEDO',
        selectedModPath: 'E:/Mods/ALBEDO/DISABLED Variant',
      },
      {
        selected_object_folder_path: 'ALBEDO',
        explorer_sub_path: 'ALBEDO',
        selected_mod_path: 'E:/Mods/ALBEDO/Variant',
        current_path: ['ALBEDO'],
        reconciliation_status: 'unchanged',
        reconciliation_reason: null,
        affected_paths: [],
      },
    );

    expect(shouldApply).toBe(false);
  });

  it('applies explicit backend fallback or clear reconciliation', () => {
    const shouldApply = shouldApplySelectionReconciledEvent(
      {
        selectedObjectFolderPath: 'ALBEDO',
        explorerSubPath: 'ALBEDO/Deleted',
        selectedModPath: 'E:/Mods/ALBEDO/Deleted',
      },
      {
        selected_object_folder_path: 'ALBEDO',
        explorer_sub_path: 'ALBEDO',
        selected_mod_path: null,
        current_path: ['ALBEDO'],
        reconciliation_status: 'fallback',
        reconciliation_reason: 'missing_explorer_path',
        affected_paths: ['ALBEDO/Deleted'],
      },
    );

    expect(shouldApply).toBe(true);
  });

  it('ignores stale fallback reconciliation covered by a recent internal path rewrite', () => {
    recordInternalWorkspacePathRewrites(
      [{ oldPath: 'E:/Mods/ALBEDO/Variant', newPath: 'E:/Mods/ALBEDO/DISABLED Variant' }],
      1_000,
    );

    const shouldApply = shouldApplySelectionReconciledEvent(
      {
        selectedObjectFolderPath: 'ALBEDO',
        explorerSubPath: 'ALBEDO',
        selectedModPath: 'E:/Mods/ALBEDO/DISABLED Variant',
      },
      {
        selected_object_folder_path: 'ALBEDO',
        explorer_sub_path: 'ALBEDO',
        selected_mod_path: null,
        current_path: ['ALBEDO'],
        reconciliation_status: 'fallback',
        reconciliation_reason: 'missing_mod_path',
        affected_paths: ['E:/Mods/ALBEDO/Variant'],
      },
      1_500,
    );

    expect(shouldApply).toBe(false);
  });

  it('suppresses disk-change toast for internal enable-disable rewrites', () => {
    recordInternalWorkspacePathRewrites(
      [{ oldPath: 'E:/Mods/ALBEDO/Variant', newPath: 'E:/Mods/ALBEDO/DISABLED Variant' }],
      2_000,
    );

    const shouldToast = shouldShowSelectionReconciliationToast(
      {
        selectedObjectFolderPath: 'ALBEDO',
        explorerSubPath: 'ALBEDO',
        selectedModPath: 'E:/Mods/ALBEDO/DISABLED Variant',
      },
      {
        selected_object_folder_path: 'ALBEDO',
        explorer_sub_path: 'ALBEDO',
        selected_mod_path: 'E:/Mods/ALBEDO/DISABLED Variant',
        current_path: ['ALBEDO'],
        reconciliation_status: 'fallback',
        reconciliation_reason: 'missing_mod_path',
        affected_paths: ['E:/Mods/ALBEDO/Variant'],
      },
      2_500,
    );

    expect(shouldToast).toBe(false);
  });

  it('deduplicates the same selection reconciliation effect across workspace consumers', () => {
    const effectKey = {
      gameId: 'genshin',
      safeMode: false,
      selection: {
        selected_object_folder_path: 'ALBEDO',
        explorer_sub_path: 'ALBEDO',
        selected_mod_path: 'E:/Mods/ALBEDO/DISABLED Variant',
        current_path: ['ALBEDO'],
        reconciliation_status: 'fallback' as const,
        reconciliation_reason: 'missing_mod_path' as const,
        affected_paths: ['E:/Mods/ALBEDO/Variant'],
      },
    };

    expect(shouldRunSelectionReconciliationEffect(effectKey)).toBe(true);
    expect(shouldRunSelectionReconciliationEffect(effectKey)).toBe(false);
  });

  it('deduplicates reconciliation effects even when workspace consumers use separate query keys', () => {
    const selection = {
      selected_object_folder_path: 'ALBEDO',
      explorer_sub_path: 'ALBEDO',
      selected_mod_path: 'E:/Mods/ALBEDO/DISABLED Variant',
      current_path: ['ALBEDO'],
      reconciliation_status: 'fallback' as const,
      reconciliation_reason: 'missing_mod_path' as const,
      affected_paths: ['E:/Mods/ALBEDO/Variant'],
    };

    expect(
      shouldRunSelectionReconciliationEffect({
        gameId: 'genshin',
        safeMode: false,
        selection,
      }),
    ).toBe(true);
    expect(
      shouldRunSelectionReconciliationEffect({
        gameId: 'genshin',
        safeMode: false,
        selection,
      }),
    ).toBe(false);
  });
});
