import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook } from '@testing-library/react';
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

describe('useWorkspaceSwitchActions parent confirmation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
});
