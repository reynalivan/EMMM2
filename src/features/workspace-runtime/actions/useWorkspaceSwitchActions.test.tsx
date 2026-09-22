import React from 'react';
import { QueryClient, QueryClientProvider, useQueryClient } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/app/store';
import type {
  WorkspaceParentEnableRequirement,
  WorkspaceSwitchInput,
  WorkspaceSwitchResult,
} from '@/entities/workspace';
import { useWorkspaceSwitchActions } from './useWorkspaceSwitchActions';

const executeWorkspaceSwitch = vi.fn();

vi.mock('../../../shared/api/tauri/bindings', () => ({
  commands: {
    executeWorkspaceSwitch: (...args: unknown[]) => executeWorkspaceSwitch(...args),
  },
}));

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: { id: 'game-1' } }),
}));

vi.mock('../../../shared/lib/committedMutationWarning', () => ({
  notifyCommittedMutationSyncWarning: vi.fn(),
}));

function requirement(token: string): WorkspaceParentEnableRequirement {
  return {
    confirmation_token: token,
    requested_target: { path: 'E:/Mods/DISABLED Group/Alice', name: 'Alice' },
    parents: [{ path: 'E:/Mods/DISABLED Group', name: 'Group' }],
    will_activate: [{ path: 'E:/Mods/Group/Alice', name: 'Alice' }],
    stay_disabled: [],
  };
}

const resumeInput: WorkspaceSwitchInput = {
  game_id: 'game-1',
  target: { kind: 'mod_path', value: 'E:/Mods/DISABLED Group/Alice' },
  desired_enabled: true,
  resolution: 'normal',
  enable_disabled_ancestors: false,
  parent_enable_confirmation: null,
  origin_surface: 'folder_grid',
};

function wrapper({ children }: { children: React.ReactNode }) {
  return React.createElement(
    QueryClientProvider,
    { client: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
    children,
  );
}

function wrapperWithClient(client: QueryClient) {
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client }, children);
}

function openParentDialog(token: string): void {
  useAppStore.setState({
    activeGameId: 'game-1',
    gameActivationByGame: {
      'game-1': {
        game_id: 'game-1',
        generation: 1,
        phase: 'ready',
        reconcile_revision: 1,
        runtime_sync_generation: null,
        error: null,
      },
    },
    workspaceDialogState: {
      kind: 'folderEnableParent',
      folder: { id: null, path: resumeInput.target.value, name: 'Alice' },
      requirement: requirement(token),
      resumeInput,
    },
  });
}

function appliedSwitchResult(primaryPath: string): WorkspaceSwitchResult {
  return {
    status: 'applied',
    primary_path: primaryPath,
    changed_folder_paths: [],
    changed_object_ids: [],
    duplicates: [],
    parent_enable_requirement: null,
    impact: {
      rewrites: [],
      changed_object_ids: [],
      changed_folder_paths: [],
      refresh_scopes: [],
      warnings: [],
    },
    sync_warning: null,
    runtime_sync_generation: null,
  };
}

describe('useWorkspaceSwitchActions parent confirmation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useQueryClient).mockReturnValue(
      new QueryClient({ defaultOptions: { queries: { retry: false } } }),
    );
    openParentDialog('confirm-1');
  });

  it('binds the reviewed token to the confirmed mutation and duplicate continuation', async () => {
    executeWorkspaceSwitch.mockResolvedValue({
      status: 'requires_duplicate_resolution',
      primary_path: null,
      duplicates: [
        {
          mod_id: 'mod-2',
          object_id: 'object-1',
          folder_path: 'Group/Alice/Other',
          actual_name: 'Other',
          is_variant: false,
          parent_path: 'Group/Alice',
        },
      ],
      parent_enable_requirement: null,
    } satisfies Partial<WorkspaceSwitchResult>);
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    await act(async () => {
      await result.current.resolveParentEnable();
    });

    expect(executeWorkspaceSwitch).toHaveBeenCalledWith({
      ...resumeInput,
      enable_disabled_ancestors: true,
      parent_enable_confirmation: 'confirm-1',
    });
    expect(useAppStore.getState().workspaceDialogState).toMatchObject({
      kind: 'modDuplicateWarning',
      enableDisabledAncestors: true,
      parentEnableConfirmation: 'confirm-1',
    });
  });

  it('keeps the dialog open with a fresh requirement when the reviewed subtree changed', async () => {
    executeWorkspaceSwitch.mockResolvedValue({
      status: 'requires_parent_enable',
      primary_path: null,
      duplicates: [],
      parent_enable_requirement: requirement('confirm-2'),
    } satisfies Partial<WorkspaceSwitchResult>);
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    await act(async () => {
      await result.current.resolveParentEnable();
    });

    expect(useAppStore.getState().workspaceDialogState).toMatchObject({
      kind: 'folderEnableParent',
      requirement: { confirmation_token: 'confirm-2' },
    });
  });

  it('does not reopen an old game dialog after a rapid game switch', async () => {
    let complete: ((value: Partial<WorkspaceSwitchResult>) => void) | undefined;
    executeWorkspaceSwitch.mockReturnValue(
      new Promise((resolve) => {
        complete = resolve;
      }),
    );
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    let pending!: Promise<string | null>;
    act(() => {
      pending = result.current.resolveParentEnable();
    });
    useAppStore.setState({ activeGameId: 'game-2', workspaceDialogState: { kind: 'none' } });
    await act(async () => {
      complete?.({
        status: 'requires_duplicate_resolution',
        primary_path: null,
        duplicates: [],
        parent_enable_requirement: null,
      });
      await pending;
    });

    expect(useAppStore.getState().workspaceDialogState).toEqual({ kind: 'none' });
  });

  it('keeps other switches available while one node is pending', async () => {
    let complete!: (value: WorkspaceSwitchResult) => void;
    executeWorkspaceSwitch.mockReturnValue(
      new Promise<WorkspaceSwitchResult>((resolve) => {
        complete = resolve;
      }),
    );
    const firstNode = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      path: 'E:/Mods/A',
      switch_state: 'disabled',
    } as never;
    const secondNode = {
      node_kind: 'terminal_mod',
      id: 'mod-b',
      path: 'E:/Mods/B',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    let pending!: Promise<string | null>;
    act(() => {
      pending = result.current.setNodeEnabled(firstNode, true, 'folder_grid');
    });

    await waitFor(() => {
      expect(result.current.isNodePending(firstNode)).toBe(true);
    });
    expect(result.current.getPendingDesiredEnabled(firstNode)).toBe(true);
    expect(result.current.isPending).toBe(false);
    expect(result.current.isNodePending(secondNode)).toBe(false);

    await act(async () => {
      complete({
        status: 'applied',
        primary_path: 'E:/Mods/A',
        changed_folder_paths: [],
        changed_object_ids: [],
        duplicates: [],
        parent_enable_requirement: null,
        impact: {
          rewrites: [],
          changed_object_ids: [],
          changed_folder_paths: [],
          refresh_scopes: [],
          warnings: [],
        },
        sync_warning: null,
        runtime_sync_generation: null,
      });
      await pending;
    });
  });

  it('shows each latest toggle immediately while the native switch is in flight', async () => {
    let finishFirst!: (value: WorkspaceSwitchResult) => void;
    executeWorkspaceSwitch
      .mockReturnValueOnce(
        new Promise<WorkspaceSwitchResult>((resolve) => {
          finishFirst = resolve;
        }),
      )
      .mockResolvedValueOnce(appliedSwitchResult('E:/Mods/A'));
    const node = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      path: 'E:/Mods/A',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    let firstToggle!: Promise<string | null>;
    let secondToggle!: Promise<string | null>;
    act(() => {
      firstToggle = result.current.toggleNode(node, 'folder_grid');
    });
    expect(result.current.getPendingDesiredEnabled(node)).toBe(true);

    act(() => {
      secondToggle = result.current.toggleNode(node, 'folder_grid');
    });
    expect(result.current.getPendingDesiredEnabled(node)).toBe(false);
    expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(1);

    await act(async () => {
      finishFirst(appliedSwitchResult('E:/Mods/A'));
      await waitFor(() => expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(2));
      await Promise.all([firstToggle, secondToggle]);
    });

    expect(executeWorkspaceSwitch.mock.calls.map(([input]) => input.desired_enabled)).toEqual([
      true,
      false,
    ]);
    expect(result.current.getPendingDesiredEnabled(node)).toBeUndefined();
  });

  it('does not wait for cache refresh before accepting a toggle against the new path', async () => {
    let releaseRefresh!: () => void;
    const refreshPending = new Promise<void>((resolve) => {
      releaseRefresh = resolve;
    });
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    vi.spyOn(queryClient, 'invalidateQueries').mockReturnValue(refreshPending);
    vi.mocked(useQueryClient).mockReturnValue(queryClient);
    executeWorkspaceSwitch
      .mockResolvedValueOnce(appliedSwitchResult('E:/Mods/A'))
      .mockResolvedValueOnce(appliedSwitchResult('E:/Mods/DISABLED A'));
    const node = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      path: 'E:/Mods/DISABLED A',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), {
      wrapper: wrapperWithClient(queryClient),
    });

    let firstToggle!: Promise<string | null>;
    await act(async () => {
      firstToggle = result.current.toggleNode(node, 'folder_grid');
      await firstToggle;
    });
    expect(result.current.getPendingDesiredEnabled(node)).toBe(true);

    let nextToggle!: Promise<string | null>;
    await act(async () => {
      nextToggle = result.current.toggleNode(node, 'folder_grid');
      await nextToggle;
    });

    expect(executeWorkspaceSwitch.mock.calls.map(([input]) => input.target.value)).toEqual([
      'E:/Mods/DISABLED A',
      'E:/Mods/A',
    ]);
    expect(result.current.getPendingDesiredEnabled(node)).toBe(false);

    releaseRefresh();
    await waitFor(() => expect(result.current.getPendingDesiredEnabled(node)).toBeUndefined());
  });

  it('applies only the latest rapid intent for the same mod', async () => {
    let completeFirst!: (value: WorkspaceSwitchResult) => void;
    executeWorkspaceSwitch.mockReturnValueOnce(
      new Promise<WorkspaceSwitchResult>((resolve) => {
        completeFirst = resolve;
      }),
    );
    const node = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      name: 'A',
      path: 'E:/Mods/DISABLED A',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    let first!: Promise<string | null>;
    let second!: Promise<string | null>;
    let third!: Promise<string | null>;
    act(() => {
      first = result.current.toggleNode(node, 'folder_grid');
      second = result.current.toggleNode(node, 'folder_grid');
      third = result.current.toggleNode(node, 'folder_grid');
    });

    expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(1);
    expect(executeWorkspaceSwitch).toHaveBeenLastCalledWith(
      expect.objectContaining({ desired_enabled: true }),
    );

    await act(async () => {
      completeFirst(appliedSwitchResult('E:/Mods/A'));
      await expect(Promise.all([first, second, third])).resolves.toEqual([
        'E:/Mods/A',
        'E:/Mods/A',
        'E:/Mods/A',
      ]);
    });

    expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(1);
  });

  it('runs the final opposite intent after an in-flight switch completes', async () => {
    let completeFirst!: (value: WorkspaceSwitchResult) => void;
    executeWorkspaceSwitch
      .mockImplementationOnce(
        () =>
          new Promise<WorkspaceSwitchResult>((resolve) => {
            completeFirst = resolve;
          }),
      )
      .mockResolvedValueOnce(appliedSwitchResult('E:/Mods/DISABLED A'));
    const node = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      name: 'A',
      path: 'E:/Mods/DISABLED A',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    let first!: Promise<string | null>;
    let second!: Promise<string | null>;
    act(() => {
      first = result.current.toggleNode(node, 'folder_grid');
      second = result.current.toggleNode(node, 'folder_grid');
    });

    await act(async () => {
      completeFirst(appliedSwitchResult('E:/Mods/A'));
      await waitFor(() => {
        expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(2);
      });
      await expect(Promise.all([first, second])).resolves.toEqual([
        'E:/Mods/DISABLED A',
        'E:/Mods/DISABLED A',
      ]);
    });

    expect(executeWorkspaceSwitch).toHaveBeenLastCalledWith(
      expect.objectContaining({ desired_enabled: false }),
    );
  });

  it('does not show a duplicate notice for a superseded enable intent', async () => {
    let completeFirst!: (value: WorkspaceSwitchResult) => void;
    executeWorkspaceSwitch
      .mockImplementationOnce(
        () =>
          new Promise<WorkspaceSwitchResult>((resolve) => {
            completeFirst = resolve;
          }),
      )
      .mockResolvedValueOnce(appliedSwitchResult('E:/Mods/DISABLED A'));
    useAppStore.setState({ workspaceDialogState: { kind: 'none' } });
    const node = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      name: 'A',
      path: 'E:/Mods/DISABLED A',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    let first!: Promise<string | null>;
    let second!: Promise<string | null>;
    act(() => {
      first = result.current.toggleNode(node, 'folder_grid');
      second = result.current.toggleNode(node, 'folder_grid');
    });

    await act(async () => {
      completeFirst({
        ...appliedSwitchResult('E:/Mods/A'),
        duplicates: [
          {
            mod_id: 'mod-b',
            object_id: 'object-1',
            folder_path: 'E:/Mods/B',
            actual_name: 'B',
            is_variant: false,
            parent_path: '',
          },
        ],
      });
      await Promise.all([first, second]);
    });

    expect(useAppStore.getState().workspaceDialogState).toEqual({ kind: 'none' });
  });

  it('preserves rapid intents for different mods', async () => {
    let completeFirst!: (value: WorkspaceSwitchResult) => void;
    executeWorkspaceSwitch
      .mockImplementationOnce(
        () =>
          new Promise<WorkspaceSwitchResult>((resolve) => {
            completeFirst = resolve;
          }),
      )
      .mockResolvedValueOnce(appliedSwitchResult('E:/Mods/B'));
    const enabledNode = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      path: 'E:/Mods/A',
      switch_state: 'enabled',
    } as never;
    const disabledNode = {
      node_kind: 'terminal_mod',
      id: 'mod-b',
      path: 'E:/Mods/DISABLED B',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    let first!: Promise<string | null>;
    let second!: Promise<string | null>;
    act(() => {
      first = result.current.toggleNode(enabledNode, 'folder_grid');
      second = result.current.toggleNode(disabledNode, 'folder_grid');
    });

    expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(1);
    expect(executeWorkspaceSwitch).toHaveBeenLastCalledWith(
      expect.objectContaining({ desired_enabled: false }),
    );

    await act(async () => {
      completeFirst(appliedSwitchResult('E:/Mods/DISABLED A'));
      await waitFor(() => {
        expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(2);
      });
      await expect(Promise.all([first, second])).resolves.toEqual([
        'E:/Mods/DISABLED A',
        'E:/Mods/B',
      ]);
    });

    expect(executeWorkspaceSwitch).toHaveBeenLastCalledWith(
      expect.objectContaining({ desired_enabled: true }),
    );
  });

  it('enables a conflicting mod before opening an informational warning', async () => {
    executeWorkspaceSwitch.mockResolvedValue({
      status: 'applied',
      primary_path: 'E:/Mods/A',
      changed_folder_paths: [],
      changed_object_ids: [],
      duplicates: [
        {
          mod_id: 'mod-b',
          object_id: 'object-1',
          folder_path: 'E:/Mods/B',
          actual_name: 'B',
          is_variant: false,
          parent_path: '',
        },
      ],
      parent_enable_requirement: null,
      impact: {
        rewrites: [],
        changed_object_ids: [],
        changed_folder_paths: [],
        refresh_scopes: [],
        warnings: [],
      },
      sync_warning: null,
      runtime_sync_generation: null,
    } satisfies WorkspaceSwitchResult);
    const node = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      name: 'A',
      path: 'E:/Mods/DISABLED A',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    await act(async () => {
      await expect(result.current.setNodeEnabled(node, true, 'folder_grid')).resolves.toBe(
        'E:/Mods/A',
      );
    });

    expect(useAppStore.getState().workspaceDialogState).toMatchObject({
      kind: 'modDuplicateWarning',
      requiresResolution: false,
      folder: { path: 'E:/Mods/A' },
    });
  });
});
