import { beforeEach, describe, expect, it } from 'vitest';
import {
  buildWorkspacePreviewInput,
  buildWorkspaceStructureInput,
  buildWorkspaceViewModelFilter,
  previewRequestIdentityMatches,
  shouldApplyWorkspacePreviewSelection,
  workspaceKeys,
} from './useWorkspaceViewModel';
import {
  buildSelectionReconciledEvent,
  recordInternalWorkspacePathRewrites,
  resetWorkspaceSelectionReconciliationGuardsForTest,
  shouldApplySelectionReconciledEvent,
  shouldRunSelectionReconciliationEffect,
} from '../utils/selectionReconciliation';

describe('workspace structure and preview query contract', () => {
  beforeEach(() => {
    resetWorkspaceSelectionReconciliationGuardsForTest();
  });

  it('builds workspace filter from app runtime inputs', () => {
    const filter = buildWorkspaceViewModelFilter({
      gameId: 'game-1',
      selectedObjectType: 'Character',
      objectMetaFilters: { element: ['Pyro'] },
      objectSortBy: 'name',
      objectStatusFilter: 'enabled',
    });

    expect(filter).toEqual({
      game_id: 'game-1',
      object_type: 'Character',
      search_query: null,
      meta_filters: { element: ['Pyro'] },
      sort_by: 'name',
      status_filter: 1,
    });
  });

  it('builds structure input without a selected mod path', () => {
    const filter = buildWorkspaceViewModelFilter({
      gameId: 'game-1',
      selectedObjectType: 'Character',
      objectMetaFilters: { element: ['Pyro'] },
      objectSortBy: 'name',
      objectStatusFilter: 'enabled',
    });

    const input = buildWorkspaceStructureInput(filter, {
      selectedObjectFolderPath: 'Objects/Diluc',
      explorerSubPath: 'Objects/Diluc/Variants',
    });

    expect(input).toEqual({
      filter: {
        game_id: 'game-1',
        object_type: 'Character',
        search_query: null,
        meta_filters: { element: ['Pyro'] },
        sort_by: 'name',
        status_filter: 1,
      },
      selected_object_folder_path: 'Objects/Diluc',
      explorer_sub_path: 'Objects/Diluc/Variants',
    });
  });

  it('keeps the structure key independent from preview selection', () => {
    const structureKey = workspaceKeys.structure(
      {
        game_id: 'game-1',
        object_type: 'Character',
        search_query: null,
        meta_filters: { element: ['Pyro'] },
        sort_by: 'name',
        status_filter: 1,
      },
      'Objects/Diluc',
      'Objects/Diluc/Variants',
    );
    const firstPreviewKey = workspaceKeys.preview(
      'game-1',
      'Objects/Diluc/Variants',
      'E:/Mods/Objects/Diluc/Variants/A',
    );
    const secondPreviewKey = workspaceKeys.preview(
      'game-1',
      'Objects/Diluc/Variants',
      'E:/Mods/Objects/Diluc/Variants/B',
    );

    expect(structureKey).toEqual([
      'workspace',
      'mods',
      'structure',
      {
        game_id: 'game-1',
        object_type: 'Character',
        search_query: null,
        meta_filters: { element: ['Pyro'] },
        sort_by: 'name',
        status_filter: 1,
      },
      'Objects/Diluc',
      'Objects/Diluc/Variants',
    ]);
    expect(firstPreviewKey).not.toEqual(secondPreviewKey);
    expect(firstPreviewKey.slice(0, 2)).toEqual(workspaceKeys.all);
    expect(secondPreviewKey.slice(0, 2)).toEqual(workspaceKeys.all);
  });

  it('builds preview input only from the selected preview context', () => {
    expect(
      buildWorkspacePreviewInput({
        gameId: 'game-1',
        explorerSubPath: 'Objects/Diluc/Variants',
        selectedModPath: 'E:/Mods/Objects/Diluc/Variants/A',
      }),
    ).toEqual({
      game_id: 'game-1',
      explorer_sub_path: 'Objects/Diluc/Variants',
      selected_mod_path: 'E:/Mods/Objects/Diluc/Variants/A',
    });
  });

  it('accepts a preview response only for its current request identity', () => {
    const request = {
      gameId: 'game-1',
      explorerSubPath: 'Objects/Diluc/Variants',
      selectedModPath: 'E:/Mods/Objects/Diluc/Variants/A',
    };

    expect(
      previewRequestIdentityMatches(request, {
        game_id: 'game-1',
        explorer_sub_path: 'Objects/Diluc/Variants',
        selected_mod_path: 'e:/mods/objects/diluc/variants/a',
      }),
    ).toBe(true);
    expect(
      previewRequestIdentityMatches(request, {
        game_id: 'game-1',
        explorer_sub_path: 'Objects/Diluc/Variants',
        selected_mod_path: 'E:/Mods/Objects/Diluc/Variants/B',
      }),
    ).toBe(false);

    expect(
      previewRequestIdentityMatches(
        {
          gameId: 'game-1',
          explorerSubPath: undefined,
          selectedModPath: 'E:/Mods/Standalone',
        },
        {
          game_id: 'game-1',
          explorer_sub_path: null,
          selected_mod_path: 'E:/Mods/Standalone',
        },
      ),
    ).toBe(true);
  });

  it('rejects stale-context preview reconciliation before it can clear selection', () => {
    const request = {
      gameId: 'game-1',
      explorerSubPath: 'Objects/Diluc/Variants',
      selectedModPath: 'E:/Mods/Objects/Diluc/Variants/A',
    };
    const currentSelection = {
      selectedObjectFolderPath: 'Objects/Diluc',
      explorerSubPath: 'Objects/Diluc/Variants',
      selectedModPath: 'E:/Mods/Objects/Diluc/Variants/A',
    };

    expect(
      shouldApplyWorkspacePreviewSelection(
        request,
        currentSelection,
        ['Objects', 'Diluc', 'Variants'],
        {
          game_id: 'game-1',
          explorer_sub_path: 'Objects/Diluc/Variants',
          selected_mod_path: 'E:/Mods/Objects/Diluc/Variants/A',
        },
        'context_stale',
        {
          selected_mod_path: null,
          reconciliation_status: 'cleared',
          reconciliation_reason: 'missing_mod_path',
          affected_paths: ['E:/Mods/Objects/Diluc/Variants/A'],
        },
      ),
    ).toBe(false);
  });

  it('rejects an out-of-order preview response for a previous mod', () => {
    const currentSelection = {
      selectedObjectFolderPath: 'Objects/Diluc',
      explorerSubPath: 'Objects/Diluc/Variants',
      selectedModPath: 'E:/Mods/Objects/Diluc/Variants/B',
    };

    expect(
      shouldApplyWorkspacePreviewSelection(
        {
          gameId: 'game-1',
          explorerSubPath: 'Objects/Diluc/Variants',
          selectedModPath: 'E:/Mods/Objects/Diluc/Variants/B',
        },
        currentSelection,
        ['Objects', 'Diluc', 'Variants'],
        {
          game_id: 'game-1',
          explorer_sub_path: 'Objects/Diluc/Variants',
          selected_mod_path: 'E:/Mods/Objects/Diluc/Variants/A',
        },
        'ready',
        {
          selected_mod_path: null,
          reconciliation_status: 'cleared',
          reconciliation_reason: 'missing_mod_path',
          affected_paths: ['E:/Mods/Objects/Diluc/Variants/A'],
        },
      ),
    ).toBe(false);
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

  it('deduplicates the same selection reconciliation effect across workspace consumers', () => {
    const effectKey = {
      gameId: 'genshin',
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
        selection,
      }),
    ).toBe(true);
    expect(
      shouldRunSelectionReconciliationEffect({
        gameId: 'genshin',
        selection,
      }),
    ).toBe(false);
  });
});
