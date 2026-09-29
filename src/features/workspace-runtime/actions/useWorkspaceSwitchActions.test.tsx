import React from 'react';
import { QueryClient, QueryClientProvider, useQueryClient } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/app/store';
import { toast } from '@/shared/ui/toast';
import type {
  WorkspaceParentEnableRequirement,
  WorkspaceSwitchInput,
  WorkspaceSwitchResult,
} from '@/entities/workspace';
import {
  clearFolderBulkPendingDesired,
  clearObjectBulkPendingDesired,
  setFolderBulkPendingDesired,
  setObjectBulkPendingDesired,
  useWorkspaceSwitchActions,
} from './useWorkspaceSwitchActions';
import { recordWorkspaceProjectedRevision } from './workspaceSwitchOps';

const executeWorkspaceSwitch = vi.fn();
const getWorkspaceSwitchSnapshot = vi.fn();
const admitWorkspaceSwitchIntent = vi.fn().mockResolvedValue(true);
const successToast = vi.spyOn(toast, 'success').mockReturnValue('toast-id');

vi.mock('../../../shared/api/tauri/bindings', () => ({
  commands: {
    executeWorkspaceSwitch: (...args: unknown[]) => executeWorkspaceSwitch(...args),
    getWorkspaceSwitchSnapshot: (...args: unknown[]) => getWorkspaceSwitchSnapshot(...args),
    admitWorkspaceSwitchIntent: (...args: unknown[]) => admitWorkspaceSwitchIntent(...args),
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
    disk_revision: null,
  };
}

describe('useWorkspaceSwitchActions parent confirmation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    executeWorkspaceSwitch.mockReset();
    getWorkspaceSwitchSnapshot.mockResolvedValue({
      game_id: 'game-1',
      source_epoch: 'root-a',
      disk_revision: 42,
      projected_revision: 0,
    });
    vi.mocked(useQueryClient).mockReturnValue(
      new QueryClient({ defaultOptions: { queries: { retry: false } } }),
    );
    openParentDialog('confirm-1');
  });

  it('keeps only the newest object bulk optimistic state', () => {
    const node = {
      node_kind: 'object',
      id: 'obj-1',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    act(() => setObjectBulkPendingDesired('game-1', ['obj-1'], true, 100));
    expect(result.current.getPendingDesiredEnabled(node)).toBe(true);
    act(() => setObjectBulkPendingDesired('game-1', ['obj-1'], false, 101));
    act(() => clearObjectBulkPendingDesired('game-1', ['obj-1'], 100));
    expect(result.current.getPendingDesiredEnabled(node)).toBe(false);
    act(() => clearObjectBulkPendingDesired('game-1', ['obj-1'], 101));
    expect(result.current.getPendingDesiredEnabled(node)).toBeUndefined();
  });

  it('keeps the newest bulk folder state across enabled and disabled path spellings', () => {
    const node = {
      node_kind: 'terminal_mod',
      id: 'mod-1',
      path: 'E:/Mods/Blue',
      switch_state: 'enabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    act(() => setFolderBulkPendingDesired('game-1', ['E:/Mods/DISABLED Blue'], true, 200));
    expect(result.current.getPendingDesiredEnabled(node)).toBe(true);
    act(() => setFolderBulkPendingDesired('game-1', ['E:/Mods/Blue'], false, 201));
    act(() => clearFolderBulkPendingDesired('game-1', ['E:/Mods/DISABLED Blue'], 200));
    expect(result.current.getPendingDesiredEnabled(node)).toBe(false);
    act(() => clearFolderBulkPendingDesired('game-1', ['E:/Mods/Blue'], 201));
    expect(result.current.getPendingDesiredEnabled(node)).toBeUndefined();
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

    expect(executeWorkspaceSwitch).toHaveBeenCalledWith(
      {
        ...resumeInput,
        enable_disabled_ancestors: true,
        parent_enable_confirmation: 'confirm-1',
      },
      expect.any(Number),
    );
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

  it('shows success after a confirmed parent enable has a disk receipt', async () => {
    executeWorkspaceSwitch.mockResolvedValue({
      ...appliedSwitchResult('E:/Mods/Group/Alice'),
      disk_revision: 0,
    });
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    await act(async () => {
      await result.current.resolveParentEnable();
    });

    await waitFor(() => expect(successToast).toHaveBeenCalledExactlyOnceWith('Enabled: Alice'));
  });

  it.each(['resolveDuplicateForceEnable', 'resolveDuplicateEnableOnly'] as const)(
    'shows success after %s commits the folder to disk',
    async (action) => {
      executeWorkspaceSwitch.mockResolvedValue({
        ...appliedSwitchResult('E:/Mods/A'),
        disk_revision: 0,
      });
      const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

      await act(async () => {
        await result.current[action]({ path: 'E:/Mods/DISABLED A' });
      });

      await waitFor(() => expect(successToast).toHaveBeenCalledExactlyOnceWith('Enabled: A'));
    },
  );

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
        disk_revision: null,
      });
      await pending;
    });
  });

  it('shows folder success only after the backend returns a disk receipt', async () => {
    let complete!: (value: WorkspaceSwitchResult) => void;
    executeWorkspaceSwitch.mockReturnValue(
      new Promise<WorkspaceSwitchResult>((resolve) => {
        complete = resolve;
      }),
    );
    const node = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      path: 'E:/Mods/DISABLED A',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    let switching!: Promise<string | null>;
    act(() => {
      switching = result.current.setNodeEnabled(node, true, 'folder_grid');
    });
    expect(successToast).not.toHaveBeenCalled();

    await act(async () => {
      complete({ ...appliedSwitchResult('E:/Mods/A'), disk_revision: 0 });
      await switching;
    });
    await waitFor(() => expect(successToast).toHaveBeenCalledExactlyOnceWith('Enabled: A'));
  });

  it('does not show folder success for a no-op without a disk rename', async () => {
    executeWorkspaceSwitch.mockResolvedValue({
      ...appliedSwitchResult('E:/Mods/A'),
      status: 'noop',
    });
    const node = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      path: 'E:/Mods/A',
      switch_state: 'enabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    await act(async () => {
      await result.current.setNodeEnabled(node, true, 'folder_grid');
    });
    expect(successToast).not.toHaveBeenCalled();
  });

  it('does not show folder success without a verified disk revision', async () => {
    executeWorkspaceSwitch.mockResolvedValue(appliedSwitchResult('E:/Mods/A'));
    const node = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      path: 'E:/Mods/DISABLED A',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    await act(async () => {
      await result.current.setNodeEnabled(node, true, 'folder_grid');
    });
    expect(successToast).not.toHaveBeenCalled();
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
    await waitFor(() => expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(1));

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

  it('retains the disk receipt overlay until projection and the refreshed query settle', async () => {
    let releaseRefresh!: () => void;
    const refreshPending = new Promise<void>((resolve) => {
      releaseRefresh = resolve;
    });
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const invalidate = vi.spyOn(queryClient, 'invalidateQueries').mockReturnValue(refreshPending);
    vi.mocked(useQueryClient).mockReturnValue(queryClient);
    executeWorkspaceSwitch.mockResolvedValue({
      ...appliedSwitchResult('E:/Mods/A'),
      disk_revision: 42,
    });
    const node = {
      node_kind: 'terminal_mod',
      id: 'mod-a',
      path: 'E:/Mods/DISABLED A',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), {
      wrapper: wrapperWithClient(queryClient),
    });

    await act(async () => {
      await result.current.setNodeEnabled(node, true, 'folder_grid');
    });
    expect(result.current.getPendingDesiredEnabled(node)).toBe(true);
    expect(invalidate).not.toHaveBeenCalled();

    act(() => recordWorkspaceProjectedRevision('game-1', 42));
    await waitFor(() => expect(invalidate).toHaveBeenCalled());
    expect(result.current.getPendingDesiredEnabled(node)).toBe(true);

    releaseRefresh();
    await waitFor(() => expect(result.current.getPendingDesiredEnabled(node)).toBeUndefined());
  });

  it('keeps the disk receipt overlay until a delayed projection is verified', async () => {
    vi.useFakeTimers();
    try {
      executeWorkspaceSwitch.mockResolvedValue({
        ...appliedSwitchResult('E:/Mods/A'),
        disk_revision: 1_000_000,
      });
      const node = {
        node_kind: 'terminal_mod',
        id: 'mod-a',
        path: 'E:/Mods/DISABLED A',
        switch_state: 'disabled',
      } as never;
      const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

      await act(async () => {
        await result.current.setNodeEnabled(node, true, 'folder_grid');
      });
      expect(result.current.getPendingDesiredEnabled(node)).toBe(true);

      await act(async () => {
        await vi.advanceTimersByTimeAsync(11_000);
      });
      expect(result.current.getPendingDesiredEnabled(node)).toBe(true);

      await act(async () => {
        recordWorkspaceProjectedRevision('game-1', 1_000_000);
        await vi.advanceTimersByTimeAsync(2_000);
      });
      expect(result.current.getPendingDesiredEnabled(node)).toBeUndefined();
    } finally {
      vi.useRealTimers();
    }
  });

  it('applies only the latest rapid intent for the same mod', async () => {
    let completeFirst!: (value: WorkspaceSwitchResult) => void;
    executeWorkspaceSwitch
      .mockReturnValueOnce(
        new Promise<WorkspaceSwitchResult>((resolve) => {
          completeFirst = resolve;
        }),
      )
      .mockResolvedValueOnce({ ...appliedSwitchResult('E:/Mods/A'), disk_revision: 0 });
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

    await waitFor(() => expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(1));
    expect(admitWorkspaceSwitchIntent).toHaveBeenCalledTimes(2);
    expect(admitWorkspaceSwitchIntent).toHaveBeenLastCalledWith(
      'game-1',
      [{ kind: 'mod_path', value: 'E:/Mods/DISABLED A' }],
      expect.any(Number),
    );
    expect(executeWorkspaceSwitch).toHaveBeenLastCalledWith(
      expect.objectContaining({ desired_enabled: true }),
      expect.any(Number),
    );

    await act(async () => {
      completeFirst({ ...appliedSwitchResult('E:/Mods/A'), disk_revision: 0 });
      await expect(Promise.all([first, second, third])).resolves.toEqual([
        'E:/Mods/A',
        'E:/Mods/A',
        'E:/Mods/A',
      ]);
    });

    expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(2);
    await waitFor(() => expect(successToast).toHaveBeenCalledExactlyOnceWith('Enabled: A'));
  });

  it('coalesces a thousand alternating clicks into one running and one latest operation', async () => {
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
      path: 'E:/Mods/DISABLED A',
      switch_state: 'disabled',
    } as never;
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });
    const completions: Array<Promise<string | null>> = [];

    act(() => {
      for (let click = 0; click < 1_000; click += 1) {
        completions.push(result.current.toggleNode(node, 'folder_grid'));
      }
    });
    expect(new Set(completions).size).toBe(1);
    expect(result.current.getPendingDesiredEnabled(node)).toBe(false);
    await waitFor(() => expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(1));

    await act(async () => {
      completeFirst(appliedSwitchResult('E:/Mods/A'));
      await Promise.all(completions);
    });
    expect(executeWorkspaceSwitch.mock.calls.map(([input]) => input.desired_enabled)).toEqual([
      true,
      false,
    ]);
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
      .mockResolvedValueOnce({ ...appliedSwitchResult('E:/Mods/DISABLED A'), disk_revision: 0 });
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

    await waitFor(() => expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(1));
    await act(async () => {
      completeFirst({ ...appliedSwitchResult('E:/Mods/A'), disk_revision: 0 });
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
      expect.any(Number),
    );
    await waitFor(() => expect(successToast).toHaveBeenCalledExactlyOnceWith('Disabled: A'));
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

    await waitFor(() => expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(1));
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

    await waitFor(() => expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(2));
    expect(executeWorkspaceSwitch.mock.calls.map(([input]) => input.desired_enabled)).toEqual([
      false,
      true,
    ]);

    await act(async () => {
      completeFirst(appliedSwitchResult('E:/Mods/DISABLED A'));
      await expect(Promise.all([first, second])).resolves.toEqual([
        'E:/Mods/DISABLED A',
        'E:/Mods/B',
      ]);
    });

    expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(2);
  });

  it('shares the latest same-mod intent across hook instances and explicit set-state', async () => {
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
      path: 'E:/Mods/DISABLED A',
      switch_state: 'disabled',
    } as never;
    const grid = renderHook(() => useWorkspaceSwitchActions(), { wrapper });
    const preview = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    let first!: Promise<string | null>;
    let second!: Promise<string | null>;
    act(() => {
      first = grid.result.current.toggleNode(node, 'folder_grid');
      second = preview.result.current.setNodeEnabled(node, false, 'preview');
    });

    await waitFor(() => expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(1));
    expect(grid.result.current.getPendingDesiredEnabled(node)).toBe(false);
    expect(preview.result.current.getPendingDesiredEnabled(node)).toBe(false);

    await act(async () => {
      completeFirst(appliedSwitchResult('E:/Mods/A'));
      await Promise.all([first, second]);
    });
    expect(executeWorkspaceSwitch.mock.calls.map(([input]) => input.desired_enabled)).toEqual([
      true,
      false,
    ]);
  });

  it('coalesces folder-path requests across enabled and disabled spellings', async () => {
    let completeFirst!: (value: WorkspaceSwitchResult) => void;
    executeWorkspaceSwitch
      .mockImplementationOnce(
        () =>
          new Promise<WorkspaceSwitchResult>((resolve) => {
            completeFirst = resolve;
          }),
      )
      .mockResolvedValueOnce({ ...appliedSwitchResult('E:/Mods/DISABLED A'), disk_revision: 0 });
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    let first!: Promise<string | null>;
    let second!: Promise<string | null>;
    act(() => {
      first = result.current.setFolderPathEnabled('E:/Mods/DISABLED A', true);
      second = result.current.setFolderPathEnabled('E:/Mods/A', false);
    });
    await waitFor(() => expect(executeWorkspaceSwitch).toHaveBeenCalledTimes(1));

    await act(async () => {
      completeFirst({ ...appliedSwitchResult('E:/Mods/A'), disk_revision: 0 });
      await Promise.all([first, second]);
    });
    expect(executeWorkspaceSwitch.mock.calls.map(([input]) => input.target.value)).toEqual([
      'E:/Mods/DISABLED A',
      'E:/Mods/A',
    ]);
    expect(executeWorkspaceSwitch.mock.calls.map(([input]) => input.desired_enabled)).toEqual([
      true,
      false,
    ]);
    await waitFor(() => expect(successToast).toHaveBeenCalledExactlyOnceWith('Disabled: A'));
  });

  it('summarizes rapid disk-verified switches without one toast per commit', async () => {
    executeWorkspaceSwitch
      .mockResolvedValueOnce({ ...appliedSwitchResult('E:/Mods/A'), disk_revision: 1 })
      .mockResolvedValueOnce({ ...appliedSwitchResult('E:/Mods/B'), disk_revision: 2 })
      .mockResolvedValueOnce({ ...appliedSwitchResult('E:/Mods/DISABLED A'), disk_revision: 3 });
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    await act(async () => {
      await result.current.setFolderPathEnabled('E:/Mods/DISABLED A', true);
      await result.current.setFolderPathEnabled('E:/Mods/DISABLED B', true);
      await result.current.setFolderPathEnabled('E:/Mods/A', false);
    });

    expect(successToast).not.toHaveBeenCalled();
    await waitFor(() => {
      expect(successToast).toHaveBeenCalledExactlyOnceWith('Updated: A, B');
    });
  });

  it('does not show a delayed switch success after changing games', async () => {
    executeWorkspaceSwitch.mockResolvedValue({
      ...appliedSwitchResult('E:/Mods/A'),
      disk_revision: 1,
    });
    const { result } = renderHook(() => useWorkspaceSwitchActions(), { wrapper });

    await act(async () => {
      await result.current.setFolderPathEnabled('E:/Mods/DISABLED A', true);
    });
    useAppStore.setState({ activeGameId: 'game-2' });

    await new Promise((resolve) => setTimeout(resolve, 600));
    expect(successToast).not.toHaveBeenCalled();
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
      disk_revision: null,
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
