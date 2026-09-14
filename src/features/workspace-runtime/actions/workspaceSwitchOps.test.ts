import { beforeEach, describe, expect, it, vi } from 'vitest';
import { QueryClient } from '@tanstack/react-query';
import type { WorkspaceImpact, WorkspaceSwitchResult } from '@/entities/workspace';
import {
  applyWorkspaceSwitchEffects,
  buildNodePendingKey,
  buildSwitchRefreshDescriptor,
  executeWorkspaceSwitch,
  isWorkspaceObjectNode,
  parseRenameConflict,
  togglePendingKey,
} from './workspaceSwitchOps';

const executeWorkspaceSwitchCommand = vi.fn();
const reconcileDiskStateCommand = vi.fn();
const openFolderConflictManagerDialog = vi.fn();
const openRenameConfirmationDialog = vi.fn();
const openWorkspaceFileInUseDialog = vi.fn();
const applyFolderConflictReconcileResult = vi.fn(() => true);
const setRenameConfirmations = vi.fn();
const toastError = vi.fn();
const toastInfo = vi.fn();
const notifyCommittedMutationSyncWarning = vi.fn();

vi.mock('../../../shared/api/tauri/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    executeWorkspaceSwitch: (...args: unknown[]) => executeWorkspaceSwitchCommand(...args),
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
    getState: () => ({ applyFolderConflictReconcileResult, setRenameConfirmations }),
  },
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    error: (...args: unknown[]) => toastError(...args),
    info: (...args: unknown[]) => toastInfo(...args),
  },
}));

vi.mock('@/shared/lib/queryRefresh', () => ({
  publishRuntimeDescriptor: vi.fn(),
  publishQueryInvalidations: vi.fn(),
}));

vi.mock('../../../shared/lib/committedMutationWarning', () => ({
  notifyCommittedMutationSyncWarning: (...args: unknown[]) =>
    notifyCommittedMutationSyncWarning(...args),
}));

describe('workspace switch ops', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    reconcileDiskStateCommand.mockResolvedValue(null);
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

  it('returns a committed switch and presents terminal projection lag', async () => {
    executeWorkspaceSwitchCommand.mockResolvedValue({
      status: 'applied',
      sync_warning: { kind: 'ReconcileFailed', message: 'projection pending' },
    });

    await expect(executeWorkspaceSwitch({ game_id: 'game-1' } as never)).resolves.toEqual(
      expect.objectContaining({ status: 'applied' }),
    );
    expect(notifyCommittedMutationSyncWarning).toHaveBeenCalledWith(
      expect.objectContaining({ sync_warning: expect.any(Object) }),
    );
  });

  describe('node identity', () => {
    it('keys object nodes by id and folder nodes by path', () => {
      expect(buildNodePendingKey({ node_kind: 'object', id: 'o1' } as never)).toBe('object:o1');
      expect(buildNodePendingKey({ node_kind: 'terminal_mod', path: 'a/b' } as never)).toBe(
        'folder:a/b',
      );
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
    const input = {
      game_id: 'game-1',
      target: { kind: 'mod_path', value: 'E:/Mods/A' },
      desired_enabled: true,
      resolution: 'normal',
      origin_surface: 'folder_grid',
    } as never;

    it('returns the switch result on success', async () => {
      executeWorkspaceSwitchCommand.mockResolvedValue({ primary_path: 'E:/Mods/A' });

      await expect(executeWorkspaceSwitch(input)).resolves.toEqual({ primary_path: 'E:/Mods/A' });
      expect(toastError).not.toHaveBeenCalled();
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
  });

  it('keeps an applied disk switch silent after updating the workspace', async () => {
    const result = {
      status: 'applied',
      impact: { rewrites: [], refresh_scopes: [] },
    } as unknown as WorkspaceSwitchResult;

    await applyWorkspaceSwitchEffects(new QueryClient(), result, 'folderSwitch');

    expect(toastInfo).not.toHaveBeenCalled();
  });

  it('invalidates the affected health report once across an enable rewrite', async () => {
    const queryClient = new QueryClient();
    const invalidateQueries = vi.spyOn(queryClient, 'invalidateQueries');
    const result = {
      status: 'applied',
      changed_folder_paths: ['E:/Mods/A/DISABLED Blue', 'E:/Mods/A/Blue'],
      impact: { rewrites: [], refresh_scopes: [] },
    } as unknown as WorkspaceSwitchResult;

    await applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', { gameId: 'game-1' });

    expect(invalidateQueries).toHaveBeenCalledTimes(1);
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['mod-health', 'report', 'game-1', 'e:/mods/a/blue'],
      refetchType: 'active',
    });
  });
});
