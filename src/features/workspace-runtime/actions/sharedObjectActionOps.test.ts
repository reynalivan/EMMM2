import { beforeEach, describe, expect, it, vi } from 'vitest';
import { applyObjectCategoryAndRefresh } from './sharedObjectActionOps';
import { GameType } from '@/entities/game';

const setModCategory = vi.fn();
const setObjectModsCategory = vi.fn();
const publishRuntimeDescriptor = vi.fn();

vi.mock('../../../shared/api/tauri/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    setModCategory: (...args: unknown[]) => setModCategory(...args),
    setObjectModsCategory: (...args: unknown[]) => setObjectModsCategory(...args),
  },
}));

vi.mock('@/shared/lib/queryRefresh', () => ({
  publishRuntimeDescriptor: (...args: unknown[]) => publishRuntimeDescriptor(...args),
}));

describe('shared object action operations', () => {
  beforeEach(() => {
    setModCategory.mockReset();
    setModCategory.mockResolvedValue(undefined);
    setObjectModsCategory.mockReset();
    setObjectModsCategory.mockResolvedValue(2);
    publishRuntimeDescriptor.mockReset();
    publishRuntimeDescriptor.mockResolvedValue(undefined);
  });

  it('updates object category, propagates to child mods, and refreshes runtime', async () => {
    const mutateAsync = vi.fn().mockResolvedValue(undefined);

    await applyObjectCategoryAndRefresh({
      activeGame: {
        id: 'game-1',
        name: 'Game',
        game_type: GameType.GIMI,
        mod_path: 'E:/Mods',
        game_exe: 'E:/Games/Game/Game.exe',
        loader_exe: null,
        launch_args: null,
      },
      objectId: 'object-1',
      category: 'Weapon',
      itemType: 'object',
      queryClient: {} as never,
      updateObject: {
        mutateAsync,
      },
    });

    expect(mutateAsync).not.toHaveBeenCalled();
    expect(setObjectModsCategory).toHaveBeenCalledWith('game-1', 'object-1', 'Weapon');
    expect(setModCategory).not.toHaveBeenCalled();
    expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(1);
  });
});
