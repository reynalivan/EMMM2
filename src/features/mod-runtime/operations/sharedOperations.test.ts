import { describe, expect, it, vi, beforeEach } from 'vitest';
import { moveModToObjectAndRefresh } from './sharedOperations';

const moveModsToObject = vi.fn();
const applyRuntimeEffects = vi.fn();
const applyRuntimeMutationResult = vi.fn();
const updateFolderCache = vi.fn();

vi.mock('../../../lib/bindings', () => ({
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

vi.mock('../../../hooks/folderCache', () => ({
  updateFolderCache: (...args: unknown[]) => updateFolderCache(...args),
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
    updateFolderCache.mockReset();
  });

  it('moves mod to object and publishes runtime refresh', async () => {
    await moveModToObjectAndRefresh({
      queryClient: {} as never,
      gameId: 'game-1',
      folderPath: 'Mods/Diluc/mod-a',
      targetObjectId: 'object-2',
      status: 'disabled',
      removeFromCurrentView: true,
    });

    expect(moveModsToObject).toHaveBeenCalledWith({
      input: {
        game_id: 'game-1',
        folder_paths: ['Mods/Diluc/mod-a'],
        target_object_id: 'object-2',
        target_subpath: null,
        status: 'disabled',
      },
    });
    expect(updateFolderCache).toHaveBeenCalledWith(
      {} as never,
      ['Mods/Diluc/mod-a'],
      undefined,
      true,
    );
    expect(applyRuntimeEffects).toHaveBeenCalledTimes(1);
    expect(applyRuntimeMutationResult).toHaveBeenCalledWith({} as never, 'workspaceStructure');
  });
});
