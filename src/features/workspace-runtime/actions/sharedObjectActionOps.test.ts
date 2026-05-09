import { beforeEach, describe, expect, it, vi } from 'vitest';
import { applyObjectCategoryAndRefresh, buildObjectSyncCurrentData } from './sharedObjectActionOps';
import { GameType } from '../../../types/game';

const setModCategory = vi.fn();
const setObjectModsCategory = vi.fn();
const publishRuntimeDescriptor = vi.fn();

vi.mock('../../../lib/bindings', () => ({
  commands: {
    setModCategory: (...args: unknown[]) => setModCategory(...args),
    setObjectModsCategory: (...args: unknown[]) => setObjectModsCategory(...args),
  },
}));

vi.mock('../../runtime-sync/queryRefresh', () => ({
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

  it('builds sync current data from workspace object or fallback name', () => {
    expect(
      buildObjectSyncCurrentData(
        {
          id: 'object-1',
          name: 'Diluc',
          display_name: 'Diluc',
          node_kind: 'object',
          display_mode: 'unknown',
          type_chip: null,
          object_type: 'Character',
          is_pinned: false,
          thumbnail_path: 'thumb.png',
          folder_path: 'Objects/Diluc',
          sub_category: null,
          mod_count: 2,
          enabled_count: 1,
          tags: '[]',
          metadata: '{}',
          is_auto_sync: false,
          is_object_disabled: false,
          status: 1,
          created_at: '2025-01-01T00:00:00Z',
          hash_db: null,
          custom_skins: null,
          has_naming_conflict: false,
          inactive_reason: null,
          is_effectively_active: true,
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
        },
        'Fallback',
      ),
    ).toEqual({
      name: 'Diluc',
      object_type: 'Character',
      metadata: null,
      thumbnail_path: 'thumb.png',
    });

    expect(buildObjectSyncCurrentData(undefined, 'Fallback')).toEqual({
      name: 'Fallback',
      object_type: '',
      metadata: null,
      thumbnail_path: null,
    });
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

    expect(mutateAsync).toHaveBeenCalledWith({
      id: 'object-1',
      updates: { object_type: 'Weapon' },
    });
    expect(setObjectModsCategory).toHaveBeenCalledWith({
      gameId: 'game-1',
      objectId: 'object-1',
      category: 'Weapon',
    });
    expect(setModCategory).not.toHaveBeenCalled();
    expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(1);
  });
});
