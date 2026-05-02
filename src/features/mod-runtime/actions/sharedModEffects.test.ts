import { QueryClient } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '../../../stores/useAppStore';
import { runSharedModActiveContextToggle } from './sharedModEffects';

const toggleModSafeMock = vi.fn();
const updateFolderCacheMock = vi.fn();
const applyRuntimePathInvalidationMutationResultMock = vi.fn();
const toastSuccessMock = vi.fn();

vi.mock('../../../lib/bindings', () => ({
  commands: {
    toggleModSafe: (...args: unknown[]) => toggleModSafeMock(...args),
  },
}));

vi.mock('../../../hooks/folderCache', () => ({
  updateFolderCache: (...args: unknown[]) => updateFolderCacheMock(...args),
}));

vi.mock('../../workspace-runtime/actions/sharedRuntimeResultMapper', () => ({
  applyRuntimePathInvalidationMutationResult: (...args: unknown[]) =>
    applyRuntimePathInvalidationMutationResultMock(...args),
}));

vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
  },
}));

describe('runSharedModActiveContextToggle', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    toggleModSafeMock.mockResolvedValue(undefined);
    applyRuntimePathInvalidationMutationResultMock.mockResolvedValue(undefined);
    useAppStore.setState({
      safeMode: true,
      gridSelection: new Set(['E:/Mods/ALBEDO/Private Outfit']),
      selectedModPath: 'E:/Mods/ALBEDO/Private Outfit',
    });
  });

  it('invalidates the rewritten path after retagging across corridors', async () => {
    const queryClient = new QueryClient();
    const setNodeEnabled = vi.fn().mockResolvedValue('E:/Mods/DISABLED Private Outfit');

    await runSharedModActiveContextToggle({
      activeGameId: 'game-1',
      folder: {
        path: 'E:/Mods/ALBEDO/Private Outfit',
        name: 'Private Outfit',
        folder_name: 'Private Outfit',
        is_safe: false,
        is_enabled: true,
        node_type: 'FlatModRoot',
        classification_reasons: [],
        id: null,
        owner_object_id: null,
        owner_object_folder_path: null,
        is_directory: true,
        thumbnail_path: null,
        modified_at: 0,
        size_bytes: 0,
        has_info_json: false,
        is_favorite: false,
        is_misplaced: false,
        metadata: null,
        category: null,
        conflict_group_id: null,
        conflict_state: null,
        warnings: [],
      },
      queryClient,
      removeFromCurrentView: true,
      switchSurface: 'preview',
      switchActions: { setNodeEnabled },
      hasPin: false,
      safeMode: true,
      translate: (key: string) => key,
    });

    expect(toggleModSafeMock).toHaveBeenCalledWith({
      gameId: 'game-1',
      folderPath: 'E:/Mods/DISABLED Private Outfit',
      safe: true,
    });
    expect(applyRuntimePathInvalidationMutationResultMock).toHaveBeenCalledWith(
      queryClient,
      ['E:/Mods/DISABLED Private Outfit'],
      'workspaceStructure',
    );
  });
});
