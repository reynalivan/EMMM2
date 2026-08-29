import { describe, expect, it, vi, beforeEach } from 'vitest';
import { moveModsToObjectAndRefresh } from './sharedOperations';

const moveModsToObject = vi.fn();
const applyRuntimeEffects = vi.fn();
const applyRuntimeMutationResult = vi.fn();
const notifyCommittedMutationSyncWarning = vi.fn();

vi.mock('../../../lib/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    moveModsToObject: (...args: unknown[]) => moveModsToObject(...args),
  },
}));

vi.mock('../../workspace-runtime/optimistic/applyOptimisticEffects', () => ({
  applyRuntimeEffects: (...args: unknown[]) => applyRuntimeEffects(...args),
}));

vi.mock('../../workspace-runtime/actions/sharedRuntimeResultMapper', () => ({
  applyRuntimeMutationResult: (...args: unknown[]) => applyRuntimeMutationResult(...args),
}));

vi.mock('../../../hooks/folderCache', () => ({}));

vi.mock('../../../lib/committedMutationWarning', () => ({
  notifyCommittedMutationSyncWarning: (...args: unknown[]) =>
    notifyCommittedMutationSyncWarning(...args),
}));

describe('shared mod runtime operations', () => {
  beforeEach(() => {
    moveModsToObject.mockReset();
    moveModsToObject.mockResolvedValue({
      success: [],
      successes: [],
      failures: [],
      path_rewrites: [{ old_path: 'Mods/Diluc/mod-a', new_path: 'Mods/Kaeya/mod-a' }],
    });
    applyRuntimeEffects.mockReset();
    applyRuntimeMutationResult.mockReset();
    applyRuntimeMutationResult.mockResolvedValue(undefined);
  });

  it('moves mods to object and publishes runtime refresh', async () => {
    await moveModsToObjectAndRefresh({
      queryClient: {} as never,
      gameId: 'game-1',
      folderPaths: ['Mods/Diluc/mod-a'],
      targetObjectId: 'object-2',
      targetSubpath: null,
      status: 'disabled',
    });

    expect(moveModsToObject).toHaveBeenCalledWith({
      game_id: 'game-1',
      folder_paths: ['Mods/Diluc/mod-a'],
      target_object_id: 'object-2',
      target_subpath: null,
      status: 'disabled',
    });
    expect(applyRuntimeEffects).toHaveBeenCalledTimes(1);
    expect(applyRuntimeMutationResult).toHaveBeenCalledWith({} as never, 'workspaceStructure');
  });

  it('publishes a refresh for successful moves before surfacing a partial failure', async () => {
    const partialFailure = new Error('Target folder is locked');
    const queryClient = {} as never;
    moveModsToObject.mockResolvedValueOnce({
      success: ['Mods/Kaeya/mod-a'],
      successes: ['Mods/Kaeya/mod-a'],
      failures: [{ path: 'Mods/Diluc/mod-b', error: partialFailure }],
      path_rewrites: [{ old_path: 'Mods/Diluc/mod-a', new_path: 'Mods/Kaeya/mod-a' }],
    });

    await expect(
      moveModsToObjectAndRefresh({
        queryClient,
        gameId: 'game-1',
        folderPaths: ['Mods/Diluc/mod-a', 'Mods/Diluc/mod-b'],
        targetObjectId: 'object-2',
        targetSubpath: null,
        status: 'disabled',
      }),
    ).rejects.toBe(partialFailure);

    expect(applyRuntimeEffects).toHaveBeenCalledTimes(1);
    expect(applyRuntimeMutationResult).toHaveBeenCalledWith(queryClient, 'workspaceStructure');
  });

  it('publishes successful move effects before presenting projection lag', async () => {
    moveModsToObject.mockResolvedValueOnce({
      success: ['Mods/Kaeya/mod-a'],
      failures: [],
      path_rewrites: [{ old_path: 'Mods/Diluc/mod-a', new_path: 'Mods/Kaeya/mod-a' }],
      sync_warning: { kind: 'ReconcileFailed', message: 'projection pending' },
    });

    await moveModsToObjectAndRefresh({
      queryClient: {} as never,
      gameId: 'game-1',
      folderPaths: ['Mods/Diluc/mod-a'],
      targetObjectId: 'object-2',
      targetSubpath: null,
      status: 'disabled',
    });

    expect(applyRuntimeMutationResult).toHaveBeenCalled();
    expect(notifyCommittedMutationSyncWarning).toHaveBeenCalledWith(
      expect.objectContaining({ sync_warning: expect.any(Object) }),
    );
  });
});
