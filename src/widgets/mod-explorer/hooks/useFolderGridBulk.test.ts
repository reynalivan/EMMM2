import { createElement, type ReactNode } from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ModFolder } from '@/entities/game-object';
import { thumbnailKeys } from '@/entities/mod';
import type { BulkResult } from '@/shared/api/tauri/bindings.gen';
import {
  WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT,
  type WorkspaceExplorerSelectionModel,
} from '@/entities/workspace';
import { workspaceKeys } from '@/features/workspace-runtime';
import { useFolderGridBulk } from './useFolderGridBulk';

vi.unmock('@tanstack/react-query');

const {
  applyRuntimeMutationResult,
  buildQueryRemovalDescriptor,
  executeWorkspaceExplorerBulk,
  bulkToggleMods,
  publishCollectionReferenceImpact,
  setFolderBulkPendingDesired,
  clearFolderBulkPendingDesired,
  showWorkspaceRenameConflictDialog,
  admitWorkspaceIntentOverride,
} = vi.hoisted(() => ({
  applyRuntimeMutationResult: vi.fn().mockResolvedValue(undefined),
  buildQueryRemovalDescriptor: vi.fn(() => ({ events: [] })),
  executeWorkspaceExplorerBulk: vi.fn(),
  bulkToggleMods: vi.fn(),
  publishCollectionReferenceImpact: vi.fn().mockResolvedValue(undefined),
  setFolderBulkPendingDesired: vi.fn(),
  clearFolderBulkPendingDesired: vi.fn(),
  showWorkspaceRenameConflictDialog: vi.fn().mockResolvedValue(true),
  admitWorkspaceIntentOverride: vi.fn(),
}));
const { toastError, toastSuccess } = vi.hoisted(() => ({
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
}));

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: { id: 'game-1' } }),
}));

vi.mock('@/app/store', () => ({
  useAppStore: {
    getState: () => ({ activeGameId: 'game-1' }),
  },
}));

vi.mock('@/shared/api/tauri/bindings', async (importOriginal) => {
  const original = await importOriginal<typeof import('@/shared/api/tauri/bindings')>();
  return {
    ...original,
    commands: { executeWorkspaceExplorerBulk, bulkToggleMods },
  };
});

vi.mock('@/features/workspace-runtime', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/features/workspace-runtime')>()),
  applyRuntimeEffects: vi.fn(),
  admitWorkspaceIntentOverride,
  applyRuntimeMutationResult,
  buildQueryRemovalDescriptor,
  buildWorkspacePathRewritesDescriptor: vi.fn(() => ({ events: [] })),
  publishCollectionReferenceImpact,
  ensureWorkspaceProjectionListener: vi.fn().mockResolvedValue(true),
  waitForWorkspaceProjection: vi.fn().mockResolvedValue(undefined),
  setFolderBulkPendingDesired,
  clearFolderBulkPendingDesired,
  showWorkspaceRenameConflictDialog,
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    error: toastError,
    success: toastSuccess,
    info: vi.fn(),
  },
}));

const explorerQuery = {
  game_id: 'game-1',
  explorer_sub_path: 'Alice',
  search_query: null,
  sort_field: 'name' as const,
  sort_order: 'asc' as const,
  safety_filter: 'all' as const,
};

const bulkResult: BulkResult = {
  success: ['C:/Mods/Alice/Blue'],
  failures: [],
  cancelled: false,
  processed_count: 1,
  unprocessed_count: 0,
  collection_impact: {
    affected_collection_count: 0,
    affected_collection_names: [],
    rewritten_paths: [],
    missing_paths: [],
  },
  path_rewrites: [],
  sync_warning: null,
  runtime_sync_generation: null,
  disk_revision: null,
};

const defaultQueryClient = new QueryClient();

function wrapper({ children }: { children: ReactNode }) {
  return createElement(QueryClientProvider, { client: defaultQueryClient }, children);
}

function folder(path: string): ModFolder {
  return { path, name: path, folder_name: path } as ModFolder;
}

describe('useFolderGridBulk', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    defaultQueryClient.clear();
    executeWorkspaceExplorerBulk.mockReset();
    bulkToggleMods.mockReset();
    applyRuntimeMutationResult.mockReset();
    executeWorkspaceExplorerBulk.mockResolvedValue(bulkResult);
    bulkToggleMods.mockResolvedValue(bulkResult);
    applyRuntimeMutationResult.mockResolvedValue(undefined);
  });

  it('sends the typed selection and query directly to the backend bulk command', async () => {
    const options = {
      selection: {
        mode: 'all_matching' as const,
        query: explorerQuery,
        listingRevision: 'revision-1',
        excludedPaths: new Set(['C:/Mods/Alice/Red']),
        totalMatching: WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT,
      },
      explorerQuery,
      listingRevision: 'revision-1',
      sortedFolders: [],
      clearGridSelection: vi.fn(),
      removeGridSelectionPaths: vi.fn(),
      openMoveDialog: vi.fn(),
    };
    const { result } = renderHook((props) => useFolderGridBulk(props), {
      initialProps: options,
      wrapper,
    });

    act(() => result.current.handleBulkToggle(true));

    expect(setFolderBulkPendingDesired).toHaveBeenCalledWith(
      'game-1',
      expect.any(Array),
      true,
      expect.any(Number),
    );

    await waitFor(() => {
      expect(executeWorkspaceExplorerBulk).toHaveBeenCalledWith(
        {
          selection: {
            query: explorerQuery,
            listing_revision: 'revision-1',
            selection: {
              mode: 'all_matching',
              excluded_paths: ['C:/Mods/Alice/Red'],
            },
          },
          action: {
            kind: 'toggle',
            enable: true,
            operation_id: expect.stringMatching(/^toggle-/),
          },
        },
        expect.any(Number),
      );
    });
  });

  it('does not submit a selection while its raw search is ahead of the snapshot', () => {
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths: new Set(['C:/Mods/Alice/Blue']) },
          explorerQuery,
          listingRevision: 'revision-1',
          selectionStable: false,
          sortedFolders: [],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkToggle(false));

    expect(executeWorkspaceExplorerBulk).not.toHaveBeenCalled();
  });

  it('suppresses duplicate bulk submits before React publishes pending state', async () => {
    let finish: ((value: typeof bulkResult) => void) | undefined;
    executeWorkspaceExplorerBulk.mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths: new Set(['C:/Mods/Alice/Blue']) },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => {
      result.current.handleBulkToggle(true);
      result.current.handleBulkToggle(true);
    });
    await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1));
    expect(admitWorkspaceIntentOverride).toHaveBeenCalledWith(
      'game-1',
      [{ kind: 'mod_path', value: 'C:/Mods/Alice/Blue' }],
      expect.any(Number),
    );
    expect(executeWorkspaceExplorerBulk).toHaveBeenCalledWith(
      expect.objectContaining({
        selection: {
          query: explorerQuery,
          listing_revision: 'revision-1',
          selection: { mode: 'explicit', paths: ['C:/Mods/Alice/Blue'] },
        },
      }),
      expect.any(Number),
    );

    await act(async () => finish?.(bulkResult));
  });

  it('submits the latest opposite toggle after an in-flight toggle completes', async () => {
    let finishFirst!: (value: typeof bulkResult) => void;
    executeWorkspaceExplorerBulk
      .mockImplementationOnce(
        () =>
          new Promise<typeof bulkResult>((resolve) => {
            finishFirst = resolve;
          }),
      )
      .mockResolvedValueOnce(bulkResult);
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths: new Set(['C:/Mods/Alice/Blue']) },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => {
      result.current.handleBulkToggle(true);
      result.current.handleBulkToggle(false);
    });
    await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1));

    await act(async () => finishFirst(bulkResult));
    await waitFor(() => expect(bulkToggleMods).toHaveBeenCalledTimes(1));
    expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1);
    expect(executeWorkspaceExplorerBulk.mock.calls[0][0].action.enable).toBe(true);
    expect(bulkToggleMods).toHaveBeenCalledWith(
      'game-1',
      ['C:/Mods/Alice/Blue'],
      false,
      expect.any(String),
      expect.any(Number),
    );
  });

  it('continues a rapid bulk toggle using the renamed physical path', async () => {
    let finishFirst!: (value: typeof bulkResult) => void;
    executeWorkspaceExplorerBulk.mockImplementationOnce(
      () =>
        new Promise<typeof bulkResult>((resolve) => {
          finishFirst = resolve;
        }),
    );
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths: new Set(['C:/Mods/Alice/Blue']) },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => {
      result.current.handleBulkToggle(false);
      result.current.handleBulkToggle(true);
    });
    await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1));
    await act(async () =>
      finishFirst({
        ...bulkResult,
        success: ['C:/Mods/Alice/DISABLED Blue'],
        path_rewrites: [
          { old_path: 'C:/Mods/Alice/Blue', new_path: 'C:/Mods/Alice/DISABLED Blue' },
        ],
      }),
    );

    await waitFor(() =>
      expect(bulkToggleMods).toHaveBeenCalledWith(
        'game-1',
        ['C:/Mods/Alice/DISABLED Blue'],
        true,
        expect.any(String),
        expect.any(Number),
      ),
    );
  });

  it('retries the latest bulk intent for a path that failed in the earlier pass', async () => {
    let finishFirst!: (value: typeof bulkResult) => void;
    executeWorkspaceExplorerBulk.mockImplementationOnce(
      () =>
        new Promise<typeof bulkResult>((resolve) => {
          finishFirst = resolve;
        }),
    );
    bulkToggleMods.mockResolvedValueOnce(bulkResult);
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths: new Set(['C:/Mods/Alice/Blue']) },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => {
      result.current.handleBulkToggle(true);
      result.current.handleBulkToggle(false);
    });
    await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1));
    await act(async () =>
      finishFirst({
        ...bulkResult,
        success: [],
        failures: [{ path: 'C:/Mods/Alice/Blue', error: { type: 'Io', payload: 'path is busy' } }],
      }),
    );
    await waitFor(() =>
      expect(bulkToggleMods).toHaveBeenCalledWith(
        'game-1',
        ['C:/Mods/Alice/Blue'],
        false,
        expect.any(String),
        expect.any(Number),
      ),
    );
  });

  it('releases the bulk control before its background refresh completes', async () => {
    let finishRefresh!: () => void;
    applyRuntimeMutationResult.mockReturnValue(
      new Promise<void>((resolve) => {
        finishRefresh = resolve;
      }),
    );
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths: new Set(['C:/Mods/Alice/Blue']) },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkToggle(true));

    await waitFor(() => {
      expect(applyRuntimeMutationResult).toHaveBeenCalledWith(
        expect.any(QueryClient),
        'folderSwitch',
      );
    });
    try {
      expect(result.current.bulkMutationPending).toBe(false);
    } finally {
      finishRefresh();
    }
  });

  it('keeps the disk-backed toggle overlay when its background refresh fails', async () => {
    applyRuntimeMutationResult.mockRejectedValue(new Error('projection refresh failed'));
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths: new Set(['C:/Mods/Alice/Blue']) },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkToggle(true));

    await waitFor(() => expect(applyRuntimeMutationResult).toHaveBeenCalled());
    await waitFor(() => expect(result.current.bulkMutationPending).toBe(false));
    expect(clearFolderBulkPendingDesired).not.toHaveBeenCalled();
  });

  it('refreshes the exact listing and clears selection when its revision expires', async () => {
    executeWorkspaceExplorerBulk.mockRejectedValue({ type: 'ExplorerSnapshotExpired' });
    const queryClient = new QueryClient();
    const resetQueries = vi.spyOn(queryClient, 'resetQueries');
    const clearGridSelection = vi.fn();
    const queryWrapper = ({ children }: { children: ReactNode }) =>
      createElement(QueryClientProvider, { client: queryClient }, children);
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: {
            mode: 'all_matching',
            query: explorerQuery,
            listingRevision: 'expired-revision',
            excludedPaths: new Set(),
            totalMatching: WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT,
          },
          explorerQuery,
          listingRevision: 'expired-revision',
          sortedFolders: [],
          clearGridSelection,
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper: queryWrapper },
    );

    act(() => result.current.handleBulkToggle(true));

    await waitFor(() => {
      expect(clearGridSelection).toHaveBeenCalledTimes(1);
      expect(resetQueries).toHaveBeenCalledWith({
        queryKey: workspaceKeys.explorerPages(explorerQuery),
        exact: true,
      });
    });
  });

  it('opens folder conflict handling only for an actual rename collision', async () => {
    const collision = {
      type: 'Io',
      payload: JSON.stringify({
        type: 'RenameConflict',
        attempted_target: 'C:/Mods/Alice/Blue',
        existing_path: 'C:/Mods/Alice/Blue',
        base_name: 'Blue',
      }),
    };
    executeWorkspaceExplorerBulk.mockResolvedValueOnce({
      ...bulkResult,
      success: [],
      failures: [{ path: 'C:/Mods/Alice/DISABLED Blue', error: collision }],
    });
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths: new Set(['C:/Mods/Alice/DISABLED Blue']) },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkToggle(true));
    await waitFor(() =>
      expect(showWorkspaceRenameConflictDialog).toHaveBeenCalledWith('game-1', collision),
    );
    expect(toastError).not.toHaveBeenCalled();
  });

  it('blocks selections above the backend bulk limit before command submission', async () => {
    const paths = new Set(
      Array.from(
        { length: WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT + 1 },
        (_, index) => `C:/Mods/Alice/${index}`,
      ),
    );
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkToggle(true));

    await waitFor(() => {
      expect(executeWorkspaceExplorerBulk).not.toHaveBeenCalled();
      expect(toastError).toHaveBeenCalledWith(expect.stringContaining('10,000'));
    });
  });

  it('blocks all-matching and incompletely loaded selections from bulk move', () => {
    const openMoveDialog = vi.fn();
    const initialSelection: WorkspaceExplorerSelectionModel = {
      mode: 'all_matching',
      query: explorerQuery,
      listingRevision: 'revision-1',
      excludedPaths: new Set<string>(),
      totalMatching: 2,
    };
    const { result } = renderHook(
      ({ selection }: { selection: WorkspaceExplorerSelectionModel }) =>
        useFolderGridBulk({
          selection,
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [folder('C:/Mods/Alice/Blue')],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog,
        }),
      {
        initialProps: { selection: initialSelection },
        wrapper,
      },
    );

    act(() => result.current.handleBulkMoveToObject());
    expect(openMoveDialog).not.toHaveBeenCalled();
    expect(toastError).toHaveBeenCalledWith(expect.stringContaining('full exact selection'));

    const { result: explicitResult } = renderHook(
      () =>
        useFolderGridBulk({
          selection: {
            mode: 'explicit',
            paths: new Set(['C:/Mods/Alice/Blue', 'C:/Mods/Alice/Red']),
          },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [folder('C:/Mods/Alice/Blue')],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog,
        }),
      { wrapper },
    );
    act(() => explicitResult.current.handleBulkMoveToObject());

    expect(openMoveDialog).not.toHaveBeenCalled();
    expect(toastError).toHaveBeenLastCalledWith(expect.stringContaining('full exact selection'));
  });

  it('opens bulk move only with the full exact explicit path set', async () => {
    const openMoveDialog = vi.fn();
    const paths = ['C:/Mods/Alice/Blue', 'C:/Mods/Alice/Red'];
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths: new Set(paths) },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: paths.map(folder),
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog,
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkMoveToObject());

    expect(openMoveDialog).toHaveBeenCalledWith(expect.objectContaining({ path: paths[0] }));
    await waitFor(() => expect(result.current.bulkMovePaths).toEqual(paths));
  });

  it('submits the selection snapshot shown when the bulk move dialog opened', async () => {
    const shownPaths = ['C:/Mods/Alice/Blue', 'C:/Mods/Alice/Red'];
    const replacementPath = 'C:/Mods/Alice/Green';
    const { result, rerender } = renderHook(
      ({ selection, revision }) =>
        useFolderGridBulk({
          selection,
          explorerQuery,
          listingRevision: revision,
          sortedFolders: [...shownPaths, replacementPath].map(folder),
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      {
        initialProps: {
          selection: { mode: 'explicit' as const, paths: new Set(shownPaths) },
          revision: 'revision-shown',
        },
        wrapper,
      },
    );

    act(() => result.current.handleBulkMoveToObject());
    rerender({
      selection: { mode: 'explicit', paths: new Set([replacementPath]) },
      revision: 'revision-new',
    });

    await act(async () => {
      await result.current.handleBulkMoveSubmit('object-2', 'keep', null);
    });

    expect(executeWorkspaceExplorerBulk).toHaveBeenCalledWith(
      expect.objectContaining({
        selection: {
          query: explorerQuery,
          listing_revision: 'revision-shown',
          selection: { mode: 'explicit', paths: shownPaths },
        },
      }),
      null,
    );
  });

  it('reports partial move results and removes only committed source paths', async () => {
    const paths = ['C:/Mods/Alice/Blue', 'C:/Mods/Alice/Red'];
    const removeGridSelectionPaths = vi.fn();
    executeWorkspaceExplorerBulk.mockResolvedValue({
      ...bulkResult,
      success: ['Target/Blue'],
      failures: [{ path: paths[1], error: 'Destination is locked' }],
      processed_count: 2,
      path_rewrites: [{ old_path: 'Alice/Blue', new_path: 'Target/Blue' }],
    });
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths: new Set(paths) },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: paths.map(folder),
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths,
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkMoveToObject());
    await act(async () => {
      await expect(result.current.handleBulkMoveSubmit('object-2', 'keep', null)).resolves.toBe(
        undefined,
      );
    });

    expect(removeGridSelectionPaths).toHaveBeenCalledWith([paths[0]]);
    expect(buildQueryRemovalDescriptor).toHaveBeenCalledWith(
      [thumbnailKeys.folder('Alice/Blue')],
      [],
    );
    expect(toastSuccess).toHaveBeenCalledWith(expect.stringContaining('1'));
    expect(toastError).toHaveBeenCalledWith(expect.stringContaining('1'));
  });

  it('continues to invalidate successful delete paths directly', async () => {
    const deletedPath = 'C:/Mods/Alice/Blue';
    const { result } = renderHook(
      () =>
        useFolderGridBulk({
          selection: { mode: 'explicit', paths: new Set([deletedPath]) },
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [folder(deletedPath)],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkDeleteConfirm());

    await waitFor(() => {
      expect(buildQueryRemovalDescriptor).toHaveBeenCalledWith(
        [thumbnailKeys.folder(deletedPath)],
        [],
      );
    });
  });
});
