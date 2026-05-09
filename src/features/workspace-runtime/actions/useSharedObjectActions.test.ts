import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '../../../stores/useAppStore';
import type { WorkspaceObjectNode } from '../../../types/workspace';
import { useSharedObjectActions } from './useSharedObjectActions';

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

const deleteObjectMutateAsync = vi.fn();
const runObjectBatchMutation = vi.fn();
const updateObjectMutateAsync = vi.fn();
const applyObjectCategoryAndRefresh = vi.fn();
const revealObjectInExplorer = vi.fn();
const publishRuntimeDescriptor = vi.fn();
const toastError = vi.fn();
const toastSuccess = vi.fn();
const sharedSyncHandleSyncWithDb = vi.fn();
const sharedSyncHandleApplySyncMatch = vi.fn();
const sharedSyncSetSyncConfirm = vi.fn();
const switchSetNodeEnabled = vi.fn();
const switchIsNodePending = vi.fn((_node?: unknown) => false);

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
      name: 'Game',
      game_type: 'GIMI',
      mod_path: 'E:/Mods',
      game_exe: 'E:/Games/Game.exe',
      loader_exe: null,
      launch_args: null,
    },
  }),
}));

vi.mock('../../../hooks/objectQueryCache', () => ({
  runObjectBatchMutation: (...args: unknown[]) => runObjectBatchMutation(...args),
}));

vi.mock('../../../hooks/useObjectMutations', () => ({
  useDeleteObject: () => ({
    mutateAsync: deleteObjectMutateAsync,
  }),
  useUpdateObject: () => ({
    mutateAsync: updateObjectMutateAsync,
  }),
}));

vi.mock('./sharedObjectActionOps', () => ({
  applyObjectCategoryAndRefresh: (...args: unknown[]) => applyObjectCategoryAndRefresh(...args),
  revealObjectInExplorer: (...args: unknown[]) => revealObjectInExplorer(...args),
}));

vi.mock('./useWorkspaceSwitchActions', () => ({
  useWorkspaceSwitchActions: () => ({
    isPending: false,
    isNodePending: (node: unknown) => switchIsNodePending(node),
    setNodeEnabled: (node: unknown, desiredEnabled: unknown, surface: unknown, options: unknown) =>
      switchSetNodeEnabled(node, desiredEnabled, surface, options),
  }),
}));

vi.mock('./useSharedObjectSyncActions', () => ({
  useSharedObjectSyncActions: () => ({
    setSyncConfirm: sharedSyncSetSyncConfirm,
    handleSyncWithDb: sharedSyncHandleSyncWithDb,
    handleApplySyncMatch: sharedSyncHandleApplySyncMatch,
  }),
}));

vi.mock('../../runtime-sync/queryRefresh', () => ({
  publishRuntimeDescriptor: (...args: unknown[]) => publishRuntimeDescriptor(...args),
}));

vi.mock('../../../lib/bindings', () => ({
  commands: {
    pinObject: vi.fn(),
  },
}));

vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccess(...args),
    error: (...args: unknown[]) => toastError(...args),
  },
}));

function createWrapper(queryClient: QueryClient) {
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

function createObject(overrides: Partial<WorkspaceObjectNode> = {}): WorkspaceObjectNode {
  return {
    id: 'object-1',
    name: 'Alpha',
    display_name: 'Alpha',
    folder_path: 'Objects/Alpha',
    object_type: 'Character',
    sub_category: null,
    status: 1,
    created_at: '2025-01-01T00:00:00Z',
    mod_count: 2,
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
    active_mod_paths: null,
    matched_entry_key: null,
    matched_alias_name: null,
    matched_confidence: null,
    matched_reason: null,
    matched_source: null,
    node_kind: 'object',
    display_mode: 'unknown',
    type_chip: null,
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
    ...overrides,
  };
}

describe('useSharedObjectActions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      workspaceDialogState: { kind: 'none' },
      workspacePreviewDirty: false,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
    });
    deleteObjectMutateAsync.mockResolvedValue(undefined);
    runObjectBatchMutation.mockResolvedValue(undefined);
    updateObjectMutateAsync.mockResolvedValue(undefined);
    applyObjectCategoryAndRefresh.mockResolvedValue(undefined);
    revealObjectInExplorer.mockResolvedValue(undefined);
    publishRuntimeDescriptor.mockResolvedValue(undefined);
    switchSetNodeEnabled.mockResolvedValue(undefined);
  });

  it('opens edit dialog through workspace runtime state', () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const object = createObject();
    const { result } = renderHook(
      () =>
        useSharedObjectActions({
          objects: [object],
          schema: undefined,
        }),
      { wrapper: createWrapper(queryClient) },
    );

    act(() => {
      result.current.handleEdit(object.id);
    });

    expect(useAppStore.getState().workspaceDialogState).toEqual({
      kind: 'objectEdit',
      object,
    });
  });

  it('escalates delete failure with mods into force-delete dialog', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const object = createObject();
    useAppStore.setState({
      workspaceDialogState: {
        kind: 'objectDelete',
        id: object.id,
        name: object.name,
      },
    });
    deleteObjectMutateAsync.mockRejectedValue(new Error('ObjectHasMods 3'));

    const { result } = renderHook(
      () =>
        useSharedObjectActions({
          objects: [object],
          schema: undefined,
        }),
      { wrapper: createWrapper(queryClient) },
    );

    await act(async () => {
      await result.current.confirmDeleteObject();
    });

    await waitFor(() => {
      const dialogState = useAppStore.getState().workspaceDialogState;
      expect(dialogState.kind).toBe('objectForceDelete');
      if (dialogState.kind !== 'objectForceDelete') {
        throw new Error('Expected force delete dialog');
      }
      expect(dialogState.id).toBe(object.id);
      expect(dialogState.count).toBe(3);
    });
  });

  it('pins object through shared batch mutation', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const object = createObject({ is_pinned: false });
    const { result } = renderHook(
      () =>
        useSharedObjectActions({
          objects: [object],
          schema: undefined,
        }),
      { wrapper: createWrapper(queryClient) },
    );

    await act(async () => {
      await result.current.handlePin(object.id);
    });

    expect(runObjectBatchMutation).toHaveBeenCalledTimes(1);
    expect(toastSuccess).toHaveBeenCalledWith('toasts.pin_added_one');
  });

  it('toggles object root through shared runtime operation', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const object = createObject();
    const { result } = renderHook(
      () =>
        useSharedObjectActions({
          objects: [object],
          schema: undefined,
        }),
      { wrapper: createWrapper(queryClient) },
    );

    await act(async () => {
      await result.current.handleEnableObject(object.id);
    });

    expect(switchSetNodeEnabled).toHaveBeenCalledWith(object, true, 'object_list', {
      syncExplorerPath: false,
    });
  });

  it('publishes runtime refresh when reveal in explorer fails', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const object = createObject();
    revealObjectInExplorer.mockRejectedValue(new Error('boom'));

    const { result } = renderHook(
      () =>
        useSharedObjectActions({
          objects: [object],
          schema: undefined,
        }),
      { wrapper: createWrapper(queryClient) },
    );

    await act(async () => {
      await result.current.handleRevealInExplorer(object.id);
    });

    expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(1);
    expect(toastError).toHaveBeenCalledWith('boom');
  });
});
