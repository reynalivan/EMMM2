import { beforeEach, describe, expect, it, vi } from 'vitest';
import { QueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import type {
  WorkspaceImpact,
  WorkspaceSwitchInput,
  WorkspaceSwitchResult,
} from '@/entities/workspace';
import {
  applyWorkspaceSwitchEffects,
  buildNodePendingKey,
  buildSwitchRefreshDescriptor,
  executeWorkspaceSwitch,
  isWorkspaceObjectNode,
  parseRenameConflict,
  recordWorkspaceProjectedRevision,
  togglePendingKey,
  waitForWorkspaceProjection,
  primeWorkspaceRootEpoch,
} from './workspaceSwitchOps';

const executeWorkspaceSwitchCommand = vi.fn();
const getWorkspaceSwitchSnapshotCommand = vi.fn();
const reconcileDiskStateCommand = vi.fn();
const openFolderConflictManagerDialog = vi.fn();
const openRenameConfirmationDialog = vi.fn();
const openWorkspaceFileInUseDialog = vi.fn();
const applyFolderConflictReconcileResult = vi.fn(() => true);
const setRenameConfirmations = vi.fn();
const toastError = vi.fn();
const toastInfo = vi.fn();
const toastWarning = vi.fn();
const notifyCommittedMutationSyncWarning = vi.fn();
const publishRuntimeDescriptor = vi.fn();
const cancelRuntimeDescriptorQueries = vi.fn((queryClient: QueryClient) =>
  queryClient.cancelQueries({ queryKey: ['workspace', 'mods'] }),
);
const appState = vi.hoisted(() => ({ activeGameId: 'game-1' as string | null }));

vi.mock('../../../shared/api/tauri/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    executeWorkspaceSwitch: (...args: unknown[]) => executeWorkspaceSwitchCommand(...args),
    getWorkspaceSwitchSnapshot: (...args: unknown[]) => getWorkspaceSwitchSnapshotCommand(...args),
    reconcileDiskStateCmd: (...args: unknown[]) => reconcileDiskStateCommand(...args),
  },
}));

vi.mock('../state/workspaceDialogs', () => ({
  openFolderConflictManagerDialog: (...args: unknown[]) => openFolderConflictManagerDialog(...args),
  openRenameConfirmationDialog: (...args: unknown[]) => openRenameConfirmationDialog(...args),
  openWorkspaceFileInUseDialog: (...args: unknown[]) => openWorkspaceFileInUseDialog(...args),
}));

vi.mock('@/app/store', () => ({
  useAppStore: {
    getState: () => ({
      activeGameId: appState.activeGameId,
      applyFolderConflictReconcileResult,
      setRenameConfirmations,
    }),
  },
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    error: (...args: unknown[]) => toastError(...args),
    info: (...args: unknown[]) => toastInfo(...args),
    warning: (...args: unknown[]) => toastWarning(...args),
  },
}));

vi.mock('@/shared/lib/queryRefresh', () => ({
  cancelRuntimeDescriptorQueries: (queryClient: QueryClient) =>
    cancelRuntimeDescriptorQueries(queryClient),
  publishRuntimeDescriptor: (...args: unknown[]) => publishRuntimeDescriptor(...args),
  publishQueryInvalidations: vi.fn(),
}));

vi.mock('../../../shared/lib/committedMutationWarning', () => ({
  notifyCommittedMutationSyncWarning: (...args: unknown[]) =>
    notifyCommittedMutationSyncWarning(...args),
}));

describe('workspace switch ops', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    appState.activeGameId = 'game-1';
    reconcileDiskStateCommand.mockResolvedValue(null);
    getWorkspaceSwitchSnapshotCommand.mockResolvedValue({
      game_id: 'game-1',
      source_epoch: 'root-a',
      disk_revision: 0,
      projected_revision: 0,
    });
    publishRuntimeDescriptor.mockResolvedValue(undefined);
  });

  describe('togglePendingKey', () => {
    it('adds a pending key and keeps the map immutable', () => {
      const current = {};
      const next = togglePendingKey(current, 'folder:a', true);

      expect(next).toEqual({ 'folder:a': true });
      expect(current).toEqual({});
    });

    it('removes a pending key', () => {
      expect(togglePendingKey({ 'folder:a': true, 'folder:b': true }, 'folder:a', false)).toEqual({
        'folder:b': true,
      });
    });

    it('returns the same reference when clearing an untracked key', () => {
      const current = { 'folder:b': true };
      expect(togglePendingKey(current, 'folder:a', false)).toBe(current);
    });
  });

  describe('node identity', () => {
    it('keys object nodes and identified folders by stable ids', () => {
      expect(buildNodePendingKey({ node_kind: 'object', id: 'o1' } as never)).toBe('object:o1');
      expect(
        buildNodePendingKey({ node_kind: 'terminal_mod', id: 'm1', path: 'a/b' } as never),
      ).toBe('folder:m1');
      expect(buildNodePendingKey({ node_kind: 'terminal_mod', path: 'a/b' } as never)).toBe(
        'folder:a/b',
      );
      expect(
        buildNodePendingKey({ node_kind: 'terminal_mod', path: 'E:/Mods/DISABLED A' } as never),
      ).not.toBe(buildNodePendingKey({ node_kind: 'terminal_mod', path: 'E:/Mods/A' } as never));
    });

    it('narrows object nodes', () => {
      expect(isWorkspaceObjectNode({ node_kind: 'object' } as never)).toBe(true);
      expect(isWorkspaceObjectNode({ node_kind: 'terminal_mod' } as never)).toBe(false);
    });
  });

  describe('parseRenameConflict', () => {
    it('returns null for unrelated errors', () => {
      expect(parseRenameConflict(new Error('boom'))).toBeNull();
      expect(parseRenameConflict('{"type":"Io"}')).toBeNull();
    });

    it('parses the structured rename conflict payload', () => {
      const raw = JSON.stringify({
        type: 'RenameConflict',
        attempted_target: 'E:/Mods/B',
        existing_path: 'E:/Mods/A',
        base_name: 'A',
      });

      expect(parseRenameConflict(new Error(raw))).toEqual({
        type: 'RenameConflict',
        attempted_target: 'E:/Mods/B',
        existing_path: 'E:/Mods/A',
        base_name: 'A',
      });
    });
  });

  describe('buildSwitchRefreshDescriptor', () => {
    it('falls back to the mutation class when impact carries no scopes', () => {
      const fallback = buildSwitchRefreshDescriptor(null, 'objectSwitch');
      expect(fallback.refreshEvents).toContain('objectRowsChanged');
    });

    it('uses backend refresh scopes when present', () => {
      const descriptor = buildSwitchRefreshDescriptor(
        { refresh_scopes: ['thumbnailChanged'], rewrites: [] } as unknown as WorkspaceImpact,
        'folderSwitch',
      );

      expect(descriptor.refreshEvents).toEqual(['thumbnailChanged']);
    });
  });

  describe('executeWorkspaceSwitch', () => {
    const input: WorkspaceSwitchInput = {
      game_id: 'game-1',
      target: { kind: 'mod_path', value: 'E:/Mods/A' },
      desired_enabled: true,
      resolution: 'normal',
      enable_disabled_ancestors: false,
      parent_enable_confirmation: null,
      origin_surface: 'folder_grid',
    };

    it('submits the disk switch before projection listener registration completes', async () => {
      let finishRegistration!: (unlisten: () => void) => void;
      vi.mocked(listen).mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishRegistration = resolve;
          }),
      );
      executeWorkspaceSwitchCommand.mockResolvedValue({ primary_path: 'E:/Mods/A' });

      try {
        const switchPromise = executeWorkspaceSwitch(input);
        expect(executeWorkspaceSwitchCommand).toHaveBeenCalledTimes(1);
        await switchPromise;
      } finally {
        finishRegistration?.(() => undefined);
      }
    });

    it('returns a committed switch and presents terminal projection lag', async () => {
      executeWorkspaceSwitchCommand.mockResolvedValue({
        status: 'applied',
        sync_warning: { kind: 'ReconcileFailed', message: 'projection pending' },
      });

      await expect(executeWorkspaceSwitch(input)).resolves.toEqual(
        expect.objectContaining({ status: 'applied' }),
      );
      expect(notifyCommittedMutationSyncWarning).toHaveBeenCalledWith(
        expect.objectContaining({ sync_warning: expect.any(Object) }),
      );
    });

    it('returns the switch result on success', async () => {
      executeWorkspaceSwitchCommand.mockResolvedValue({ primary_path: 'E:/Mods/A' });

      await expect(executeWorkspaceSwitch(input)).resolves.toEqual({ primary_path: 'E:/Mods/A' });
      expect(toastError).not.toHaveBeenCalled();
    });

    it('submits different targets without a frontend game-wide queue', async () => {
      let completeFirst!: () => void;
      executeWorkspaceSwitchCommand
        .mockImplementationOnce(
          () =>
            new Promise<void>((resolve) => {
              completeFirst = resolve;
            }),
        )
        .mockResolvedValueOnce({ primary_path: 'E:/Mods/B' });

      const first = executeWorkspaceSwitch(input);
      const second = executeWorkspaceSwitch({
        ...input,
        target: { kind: 'mod_path', value: 'E:/Mods/B' },
        desired_enabled: false,
      });

      await vi.waitFor(() => {
        expect(executeWorkspaceSwitchCommand).toHaveBeenCalledTimes(2);
      });
      await expect(second).resolves.toEqual({ primary_path: 'E:/Mods/B' });
      completeFirst();
      await first;
    });

    it('reports a rename conflict when reconcile cannot produce a safe review queue', async () => {
      executeWorkspaceSwitchCommand.mockRejectedValue(
        new Error(
          JSON.stringify({
            type: 'RenameConflict',
            attempted_target: 'E:/Mods/B',
            existing_path: 'E:/Mods/A',
            base_name: 'A',
          }),
        ),
      );

      await expect(executeWorkspaceSwitch(input)).resolves.toBeNull();
      expect(toastError).toHaveBeenCalledTimes(1);
      expect(openFolderConflictManagerDialog).not.toHaveBeenCalled();
      expect(openRenameConfirmationDialog).not.toHaveBeenCalled();
    });

    it('normalizes a direct rename conflict into the folder conflict manager', async () => {
      executeWorkspaceSwitchCommand.mockRejectedValue(
        new Error(
          JSON.stringify({
            type: 'RenameConflict',
            attempted_target: 'E:/Mods/B',
            existing_path: 'E:/Mods/A',
            base_name: 'A',
          }),
        ),
      );
      const folderConflicts = [{ group_id: 'group-1', candidates: [] }];
      reconcileDiskStateCommand.mockResolvedValue({
        game_id: 'game-1',
        reconcile_revision: 1,
        status: 'AppliedWithFolderConflicts',
        folder_conflicts: folderConflicts,
        rename_confirmations: [],
      });

      await expect(executeWorkspaceSwitch(input)).resolves.toBeNull();

      expect(applyFolderConflictReconcileResult).toHaveBeenCalledWith(
        expect.objectContaining({ folder_conflicts: folderConflicts }),
      );
      expect(openFolderConflictManagerDialog).toHaveBeenCalledTimes(1);
    });

    it('applies an empty preflight report so an open queue can resolve externally', async () => {
      executeWorkspaceSwitchCommand.mockRejectedValue(
        new Error(
          JSON.stringify({
            type: 'RenameConflict',
            attempted_target: 'E:/Mods/B',
            existing_path: 'E:/Mods/A',
            base_name: 'A',
          }),
        ),
      );
      const report = {
        game_id: 'game-1',
        reconcile_revision: 2,
        status: 'Applied',
        folder_conflicts: [],
        rename_confirmations: [],
      };
      reconcileDiskStateCommand.mockResolvedValue(report);

      await expect(executeWorkspaceSwitch(input)).resolves.toBeNull();

      expect(applyFolderConflictReconcileResult).toHaveBeenCalledWith(report);
      expect(openFolderConflictManagerDialog).not.toHaveBeenCalled();
    });

    it('routes file-in-use failures to the file-in-use dialog', async () => {
      executeWorkspaceSwitchCommand.mockRejectedValue({
        FileInUse: { path: 'E:/Mods/A/mod.ini', processes: ['3dmigoto.exe'] },
      });

      await expect(executeWorkspaceSwitch(input)).resolves.toBeNull();
      expect(openWorkspaceFileInUseDialog).toHaveBeenCalledWith({
        path: 'E:/Mods/A/mod.ini',
        processes: ['3dmigoto.exe'],
      });
      expect(toastError).not.toHaveBeenCalled();
    });

    it('toasts unknown failures', async () => {
      executeWorkspaceSwitchCommand.mockRejectedValue(new Error('boom'));

      await expect(executeWorkspaceSwitch(input)).resolves.toBeNull();
      expect(toastError).toHaveBeenCalledTimes(1);
    });

    it('does not open repair UI for a failed switch from a superseded game', async () => {
      appState.activeGameId = 'game-2';
      executeWorkspaceSwitchCommand.mockRejectedValue(
        new Error(
          JSON.stringify({
            type: 'RenameConflict',
            attempted_target: 'E:/Mods/B',
            existing_path: 'E:/Mods/A',
            base_name: 'A',
          }),
        ),
      );

      await expect(executeWorkspaceSwitch(input)).resolves.toBeNull();

      expect(reconcileDiskStateCommand).not.toHaveBeenCalled();
      expect(openFolderConflictManagerDialog).not.toHaveBeenCalled();
      expect(openRenameConfirmationDialog).not.toHaveBeenCalled();
      expect(toastError).not.toHaveBeenCalled();
    });
  });

  it('keeps an applied disk switch silent after updating the workspace', async () => {
    const result = {
      status: 'applied',
      impact: { rewrites: [], refresh_scopes: [] },
    } as unknown as WorkspaceSwitchResult;

    applyWorkspaceSwitchEffects(new QueryClient(), result, 'folderSwitch');

    expect(toastInfo).not.toHaveBeenCalled();
  });

  it('recovers a missed projection event from the durable snapshot', async () => {
    getWorkspaceSwitchSnapshotCommand.mockResolvedValueOnce({
      game_id: 'game-1',
      source_epoch: 'root-a',
      disk_revision: 42,
      projected_revision: 42,
    });

    await waitForWorkspaceProjection('game-1', 42);

    expect(getWorkspaceSwitchSnapshotCommand).toHaveBeenCalledWith('game-1');
  });

  it('shares one snapshot request across simultaneous receipts in an epoch', async () => {
    let completeSnapshot!: (value: unknown) => void;
    getWorkspaceSwitchSnapshotCommand.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          completeSnapshot = resolve;
        }),
    );
    const first = waitForWorkspaceProjection('game-1', 101, 'shared-root');
    const second = waitForWorkspaceProjection('game-1', 102, 'shared-root');
    await vi.waitFor(() => expect(getWorkspaceSwitchSnapshotCommand).toHaveBeenCalledTimes(1));
    completeSnapshot({ game_id: 'game-1', source_epoch: 'shared-root', projected_revision: 102 });
    await Promise.all([first, second]);
  });

  it('does not acknowledge an old-root receipt using a newer root checkpoint', async () => {
    getWorkspaceSwitchSnapshotCommand.mockResolvedValueOnce({
      game_id: 'game-1',
      source_epoch: 'new-root',
      projected_revision: 999_999,
    });
    await expect(waitForWorkspaceProjection('game-1', 150, 'old-root')).rejects.toThrow('epoch');
  });

  it('does not refresh queries for a no-op receipt', async () => {
    await applyWorkspaceSwitchEffects(
      new QueryClient(),
      {
        status: 'noop',
        impact: { rewrites: [], refresh_scopes: [] },
      } as unknown as WorkspaceSwitchResult,
      'folderSwitch',
      { gameId: 'game-1' },
    );
    expect(publishRuntimeDescriptor).not.toHaveBeenCalled();
    expect(cancelRuntimeDescriptorQueries).not.toHaveBeenCalled();
  });

  it('recovers a rejected refresh without dropping its verified disk observation', async () => {
    publishRuntimeDescriptor
      .mockRejectedValueOnce(new Error('query busy'))
      .mockResolvedValue(undefined);
    await applyWorkspaceSwitchEffects(
      new QueryClient(),
      {
        status: 'applied',
        impact: { rewrites: [], refresh_scopes: ['workspaceChanged'] },
      } as unknown as WorkspaceSwitchResult,
      'folderSwitch',
      { gameId: 'game-1' },
    );
    expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(2);
  });

  it('coalesces a burst of failed refresh receipts into one retry owner', async () => {
    vi.useFakeTimers();
    try {
      publishRuntimeDescriptor.mockRejectedValue(new Error('query busy'));
      const client = new QueryClient();
      const completions = Array.from({ length: 100 }, (_, index) =>
        applyWorkspaceSwitchEffects(
          client,
          {
            status: 'applied',
            primary_path: 'E:/Mods/A',
            impact: {
              rewrites: [],
              refresh_scopes: index % 2 ? ['workspaceChanged'] : ['collectionsChanged'],
            },
          } as unknown as WorkspaceSwitchResult,
          'folderSwitch',
          { gameId: 'game-1' },
        ),
      );
      expect(new Set(completions).size).toBe(1);
      await vi.advanceTimersByTimeAsync(1);
      expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(1);
      publishRuntimeDescriptor.mockResolvedValue(undefined);
      await vi.advanceTimersByTimeAsync(250);
      await Promise.all(completions);
      expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(2);
      expect(publishRuntimeDescriptor).toHaveBeenLastCalledWith(
        client,
        expect.objectContaining({
          refreshEvents: expect.arrayContaining(['workspaceChanged', 'collectionsChanged']),
        }),
        'active',
      );
    } finally {
      vi.useRealTimers();
    }
  });

  it('recovers an unknown receipt epoch after the first snapshot fails', async () => {
    vi.useFakeTimers();
    try {
      appState.activeGameId = 'game-unknown-epoch';
      getWorkspaceSwitchSnapshotCommand
        .mockRejectedValueOnce(new Error('snapshot database busy'))
        .mockResolvedValue({
          game_id: 'game-unknown-epoch',
          source_epoch: 'first-root',
          projected_revision: 7,
        });
      const onSyncError = vi.fn();
      const completion = applyWorkspaceSwitchEffects(
        new QueryClient(),
        {
          status: 'applied',
          primary_path: 'E:/Mods/A',
          source_epoch: 'first-root',
          disk_revision: 7,
          impact: { rewrites: [], refresh_scopes: ['workspaceChanged'] },
        } as unknown as WorkspaceSwitchResult,
        'folderSwitch',
        { gameId: 'game-unknown-epoch', onSyncError },
      );
      await vi.advanceTimersByTimeAsync(1);
      expect(onSyncError).toHaveBeenCalledTimes(1);
      expect(publishRuntimeDescriptor).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(250);
      await expect(completion).resolves.toBeUndefined();
      expect(getWorkspaceSwitchSnapshotCommand).toHaveBeenCalledTimes(2);
      expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
      appState.activeGameId = 'game-1';
    }
  });

  it('retries cancellation failure without abandoning the shared receipt owner', async () => {
    vi.useFakeTimers();
    try {
      cancelRuntimeDescriptorQueries.mockRejectedValueOnce(new Error('cancellation interrupted'));
      const client = new QueryClient();
      const completion = applyWorkspaceSwitchEffects(
        client,
        {
          status: 'applied',
          primary_path: 'E:/Mods/A',
          impact: { rewrites: [], refresh_scopes: ['workspaceChanged'] },
        } as unknown as WorkspaceSwitchResult,
        'folderSwitch',
        { gameId: 'game-1' },
      );
      await vi.advanceTimersByTimeAsync(1);
      expect(publishRuntimeDescriptor).not.toHaveBeenCalled();
      const second = applyWorkspaceSwitchEffects(
        client,
        {
          status: 'applied',
          primary_path: 'E:/Mods/B',
          impact: { rewrites: [], refresh_scopes: ['collectionsChanged'] },
        } as unknown as WorkspaceSwitchResult,
        'folderSwitch',
        { gameId: 'game-1' },
      );
      expect(second).toBe(completion);
      await vi.advanceTimersByTimeAsync(250);
      await expect(completion).resolves.toBeUndefined();
      expect(cancelRuntimeDescriptorQueries).toHaveBeenCalledTimes(3);
      expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it('revalidates a new receipt epoch instead of permanently trusting a cached old root', async () => {
    getWorkspaceSwitchSnapshotCommand.mockResolvedValueOnce({
      game_id: 'game-1',
      source_epoch: 'cached-old-root',
      projected_revision: 5,
    });
    await primeWorkspaceRootEpoch('game-1');
    getWorkspaceSwitchSnapshotCommand.mockResolvedValue({
      game_id: 'game-1',
      source_epoch: 'verified-new-root',
      projected_revision: 60,
    });
    const settled = applyWorkspaceSwitchEffects(
      new QueryClient(),
      {
        status: 'applied',
        primary_path: 'E:/Mods/A',
        source_epoch: 'verified-new-root',
        disk_revision: 60,
        impact: { rewrites: [], refresh_scopes: ['workspaceChanged'] },
      } as unknown as WorkspaceSwitchResult,
      'folderSwitch',
      { gameId: 'game-1' },
    );
    await expect(settled).resolves.toBeUndefined();
    expect(getWorkspaceSwitchSnapshotCommand).toHaveBeenCalledTimes(2);
    expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(1);
  });

  it('waits for the latest revision when another receipt arrives during an active refresh', async () => {
    let releaseRefresh!: () => void;
    publishRuntimeDescriptor.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          releaseRefresh = resolve;
        }),
    );
    const client = new QueryClient();
    const receipt = (revision: number) =>
      ({
        status: 'applied',
        primary_path: 'E:/Mods/A',
        disk_revision: revision,
        source_epoch: 'root-a',
        impact: { rewrites: [], refresh_scopes: ['workspaceChanged'] },
      }) as unknown as WorkspaceSwitchResult;
    recordWorkspaceProjectedRevision('game-1', 51, 'root-a');
    getWorkspaceSwitchSnapshotCommand.mockResolvedValue({
      game_id: 'game-1',
      source_epoch: 'root-a',
      projected_revision: 51,
    });
    const first = applyWorkspaceSwitchEffects(client, receipt(51), 'folderSwitch', {
      gameId: 'game-1',
    });
    await vi.waitFor(() => expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(1));
    const second = applyWorkspaceSwitchEffects(client, receipt(52), 'folderSwitch', {
      gameId: 'game-1',
    });
    expect(second).toBe(first);
    releaseRefresh();
    recordWorkspaceProjectedRevision('game-1', 52, 'root-a');
    await Promise.all([first, second]);
    expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(2);
  });

  it('retries a transient projection snapshot error without losing the disk receipt', async () => {
    getWorkspaceSwitchSnapshotCommand
      .mockRejectedValueOnce(new Error('database busy'))
      .mockResolvedValueOnce({
        game_id: 'game-1',
        source_epoch: 'root-a',
        disk_revision: 100_000,
        projected_revision: 100_000,
      });

    await expect(waitForWorkspaceProjection('game-1', 100_000)).resolves.toBeUndefined();
    expect(getWorkspaceSwitchSnapshotCommand).toHaveBeenCalledTimes(2);
  });

  it('stops shared refresh at a repair hole without acknowledging merged later receipts', async () => {
    vi.useFakeTimers();
    try {
      appState.activeGameId = 'game-repair-hole';
      getWorkspaceSwitchSnapshotCommand.mockResolvedValue({
        game_id: 'game-repair-hole',
        source_epoch: 'repair-root',
        projected_revision: 9,
        projection_repair_reason: 'Folder ownership needs repair before synchronization can finish',
      });
      const client = new QueryClient();
      const onSyncError = vi.fn();
      const onOtherSyncError = vi.fn();
      const receipt = (revision: number) =>
        ({
          status: 'applied',
          primary_path: `E:/Mods/repair-${revision}`,
          source_epoch: 'repair-root',
          disk_revision: revision,
          impact: { rewrites: [], refresh_scopes: ['workspaceChanged'] },
        }) as unknown as WorkspaceSwitchResult;
      const first = applyWorkspaceSwitchEffects(client, receipt(10), 'folderSwitch', {
        gameId: 'game-repair-hole',
        onSyncError,
      });
      const failed = expect(first).rejects.toThrow('Folder ownership needs repair');
      const second = applyWorkspaceSwitchEffects(client, receipt(11), 'folderSwitch', {
        gameId: 'game-repair-hole',
        onSyncError: onOtherSyncError,
      });
      expect(second).toBe(first);
      await vi.advanceTimersByTimeAsync(1);
      await failed;
      expect(onSyncError).toHaveBeenCalledOnce();
      expect(onOtherSyncError).toHaveBeenCalledOnce();
      expect(toastWarning).toHaveBeenCalledExactlyOnceWith(
        'Changes were applied on disk, but workspace synchronization needs repair.',
        7000,
      );
      expect(onSyncError.mock.calls[0]?.[0]).toBeInstanceOf(Error);
      expect(publishRuntimeDescriptor).not.toHaveBeenCalled();
      const repeatedRepair = applyWorkspaceSwitchEffects(client, receipt(12), 'folderSwitch', {
        gameId: 'game-repair-hole',
      });
      const repeatedFailure = expect(repeatedRepair).rejects.toThrow(
        'Folder ownership needs repair',
      );
      await vi.advanceTimersByTimeAsync(1);
      await repeatedFailure;
      expect(toastWarning).toHaveBeenCalledOnce();
      const snapshots = getWorkspaceSwitchSnapshotCommand.mock.calls.length;
      await vi.advanceTimersByTimeAsync(60_000);
      expect(getWorkspaceSwitchSnapshotCommand).toHaveBeenCalledTimes(snapshots);

      getWorkspaceSwitchSnapshotCommand.mockResolvedValue({
        game_id: 'game-repair-hole',
        source_epoch: 'repair-root',
        projected_revision: 11,
        projection_repair_reason: null,
      });
      const recovered = applyWorkspaceSwitchEffects(client, receipt(11), 'folderSwitch', {
        gameId: 'game-repair-hole',
      });
      await vi.advanceTimersByTimeAsync(1);
      await expect(recovered).resolves.toBeUndefined();
      expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
      appState.activeGameId = 'game-1';
    }
  });

  it('retains recently renewed repair-warning keys when the warning cache reaches capacity', async () => {
    vi.useFakeTimers();
    try {
      const client = new QueryClient();
      const failWithRepair = async (index: number) => {
        const gameId = `warning-capacity-${index}`;
        const sourceEpoch = `${gameId}-root`;
        appState.activeGameId = gameId;
        getWorkspaceSwitchSnapshotCommand.mockResolvedValue({
          game_id: gameId,
          source_epoch: sourceEpoch,
          projected_revision: 0,
          projection_repair_reason: 'Folder ownership needs repair',
        });
        await expect(
          applyWorkspaceSwitchEffects(
            client,
            {
              status: 'applied',
              source_epoch: sourceEpoch,
              disk_revision: 1,
              impact: { rewrites: [], refresh_scopes: ['workspaceChanged'] },
            } as unknown as WorkspaceSwitchResult,
            'folderSwitch',
            { gameId },
          ),
        ).rejects.toThrow('Folder ownership needs repair');
      };
      for (let index = 0; index < 64; index += 1) await failWithRepair(index);
      expect(toastWarning).toHaveBeenCalledTimes(64);
      await vi.advanceTimersByTimeAsync(30_000);
      await failWithRepair(0);
      await failWithRepair(64);
      expect(toastWarning).toHaveBeenCalledTimes(66);
      await failWithRepair(0);
      expect(toastWarning).toHaveBeenCalledTimes(66);
    } finally {
      appState.activeGameId = 'game-1';
      vi.useRealTimers();
    }
  });

  it('keeps the disk receipt until the database actually catches up', async () => {
    vi.useFakeTimers();
    try {
      const waiting = waitForWorkspaceProjection('game-1', 9_000_000);
      await vi.advanceTimersByTimeAsync(11_000);
      let settled = false;
      void waiting.then(() => {
        settled = true;
      });
      expect(settled).toBe(false);
      recordWorkspaceProjectedRevision('game-1', 9_000_000);
      await vi.advanceTimersByTimeAsync(2_000);
      await expect(waiting).resolves.toBeUndefined();
    } finally {
      vi.useRealTimers();
    }
  });

  it('does not launch overlapping native snapshots when one stalls', async () => {
    vi.useFakeTimers();
    try {
      let completeSnapshot!: (value: unknown) => void;
      getWorkspaceSwitchSnapshotCommand.mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            completeSnapshot = resolve;
          }),
      );
      const waiting = waitForWorkspaceProjection('game-1', 9_000_001);
      await vi.advanceTimersByTimeAsync(15_000);
      expect(getWorkspaceSwitchSnapshotCommand).toHaveBeenCalledTimes(1);
      recordWorkspaceProjectedRevision('game-1', 9_000_001);
      await expect(waiting).resolves.toBeUndefined();
      completeSnapshot({
        game_id: 'game-1',
        source_epoch: 'root-a',
        projected_revision: 9_000_001,
      });
      await vi.advanceTimersByTimeAsync(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it('stops a background projection wait after switching games', async () => {
    vi.useFakeTimers();
    try {
      const waiting = waitForWorkspaceProjection('game-1', 9_000_002);
      appState.activeGameId = 'game-2';
      await vi.advanceTimersByTimeAsync(3_000);
      await expect(waiting).resolves.toBeUndefined();
    } finally {
      vi.useRealTimers();
    }
  });

  it('invalidates the affected health report once across an enable rewrite', async () => {
    const queryClient = new QueryClient();
    const invalidateQueries = vi.spyOn(queryClient, 'invalidateQueries');
    const result = {
      status: 'applied',
      changed_folder_paths: ['E:/Mods/A/DISABLED Blue', 'E:/Mods/A/Blue'],
      impact: { rewrites: [], refresh_scopes: [] },
    } as unknown as WorkspaceSwitchResult;

    applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', { gameId: 'game-1' });

    await vi.waitFor(() => {
      expect(invalidateQueries).toHaveBeenCalledTimes(1);
      expect(invalidateQueries).toHaveBeenCalledWith({
        queryKey: ['mod-health', 'report', 'game-1', 'e:/mods/a/blue'],
        refetchType: 'active',
      });
    });
  });

  it('returns after committed effects without waiting for background revalidation', async () => {
    let finishCancellation!: () => void;
    let finishRefresh!: () => void;
    cancelRuntimeDescriptorQueries.mockReturnValue(
      new Promise<void>((resolve) => {
        finishCancellation = resolve;
      }),
    );
    publishRuntimeDescriptor.mockReturnValue(
      new Promise<void>((resolve) => {
        finishRefresh = resolve;
      }),
    );
    const queryClient = new QueryClient();
    const cancelQueries = vi.spyOn(queryClient, 'cancelQueries');
    const result = {
      status: 'applied',
      changed_folder_paths: ['E:/Mods/A/Blue'],
      impact: {
        rewrites: [],
        refresh_scopes: ['workspaceChanged'],
      },
    } as unknown as WorkspaceSwitchResult;

    const settled = applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
      gameId: 'game-1',
    });
    expect(settled).toBeInstanceOf(Promise);
    expect(cancelRuntimeDescriptorQueries).toHaveBeenCalled();
    expect(publishRuntimeDescriptor).not.toHaveBeenCalled();

    finishCancellation();
    await vi.waitFor(() => {
      expect(cancelQueries).toHaveBeenCalled();
      expect(publishRuntimeDescriptor).toHaveBeenCalled();
    });
    finishRefresh();
    await settled;
  });

  it('ignores a completed switch after another game became active', async () => {
    appState.activeGameId = 'game-2';
    const queryClient = new QueryClient();
    const invalidateQueries = vi.spyOn(queryClient, 'invalidateQueries');
    const result = {
      status: 'applied',
      changed_folder_paths: ['E:/Games/One/Mods/A'],
      impact: {
        rewrites: [{ old_path: 'E:/Games/One/Mods/DISABLED A', new_path: 'E:/Games/One/Mods/A' }],
        refresh_scopes: [],
      },
    } as unknown as WorkspaceSwitchResult;

    applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', { gameId: 'game-1' });

    expect(invalidateQueries).not.toHaveBeenCalled();
    expect(publishRuntimeDescriptor).not.toHaveBeenCalled();
  });
});
