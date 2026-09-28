import { renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useFolderGridViewModel } from './useFolderGridViewModel';

const { state } = vi.hoisted(() => ({
  state: {
    activeGameId: 'game-1',
    activePane: 'folderGrid',
    diskReconcileByGame: {},
    gameActivationByGame: {
      'game-1': { phase: 'ready' },
    } as Record<string, { phase: string }>,
    folderConflictsByGame: {},
    renameConfirmationsByGame: {},
    isIgnoreManagementOpen: false,
    setActivePane: vi.fn(),
    setIgnoreManagementOpen: vi.fn(),
  },
}));

vi.mock('@/app/store', () => ({
  useAppStore: (selector: (value: typeof state) => unknown) => selector(state),
}));

vi.mock('./useFolderMutations', () => ({
  useActiveConflicts: () => ({ data: [] }),
}));

describe('useFolderGridViewModel', () => {
  afterEach(() => {
    state.gameActivationByGame['game-1'] = { phase: 'ready' };
  });

  it('does not keep a ready game locked by an older workspace syncing snapshot', () => {
    const { result } = renderHook(() =>
      useFolderGridViewModel({
        sortedFolders: [],
        sourceUnavailableMessage: null,
        recoveryStatus: 'syncing',
      }),
    );

    expect(result.current.recoveryStatus).toBe('ready');
    expect(result.current.mutationsDisabled).toBe(false);
  });

  it('keeps mutations locked while the current activation is still syncing', () => {
    state.gameActivationByGame['game-1'].phase = 'syncing';
    const { result } = renderHook(() =>
      useFolderGridViewModel({
        sortedFolders: [],
        sourceUnavailableMessage: null,
        recoveryStatus: 'ready',
      }),
    );

    expect(result.current.recoveryStatus).toBe('syncing');
    expect(result.current.mutationsDisabled).toBe(true);
  });

  it('keeps mutations locked when the active game has no activation proof', () => {
    delete state.gameActivationByGame['game-1'];
    const { result } = renderHook(() =>
      useFolderGridViewModel({
        sortedFolders: [],
        sourceUnavailableMessage: null,
        recoveryStatus: 'ready',
      }),
    );

    expect(result.current.recoveryStatus).toBe('syncing');
    expect(result.current.mutationsDisabled).toBe(true);
  });
});
