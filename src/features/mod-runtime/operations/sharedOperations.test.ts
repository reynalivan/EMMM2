import { describe, expect, it, vi, beforeEach } from 'vitest';
import { moveModToObjectAndRefresh, syncExplorerAfterRename } from './sharedOperations';

const moveModToObject = vi.fn();
const publishRuntimeDescriptor = vi.fn();
const updateFolderCache = vi.fn();
const dispatchWorkspaceRuntimeEvent = vi.fn();

vi.mock('../../../lib/bindings', () => ({
  commands: {
    moveModToObject: (...args: unknown[]) => moveModToObject(...args),
  },
}));

vi.mock('../../runtime-sync/queryRefresh', () => ({
  publishRuntimeDescriptor: (...args: unknown[]) => publishRuntimeDescriptor(...args),
}));

vi.mock('../../../hooks/folderCache', () => ({
  updateFolderCache: (...args: unknown[]) => updateFolderCache(...args),
}));

vi.mock('../../workspace-runtime/state/workspaceStoreBridge', () => ({
  dispatchWorkspaceRuntimeEvent: (...args: unknown[]) => dispatchWorkspaceRuntimeEvent(...args),
}));

vi.mock('../../../stores/useAppStore', () => ({
  useAppStore: {
    getState: vi.fn(() => ({
      explorerSubPath: 'Objects/Diluc/Variants',
    })),
  },
}));

describe('shared mod runtime operations', () => {
  beforeEach(() => {
    moveModToObject.mockReset();
    moveModToObject.mockResolvedValue(undefined);
    publishRuntimeDescriptor.mockReset();
    publishRuntimeDescriptor.mockResolvedValue(undefined);
    updateFolderCache.mockReset();
    dispatchWorkspaceRuntimeEvent.mockReset();
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

    expect(moveModToObject).toHaveBeenCalledWith({
      gameId: 'game-1',
      folderPath: 'Mods/Diluc/mod-a',
      targetObjectId: 'object-2',
      status: 'disabled',
    });
    expect(updateFolderCache).toHaveBeenCalledWith(
      {} as never,
      ['Mods/Diluc/mod-a'],
      undefined,
      true,
    );
    expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(1);
  });

  it('rewrites explorer selection when rename touches current explorer path', () => {
    syncExplorerAfterRename('E:/Mods', 'E:/Mods/Objects/Diluc', 'E:/Mods/Objects/Diluc_Renamed');

    expect(dispatchWorkspaceRuntimeEvent).toHaveBeenCalledWith({
      type: 'PATHS_REWRITTEN',
      rewrites: [
        {
          oldPath: 'E:/Mods/Objects/Diluc',
          newPath: 'E:/Mods/Objects/Diluc_Renamed',
        },
      ],
    });
  });
});
