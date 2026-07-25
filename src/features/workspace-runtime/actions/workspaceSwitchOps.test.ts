import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { WorkspaceExplorerNode, WorkspaceImpact } from '../../../types/workspace';
import {
  buildExplorerSwitchEffectDescriptor,
  buildNodePendingKey,
  buildSwitchRefreshDescriptor,
  executeWorkspaceSwitch,
  isWorkspaceObjectNode,
  parseRenameConflict,
  stripModsRoot,
  togglePendingKey,
} from './workspaceSwitchOps';

const executeWorkspaceSwitchCommand = vi.fn();
const openWorkspaceConflictDialog = vi.fn();
const openWorkspaceFileInUseDialog = vi.fn();
const toastError = vi.fn();

vi.mock('../../../lib/bindings', () => ({
  commands: {
    executeWorkspaceSwitch: (...args: unknown[]) => executeWorkspaceSwitchCommand(...args),
  },
}));

vi.mock('../state/workspaceDialogs', () => ({
  openWorkspaceConflictDialog: (...args: unknown[]) => openWorkspaceConflictDialog(...args),
  openWorkspaceFileInUseDialog: (...args: unknown[]) => openWorkspaceFileInUseDialog(...args),
}));

vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    error: (...args: unknown[]) => toastError(...args),
  },
}));

vi.mock('../../runtime-sync/queryRefresh', () => ({
  publishRuntimeDescriptor: vi.fn(),
}));

function createExplorerNode(overrides: Partial<WorkspaceExplorerNode> = {}): WorkspaceExplorerNode {
  return {
    node_kind: 'terminal_mod',
    path: 'E:/Mods/ALBEDO/Variant',
    is_enabled: false,
    owner_object_id: 'object-1',
    ...overrides,
  } as unknown as WorkspaceExplorerNode;
}

describe('workspace switch ops', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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

  describe('stripModsRoot', () => {
    it('normalizes separators and strips the mods root prefix', () => {
      expect(stripModsRoot('E:\\Mods\\ALBEDO\\Variant', 'E:/Mods')).toBe('ALBEDO/Variant');
    });

    it('keeps the path when it is the root itself or outside it', () => {
      expect(stripModsRoot('E:/Mods', 'E:/Mods')).toBe('E:/Mods');
      expect(stripModsRoot('E:/Other/A', 'E:/Mods')).toBe('E:/Other/A');
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

  describe('buildExplorerSwitchEffectDescriptor', () => {
    const impact = { rewrites: [], refresh_scopes: [] } as unknown as WorkspaceImpact;

    it('adds an owner object count delta when a terminal mod flips state', () => {
      const descriptor = buildExplorerSwitchEffectDescriptor(createExplorerNode(), true, impact);

      expect(descriptor.objectCountDeltas).toEqual([{ objectId: 'object-1', delta: 1 }]);
      expect(descriptor.removedQueryKeys).toHaveLength(1);
    });

    it('skips the delta when the node is already in the desired state', () => {
      const descriptor = buildExplorerSwitchEffectDescriptor(
        createExplorerNode({ is_enabled: true }),
        true,
        impact,
      );

      expect(descriptor.objectCountDeltas).toEqual([]);
    });

    it('skips the delta for non terminal mods', () => {
      const descriptor = buildExplorerSwitchEffectDescriptor(
        createExplorerNode({ node_kind: 'container' }),
        true,
        impact,
      );

      expect(descriptor.objectCountDeltas).toEqual([]);
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

    it('routes rename conflicts to the conflict dialog', async () => {
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
      expect(openWorkspaceConflictDialog).toHaveBeenCalledTimes(1);
      expect(toastError).not.toHaveBeenCalled();
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
      expect(openWorkspaceConflictDialog).not.toHaveBeenCalled();
    });
  });
});
