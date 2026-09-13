import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createWrapper } from '../../../tests/testing/test-utils';
import { useFolderGridActions } from './useFolderGridActions';

const { createModFolder, applyRuntimeMutationResult, clearGridSelection } = vi.hoisted(() => ({
  createModFolder: vi.fn(),
  applyRuntimeMutationResult: vi.fn(),
  clearGridSelection: vi.fn(),
}));

vi.mock('../../../shared/api/tauri/bindings', () => ({
  commands: {
    createModFolder,
    openInExplorer: vi.fn(),
    revealObjectInExplorer: vi.fn(),
  },
}));

vi.mock('@/features/mod-runtime', () => ({
  useSharedModActions: () => ({}),
}));

vi.mock('@/features/workspace-runtime', () => ({
  applyRuntimeMutationResult,
  closeWorkspaceDialog: vi.fn(),
  openWorkspaceEnableParentDialog: vi.fn(),
  useWorkspaceRuntimeSelector: () => ({ kind: 'none' }),
  useWorkspaceSwitchActions: () => ({
    isPending: false,
    isNodePending: false,
    setFolderPathEnabled: vi.fn(),
  }),
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

describe('useFolderGridActions create folder', () => {
  beforeEach(() => {
    createModFolder.mockResolvedValue('E:/Mods/Objects/Diluc/Variants');
    applyRuntimeMutationResult.mockResolvedValue(undefined);
    clearGridSelection.mockClear();
    createModFolder.mockClear();
    applyRuntimeMutationResult.mockClear();
  });

  it('keeps the canonical parent and game selected when the dialog opened', async () => {
    const initialOptions = {
      activeGame: {
        id: 'game-a',
        name: 'Game A',
        game_type: 0,
        mod_path: 'E:/Mods',
        game_exe: null,
        loader_exe: null,
        launch_args: null,
      },
      explorerSubPath: 'Objects/Diluc',
      ancestorDisabledBy: null,
      ancestorDisabledPath: null,
      rawFolders: [],
      objects: [],
      clearGridSelection,
      sourceAvailable: true,
    };
    const { result, rerender } = renderHook((options) => useFolderGridActions(options), {
      initialProps: initialOptions,
      wrapper: createWrapper,
    });

    act(() => {
      result.current.openCreateFolderDialog();
    });
    rerender({
      ...initialOptions,
      activeGame: {
        ...initialOptions.activeGame,
        id: 'game-b',
        name: 'Game B',
        mod_path: 'E:/Other',
      },
      explorerSubPath: 'Other',
    });

    await act(async () => {
      await result.current.handleCreateFolder('Variants');
    });

    expect(createModFolder).toHaveBeenCalledWith('Objects/Diluc', 'Variants', 'game-a');
    expect(clearGridSelection).toHaveBeenCalledOnce();
    expect(applyRuntimeMutationResult).toHaveBeenCalledWith(
      expect.anything(),
      'workspaceStructure',
    );
  });
});
