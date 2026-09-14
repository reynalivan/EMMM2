import { act, renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useFolderGridBulk } from './useFolderGridBulk';

const {
  bulkToggleMutate,
  bulkDeleteMutate,
  bulkUpdateInfoMutate,
  bulkSafetyMutate,
  bulkFavoriteMutate,
  bulkPinMutate,
} = vi.hoisted(() => ({
  bulkToggleMutate: vi.fn(),
  bulkDeleteMutate: vi.fn(),
  bulkUpdateInfoMutate: vi.fn(),
  bulkSafetyMutate: vi.fn(),
  bulkFavoriteMutate: vi.fn(),
  bulkPinMutate: vi.fn(),
}));

vi.mock('@/features/mod-runtime', () => ({
  useBulkToggle: () => ({ mutate: bulkToggleMutate }),
  useBulkDelete: () => ({ mutate: bulkDeleteMutate }),
  useBulkUpdateInfo: () => ({ mutate: bulkUpdateInfoMutate }),
  useBulkSafety: () => ({ mutate: bulkSafetyMutate }),
  useBulkFavorite: () => ({ mutate: bulkFavoriteMutate }),
  useBulkPin: () => ({ mutate: bulkPinMutate }),
}));

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: { id: 'game-1' } }),
}));

describe('useFolderGridBulk', () => {
  it('keeps bulk action callbacks stable when mutation result wrappers change', () => {
    const options = {
      gridSelection: new Set(['C:/Mods/Alice/Blue']),
      sortedFolders: [],
      clearGridSelection: vi.fn(),
      openMoveDialog: vi.fn(),
    };
    const { result, rerender } = renderHook((props) => useFolderGridBulk(props), {
      initialProps: options,
    });
    const initialToggle = result.current.handleBulkToggle;
    const initialFavorite = result.current.handleBulkFavorite;

    rerender(options);

    expect(result.current.handleBulkToggle).toBe(initialToggle);
    expect(result.current.handleBulkFavorite).toBe(initialFavorite);

    act(() => {
      result.current.handleBulkToggle(true);
    });
    expect(bulkToggleMutate).toHaveBeenCalledWith({
      gameId: 'game-1',
      paths: ['C:/Mods/Alice/Blue'],
      enable: true,
    });
  });
});
