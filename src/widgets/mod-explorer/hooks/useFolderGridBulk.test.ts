import { createElement, type ReactNode } from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ModFolder } from '@/entities/game-object';
import { thumbnailKeys } from '@/entities/mod';
import type { BulkResult } from '@/shared/api/tauri/bindings.gen';
import {
  WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT,
  type WorkspaceExplorerQuery,
  type WorkspaceExplorerSelectionModel,
} from '@/entities/workspace';
import { workspaceKeys } from '@/features/workspace-runtime';
import { useFolderGridBulk } from './useFolderGridBulk';

vi.unmock('@tanstack/react-query');

const {
  applyRuntimeMutationResult,
  scheduleWorkspaceSwitchRefresh,
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
  scheduleWorkspaceSwitchRefresh: vi.fn().mockResolvedValue(undefined),
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
const activeGameId = vi.hoisted(() => ({ value: 'game-1' }));

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: { id: activeGameId.value } }),
}));

vi.mock('@/app/store', () => ({
  useAppStore: {
    getState: () => ({ activeGameId: activeGameId.value }),
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
  scheduleWorkspaceSwitchRefresh,
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
  expected_identities: [['C:/Mods/Alice/Blue', 'identity-blue']] as [string, string][],
};

const defaultQueryClient = new QueryClient();

function wrapper({ children }: { children: ReactNode }) {
  return createElement(QueryClientProvider, { client: defaultQueryClient }, children);
}

function folder(path: string): ModFolder {
  return { path, name: path, folder_name: path } as ModFolder;
}

function identifiedFolder(path: string, identity: string): ModFolder {
  return { ...folder(path), filesystem_identity: identity } as ModFolder;
}

describe('useFolderGridBulk', () => {
  beforeEach(() => {
    activeGameId.value = 'game-1';
    vi.clearAllMocks();
    defaultQueryClient.clear();
    executeWorkspaceExplorerBulk.mockReset();
    bulkToggleMods.mockReset();
    applyRuntimeMutationResult.mockReset();
    scheduleWorkspaceSwitchRefresh.mockReset().mockResolvedValue(undefined);
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
      [],
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
      [['C:/Mods/Alice/Blue', 'identity-blue']],
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
        expected_identities: [['C:/Mods/Alice/DISABLED Blue', 'identity-blue']],
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
        [['C:/Mods/Alice/DISABLED Blue', 'identity-blue']],
      ),
    );
  });

  it('uses the completed physical identity while renamed paths outpace the listing refresh', async () => {
    const renamedResult: BulkResult = {
      ...bulkResult,
      success: ['C:/Mods/Alice/DISABLED Blue'],
      expected_identities: [['C:/Mods/Alice/DISABLED Blue', 'identity-blue']],
      path_rewrites: [{ old_path: 'C:/Mods/Alice/Blue', new_path: 'C:/Mods/Alice/DISABLED Blue' }],
    };
    executeWorkspaceExplorerBulk.mockResolvedValueOnce(renamedResult);
    const selection: WorkspaceExplorerSelectionModel = {
      mode: 'explicit',
      paths: new Set(['C:/Mods/Alice/Blue']),
    };
    const { result, rerender } = renderHook(
      ({ currentSelection }: { currentSelection: WorkspaceExplorerSelectionModel }) =>
        useFolderGridBulk({
          selection: currentSelection,
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [identifiedFolder('C:/Mods/Alice/Blue', 'identity-blue')],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { initialProps: { currentSelection: selection }, wrapper },
    );

    act(() => result.current.handleBulkToggle(false));
    await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1));

    rerender({
      currentSelection: {
        mode: 'explicit',
        paths: new Set(['C:/Mods/Alice/DISABLED Blue']),
      },
    });
    act(() => result.current.handleBulkToggle(true));

    await waitFor(() => expect(bulkToggleMods).toHaveBeenCalledTimes(1));
    expect(bulkToggleMods).toHaveBeenCalledWith(
      'game-1',
      ['C:/Mods/Alice/DISABLED Blue'],
      true,
      expect.any(String),
      expect.any(Number),
      [['C:/Mods/Alice/DISABLED Blue', 'identity-blue']],
    );
    expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1);
  });

  it('maps absolute backend receipts back to relative selections with existing path rewrite semantics', async () => {
    const sourcePath = 'Alice\\Blue';
    const rewrittenPath = 'Alice\\DISABLED Blue';
    const absoluteOldPath = 'C:/Mods/Alice/Blue';
    const absoluteNewPath = 'C:/Mods/Alice/DISABLED Blue';
    executeWorkspaceExplorerBulk.mockResolvedValueOnce({
      ...bulkResult,
      success: [absoluteNewPath],
      expected_identities: [[absoluteNewPath, 'identity-blue']],
      path_rewrites: [{ old_path: absoluteOldPath, new_path: absoluteNewPath }],
    });
    let selection: WorkspaceExplorerSelectionModel = {
      mode: 'explicit',
      paths: new Set([sourcePath]),
    };
    const { result, rerender } = renderHook(
      () =>
        useFolderGridBulk({
          selection,
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [identifiedFolder(sourcePath, 'identity-blue')],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkToggle(false));
    await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(result.current.bulkMutationPending).toBe(false));

    selection = { mode: 'explicit', paths: new Set([rewrittenPath]) };
    rerender();
    act(() => result.current.handleBulkToggle(true));

    await waitFor(() => expect(bulkToggleMods).toHaveBeenCalledTimes(1));
    expect(bulkToggleMods).toHaveBeenCalledWith(
      'game-1',
      [rewrittenPath],
      true,
      expect.any(String),
      expect.any(Number),
      [[rewrittenPath, 'identity-blue']],
    );
    expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1);
  });

  it('retires a completed receipt when selection leaves and returns to its rewritten paths', async () => {
    const renamedResult: BulkResult = {
      ...bulkResult,
      success: ['C:/Mods/Alice/DISABLED Blue'],
      expected_identities: [['C:/Mods/Alice/DISABLED Blue', 'identity-blue']],
      path_rewrites: [{ old_path: 'C:/Mods/Alice/Blue', new_path: 'C:/Mods/Alice/DISABLED Blue' }],
    };
    executeWorkspaceExplorerBulk.mockResolvedValueOnce(renamedResult);
    let selection: WorkspaceExplorerSelectionModel = {
      mode: 'explicit',
      paths: new Set(['C:/Mods/Alice/Blue']),
    };
    const { result, rerender } = renderHook(
      () =>
        useFolderGridBulk({
          selection,
          explorerQuery,
          listingRevision: 'revision-1',
          sortedFolders: [identifiedFolder('C:/Mods/Alice/Blue', 'identity-blue')],
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkToggle(false));
    await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(result.current.bulkMutationPending).toBe(false));

    selection = { mode: 'explicit', paths: new Set(['C:/Mods/Alice/DISABLED Blue']) };
    rerender();
    selection = { mode: 'explicit', paths: new Set(['C:/Mods/Alice/Other']) };
    rerender();
    selection = { mode: 'explicit', paths: new Set(['C:/Mods/Alice/DISABLED Blue']) };
    rerender();
    act(() => result.current.handleBulkToggle(true));

    await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(2));
    expect(bulkToggleMods).not.toHaveBeenCalled();
  });

  it.each([
    'different query',
    'different game',
    'different explicit selection',
    'incomplete receipt',
    'replacement identity',
  ])('does not reuse a completed identity receipt for %s', async (scenario) => {
    const renamedResult: BulkResult = {
      ...bulkResult,
      success: ['C:/Mods/Alice/DISABLED Blue'],
      expected_identities:
        scenario === 'incomplete receipt' ? [] : [['C:/Mods/Alice/DISABLED Blue', 'identity-blue']],
      path_rewrites: [{ old_path: 'C:/Mods/Alice/Blue', new_path: 'C:/Mods/Alice/DISABLED Blue' }],
    };
    executeWorkspaceExplorerBulk.mockResolvedValueOnce(renamedResult);
    let query: WorkspaceExplorerQuery = explorerQuery;
    let selection: WorkspaceExplorerSelectionModel = {
      mode: 'explicit',
      paths: new Set(['C:/Mods/Alice/Blue']),
    };
    let sortedFolders = [identifiedFolder('C:/Mods/Alice/Blue', 'identity-blue')];
    const { result, rerender } = renderHook(
      () =>
        useFolderGridBulk({
          selection,
          explorerQuery: query,
          listingRevision: 'revision-1',
          sortedFolders,
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkToggle(false));
    await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(result.current.bulkMutationPending).toBe(false));

    if (scenario === 'different query') {
      query = { ...explorerQuery, search_query: 'Blue' };
    } else if (scenario === 'different game') {
      activeGameId.value = 'game-2';
      query = { ...explorerQuery, game_id: 'game-2' };
    } else if (scenario === 'different explicit selection') {
      selection = { mode: 'explicit', paths: new Set(['C:/Mods/Alice/Other']) };
    } else if (scenario === 'replacement identity') {
      sortedFolders = [identifiedFolder('C:/Mods/Alice/DISABLED Blue', 'identity-replacement')];
    }
    if (scenario !== 'different explicit selection') {
      selection = {
        mode: 'explicit',
        paths: new Set(['C:/Mods/Alice/DISABLED Blue']),
      };
    }
    rerender();
    act(() => result.current.handleBulkToggle(true));

    await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(2));
    expect(bulkToggleMods).not.toHaveBeenCalled();
  });

  it.each([
    { scenario: 'before the listing revision advances', revision: 'revision-1' },
    { scenario: 'after the listing revision advances', revision: 'revision-2' },
  ])(
    'continues an opposite toggle for the same physical selection $scenario',
    async ({ revision }) => {
      let finishFirst!: (value: typeof bulkResult) => void;
      executeWorkspaceExplorerBulk.mockImplementationOnce(
        () =>
          new Promise<typeof bulkResult>((resolve) => {
            finishFirst = resolve;
          }),
      );
      let selection: WorkspaceExplorerSelectionModel = {
        mode: 'explicit',
        paths: new Set(['C:/Mods/Alice/Blue', 'C:/Mods/Alice/Green']),
      };
      let listingRevision = 'revision-1';
      let sortedFolders = [
        identifiedFolder('C:/Mods/Alice/Blue', 'identity-blue'),
        identifiedFolder('C:/Mods/Alice/Green', 'identity-green'),
      ];
      const { result, rerender } = renderHook(
        () =>
          useFolderGridBulk({
            selection,
            explorerQuery,
            listingRevision,
            sortedFolders,
            clearGridSelection: vi.fn(),
            removeGridSelectionPaths: vi.fn(),
            openMoveDialog: vi.fn(),
          }),
        { wrapper },
      );

      act(() => result.current.handleBulkToggle(false));
      await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1));

      selection = {
        mode: 'explicit',
        paths: new Set(['C:/Mods/Alice/DISABLED Blue', 'C:/Mods/Alice/DISABLED Green']),
      };
      listingRevision = revision;
      sortedFolders = [
        identifiedFolder('C:/Mods/Alice/DISABLED Blue', 'identity-blue'),
        identifiedFolder('C:/Mods/Alice/DISABLED Green', 'identity-green'),
      ];
      rerender();
      act(() => result.current.handleBulkToggle(true));

      expect(admitWorkspaceIntentOverride).toHaveBeenCalledTimes(1);
      expect(admitWorkspaceIntentOverride).toHaveBeenCalledWith(
        'game-1',
        [
          {
            kind: 'mod_path',
            value: 'C:/Mods/Alice/DISABLED Blue',
            expected_identity: 'identity-blue',
          },
          {
            kind: 'mod_path',
            value: 'C:/Mods/Alice/DISABLED Green',
            expected_identity: 'identity-green',
          },
        ],
        expect.any(Number),
      );

      await act(async () =>
        finishFirst({
          ...bulkResult,
          success: ['C:/Mods/Alice/DISABLED Blue', 'C:/Mods/Alice/DISABLED Green'],
          processed_count: 2,
          expected_identities: [
            ['C:/Mods/Alice/DISABLED Blue', 'identity-blue'],
            ['C:/Mods/Alice/DISABLED Green', 'identity-green'],
          ],
          path_rewrites: [
            { old_path: 'C:/Mods/Alice/Blue', new_path: 'C:/Mods/Alice/DISABLED Blue' },
            { old_path: 'C:/Mods/Alice/Green', new_path: 'C:/Mods/Alice/DISABLED Green' },
          ],
        }),
      );

      await waitFor(() => expect(bulkToggleMods).toHaveBeenCalledTimes(1));
      expect(bulkToggleMods).toHaveBeenCalledWith(
        'game-1',
        ['C:/Mods/Alice/DISABLED Blue', 'C:/Mods/Alice/DISABLED Green'],
        true,
        expect.any(String),
        expect.any(Number),
        [
          ['C:/Mods/Alice/DISABLED Blue', 'identity-blue'],
          ['C:/Mods/Alice/DISABLED Green', 'identity-green'],
        ],
      );
    },
  );

  it('rejects a changed physical selection across listing revisions', async () => {
    let finishFirst!: (value: typeof bulkResult) => void;
    executeWorkspaceExplorerBulk.mockImplementationOnce(
      () =>
        new Promise<typeof bulkResult>((resolve) => {
          finishFirst = resolve;
        }),
    );
    let selection: WorkspaceExplorerSelectionModel = {
      mode: 'explicit',
      paths: new Set(['C:/Mods/Alice/Blue']),
    };
    let listingRevision = 'revision-1';
    let sortedFolders = [identifiedFolder('C:/Mods/Alice/Blue', 'identity-blue')];
    const { result, rerender } = renderHook(
      () =>
        useFolderGridBulk({
          selection,
          explorerQuery,
          listingRevision,
          sortedFolders,
          clearGridSelection: vi.fn(),
          removeGridSelectionPaths: vi.fn(),
          openMoveDialog: vi.fn(),
        }),
      { wrapper },
    );

    act(() => result.current.handleBulkToggle(false));
    await waitFor(() => expect(executeWorkspaceExplorerBulk).toHaveBeenCalledTimes(1));
    selection = {
      mode: 'explicit',
      paths: new Set(['C:/Mods/Alice/Parent/Blue']),
    };
    listingRevision = 'revision-2';
    sortedFolders = [identifiedFolder('C:/Mods/Alice/Parent/Blue', 'identity-different')];
    rerender();

    act(() => result.current.handleBulkToggle(true));

    expect(toastError).toHaveBeenCalledTimes(1);
    expect(admitWorkspaceIntentOverride).not.toHaveBeenCalled();

    await act(async () => finishFirst(bulkResult));
    expect(bulkToggleMods).not.toHaveBeenCalled();
  });

  it('preserves offscreen physical identities when continuing an all-matching toggle', async () => {
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
          selection: {
            mode: 'all_matching',
            query: explorerQuery,
            listingRevision: 'revision-1',
            excludedPaths: new Set<string>(),
            totalMatching: 2,
          },
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
    const identities: [string, string][] = [
      ['C:/Mods/Alice/DISABLED Blue', 'identity-blue'],
      ['C:/Mods/Alice/DISABLED Offscreen', 'identity-offscreen'],
    ];
    await act(async () => finishFirst({ ...bulkResult, expected_identities: identities }));
    await waitFor(() => expect(bulkToggleMods).toHaveBeenCalledTimes(1));
    expect(bulkToggleMods).toHaveBeenCalledWith(
      'game-1',
      identities.map(([path]) => path),
      true,
      expect.any(String),
      expect.any(Number),
      identities,
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
        [['C:/Mods/Alice/Blue', 'identity-blue']],
      ),
    );
  });

  it('releases the bulk control before its background refresh completes', async () => {
    let finishRefresh!: () => void;
    scheduleWorkspaceSwitchRefresh.mockReturnValue(
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
      expect(scheduleWorkspaceSwitchRefresh).toHaveBeenCalledWith(
        expect.any(QueryClient),
        expect.objectContaining({ gameId: 'game-1', affectedPaths: bulkResult.success }),
      );
    });
    try {
      expect(result.current.bulkMutationPending).toBe(false);
    } finally {
      finishRefresh();
    }
  });

  it('keeps the disk-backed toggle overlay when its background refresh fails', async () => {
    scheduleWorkspaceSwitchRefresh.mockRejectedValue(new Error('projection refresh failed'));
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

    await waitFor(() => expect(scheduleWorkspaceSwitchRefresh).toHaveBeenCalled());
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
