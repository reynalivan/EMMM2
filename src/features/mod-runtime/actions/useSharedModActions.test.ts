import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/app/store';
import type { ModFolder } from '@/entities/game-object';
import type { DuplicateInfo } from '@/entities/workspace';
import { useSharedModActions } from './useSharedModActions';

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

const bulkFavoriteMutate = vi.fn();
const renameMutateAsync = vi.fn();
const deleteMutateAsync = vi.fn();
const toggleSafeMutate = vi.fn();
const openObjectClassificationWizard = vi.fn();
const toastError = vi.fn();
const switchToggleNode = vi.fn();
const switchResolveDuplicateForceEnable = vi.fn();
const switchResolveDuplicateEnableOnly = vi.fn();

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: () => undefined },
  useTranslation: () => ({
    t: (key: string, vars?: Record<string, unknown>) => {
      if (vars?.name && typeof vars.name === 'string') {
        return `${key}:${vars.name}`;
      }

      return key;
    },
  }),
}));

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({
    activeGame: {
      id: 'game-1',
      game_type: 'GIMI',
    },
  }),
}));

vi.mock('@/widgets/mod-explorer/hooks/folderCache', () => ({
  updateFolderCache: vi.fn(),
}));

vi.mock('../hooks/useBulkModMutations', () => ({
  useBulkFavorite: () => ({
    mutate: bulkFavoriteMutate,
  }),
}));

vi.mock('../hooks/useFolderMutations', () => ({
  useToggleModSafe: () => ({
    mutate: toggleSafeMutate,
  }),
}));

vi.mock('../hooks/useFolderCoreMutations', () => ({
  useRenameMod: () => ({
    mutateAsync: renameMutateAsync,
  }),
  useDeleteMod: () => ({
    mutateAsync: deleteMutateAsync,
  }),
}));

vi.mock('@/features/workspace-runtime/@x/mod-runtime', async (importOriginal) => {
  const actual =
    await importOriginal<typeof import('@/features/workspace-runtime/@x/mod-runtime')>();
  return {
    ...actual,
    useWorkspaceSwitchActions: () => ({
      isPending: false,
      isNodePending: vi.fn(() => false),
      toggleNode: (...args: unknown[]) => switchToggleNode(...args),
      setNodeEnabled: vi.fn(),
      setFolderPathEnabled: vi.fn(),
      resolveDuplicateForceEnable: (...args: unknown[]) =>
        switchResolveDuplicateForceEnable(...args),
      resolveDuplicateEnableOnly: (...args: unknown[]) => switchResolveDuplicateEnableOnly(...args),
    }),
  };
});

vi.mock('@/features/import-batches/@x/mod-runtime', () => ({
  openObjectClassificationWizard: (...args: unknown[]) => openObjectClassificationWizard(...args),
}));

vi.mock('../../../shared/api/tauri/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    toggleModSafe: vi.fn(),
  },
}));

vi.mock('@/shared/ui/toast', () => ({
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
    is_safety_classified: true,
    contains_safe_mods: true,
    contains_unsafe_mods: false,
    metadata: null,
    category: 'Character',
    conflict_group_id: null,
    conflict_state: null,
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
      gridSelection: new Set(),
      workspaceDialogState: { kind: 'none' },
      workspacePreviewDirty: false,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
    });
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

    expect(switchToggleNode).toHaveBeenCalledWith(folder, 'folder_grid');
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

  it('opens the shared classification wizard for the owning object', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { result } = renderHook(() => useSharedModActions(), {
      wrapper: createWrapper(queryClient),
    });
    const folder = createFolder();

    await act(async () => {
      await result.current.handleSyncWithDb(folder);
    });

    expect(openObjectClassificationWizard).toHaveBeenCalledWith({
      gameId: 'game-1',
      objectIds: ['object-1'],
    });
  });
});
