import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '../../../stores/useAppStore';
import type { ModFolder } from '../../../types/mod';
import type { DuplicateInfo } from '../../../types/scanner';
import { useSharedModActions } from './useSharedModActions';

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

const bulkFavoriteMutate = vi.fn();
const renameMutateAsync = vi.fn();
const deleteMutateAsync = vi.fn();
const toggleSafeMutate = vi.fn();
const moveModToObjectAndRefresh = vi.fn();
const applyFolderDbSyncMatchAndRefresh = vi.fn();
const matchObjectWithDb = vi.fn();
const toastError = vi.fn();
const switchToggleNode = vi.fn();
const switchResolveDuplicateForceEnable = vi.fn();
const switchResolveDuplicateEnableOnly = vi.fn();

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, vars?: Record<string, unknown>) => {
      if (vars?.name && typeof vars.name === 'string') {
        return `${key}:${vars.name}`;
      }

      return key;
    },
  }),
}));

vi.mock('../../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({
    activeGame: {
      id: 'game-1',
      game_type: 'GIMI',
    },
  }),
}));

vi.mock('../../../hooks/useSettings', () => ({
  useSettings: () => ({
    settings: {
      safe_mode: {
        pin_hash: '1234',
      },
    },
  }),
}));

vi.mock('../../../hooks/folderCache', () => ({
  updateFolderCache: vi.fn(),
}));

vi.mock('../../../hooks/useFolderMutations', () => ({
  useBulkFavorite: () => ({
    mutate: bulkFavoriteMutate,
  }),
  useToggleModSafe: () => ({
    mutate: toggleSafeMutate,
  }),
}));

vi.mock('../../../hooks/useFolderCoreMutations', () => ({
  useRenameMod: () => ({
    mutateAsync: renameMutateAsync,
  }),
  useDeleteMod: () => ({
    mutateAsync: deleteMutateAsync,
  }),
}));

vi.mock('../../workspace-runtime/actions/useWorkspaceSwitchActions', () => ({
  useWorkspaceSwitchActions: () => ({
    isPending: false,
    isNodePending: vi.fn(() => false),
    toggleNode: (...args: unknown[]) => switchToggleNode(...args),
    setNodeEnabled: vi.fn(),
    setFolderPathEnabled: vi.fn(),
    resolveDuplicateForceEnable: (...args: unknown[]) => switchResolveDuplicateForceEnable(...args),
    resolveDuplicateEnableOnly: (...args: unknown[]) => switchResolveDuplicateEnableOnly(...args),
  }),
}));

vi.mock('../operations/sharedOperations', () => ({
  moveModToObjectAndRefresh: (...args: unknown[]) => moveModToObjectAndRefresh(...args),
  applyFolderDbSyncMatchAndRefresh: (...args: unknown[]) =>
    applyFolderDbSyncMatchAndRefresh(...args),
}));

vi.mock('../../../lib/bindings', () => ({
  commands: {
    matchObjectWithDb: (...args: unknown[]) => matchObjectWithDb(...args),
    toggleModSafe: vi.fn(),
  },
}));

vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    success: vi.fn(),
    error: (...args: unknown[]) => toastError(...args),
    withAction: vi.fn(),
  },
}));

function createWrapper(queryClient: QueryClient) {
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

function createFolder(overrides: Partial<ModFolder> = {}): ModFolder {
  return {
    node_type: 'FlatModRoot',
    classification_reasons: [],
    id: 'folder-1',
    owner_object_id: 'object-1',
    owner_object_folder_path: 'Objects/Alpha',
    name: 'Alpha Mod',
    folder_name: 'Alpha Mod',
    path: 'Objects/Alpha/Alpha Mod',
    is_enabled: false,
    is_directory: true,
    thumbnail_path: null,
    modified_at: 0,
    size_bytes: 0,
    has_info_json: false,
    is_favorite: false,
    is_misplaced: false,
    is_safe: true,
    metadata: null,
    category: 'Character',
    conflict_group_id: null,
    conflict_state: null,
    pin_hash: null,
    warnings: [],
    ...overrides,
  };
}

function createDuplicate(): DuplicateInfo {
  return {
    mod_id: 'dup-1',
    object_id: 'object-1',
    folder_path: 'Objects/Alpha/Other Mod',
    actual_name: 'Other Mod',
    is_variant: false,
    parent_path: 'Objects/Alpha',
  };
}

describe('useSharedModActions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      safeMode: true,
      gridSelection: new Set(),
      workspaceDialogState: { kind: 'none' },
      workspacePreviewDirty: false,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
    });
    matchObjectWithDb.mockResolvedValue({ id: 'db-1', name: 'Alpha' });
    moveModToObjectAndRefresh.mockResolvedValue(undefined);
    applyFolderDbSyncMatchAndRefresh.mockResolvedValue(undefined);
    renameMutateAsync.mockResolvedValue(undefined);
    deleteMutateAsync.mockResolvedValue(undefined);
  });

  it('opens move dialog through workspace runtime state', () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { result } = renderHook(() => useSharedModActions(), {
      wrapper: createWrapper(queryClient),
    });
    const folder = createFolder();

    act(() => {
      result.current.openMoveDialog(folder);
    });

    expect(useAppStore.getState().workspaceDialogState).toEqual({
      kind: 'modMove',
      folder,
    });
  });

  it('enables a mod directly when duplicate check is clean', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { result } = renderHook(() => useSharedModActions(), {
      wrapper: createWrapper(queryClient),
    });
    const folder = createFolder({ is_enabled: false });

    await act(async () => {
      await result.current.handleToggleEnabled(folder);
    });

    expect(switchToggleNode).toHaveBeenCalledWith(folder, 'folder_grid', {
      syncExplorerPath: false,
    });
  });

  it('routes duplicate resolution to the shared switch engine', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const folder = createFolder({ is_enabled: false });
    const duplicates = [createDuplicate()];
    useAppStore.setState({
      workspaceDialogState: {
        kind: 'modDuplicateWarning',
        folder,
        duplicates,
      },
    });
    const { result } = renderHook(() => useSharedModActions(), {
      wrapper: createWrapper(queryClient),
    });

    act(() => {
      result.current.handleDuplicateForceEnable();
      result.current.handleDuplicateEnableOnly();
    });

    expect(switchResolveDuplicateForceEnable).toHaveBeenCalledWith(folder);
    expect(switchResolveDuplicateEnableOnly).toHaveBeenCalledWith(folder);
  });

  it('rejects invalid rename input without calling the mutation', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const folder = createFolder();
    useAppStore.setState({
      workspaceDialogState: {
        kind: 'modRename',
        folder,
      },
    });

    const { result } = renderHook(() => useSharedModActions(), {
      wrapper: createWrapper(queryClient),
    });

    await act(async () => {
      await result.current.handleRenameSubmit('bad:name');
    });

    expect(renameMutateAsync).not.toHaveBeenCalled();
    expect(toastError).toHaveBeenCalledWith('objects:edit_modal.validation.path_invalid');
  });

  it('loads sync match into runtime dialog state', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { result } = renderHook(() => useSharedModActions(), {
      wrapper: createWrapper(queryClient),
    });
    const folder = createFolder();

    await act(async () => {
      await result.current.handleSyncWithDb(folder);
    });

    await waitFor(() => {
      const dialogState = useAppStore.getState().workspaceDialogState;
      expect(dialogState.kind).toBe('modSync');
      if (dialogState.kind !== 'modSync') {
        throw new Error('Expected mod sync dialog');
      }
      expect(dialogState.folder).toEqual(folder);
      expect(dialogState.isLoading).toBe(false);
      expect(dialogState.match).toEqual({ id: 'db-1', name: 'Alpha' });
    });
  });
});
