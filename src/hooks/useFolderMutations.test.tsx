/**
 * These mutations resolve `gameId` from the active game rather than taking it
 * from the caller. Rust requires the field, so a payload that omits it fails
 * serde at runtime after type-checking cleanly — the defect class this asserts.
 */
import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  useDeleteModThumbnail,
  usePasteThumbnail,
  useToggleModSafe,
  useUpdateModInfo,
  useUpdateModThumbnail,
} from './useFolderMutations';

const updateModThumbnail = vi.fn();
const pasteThumbnail = vi.fn();
const updateModInfo = vi.fn();
const deleteModThumbnail = vi.fn();
const toggleModSafe = vi.fn();
const notifyCommittedMutationSyncWarning = vi.fn();

let activeGame: { id: string } | null = { id: 'game-1' };

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('../lib/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    updateModThumbnail: (...args: unknown[]) => updateModThumbnail(...args),
    pasteThumbnail: (...args: unknown[]) => pasteThumbnail(...args),
    updateModInfo: (...args: unknown[]) => updateModInfo(...args),
    deleteModThumbnail: (...args: unknown[]) => deleteModThumbnail(...args),
    toggleModSafe: (...args: unknown[]) => toggleModSafe(...args),
  },
}));

vi.mock('./useActiveGame', () => ({
  useActiveGame: () => ({ activeGame }),
}));

vi.mock('../lib/committedMutationWarning', () => ({
  notifyCommittedMutationSyncWarning: (...args: unknown[]) =>
    notifyCommittedMutationSyncWarning(...args),
}));

function wrapper({ children }: { children: React.ReactNode }) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return React.createElement(QueryClientProvider, { client: queryClient }, children);
}

beforeEach(() => {
  vi.clearAllMocks();
  activeGame = { id: 'game-1' };
});

describe('gameId injection', () => {
  it('sends gameId with the thumbnail source path', async () => {
    updateModThumbnail.mockResolvedValue('thumb.png');
    const { result } = renderHook(() => useUpdateModThumbnail(), { wrapper });

    await result.current.mutateAsync({ folderPath: 'C:/Mods/Ayaka', sourcePath: 'C:/pic.png' });

    expect(updateModThumbnail).toHaveBeenCalledWith('game-1', 'C:/Mods/Ayaka', 'C:/pic.png');
  });

  it('sends gameId with pasted thumbnail bytes', async () => {
    pasteThumbnail.mockResolvedValue('thumb.png');
    const { result } = renderHook(() => usePasteThumbnail(), { wrapper });

    await result.current.mutateAsync({ folderPath: 'C:/Mods/Ayaka', imageData: [1, 2, 3] });

    expect(pasteThumbnail).toHaveBeenCalledWith('game-1', 'C:/Mods/Ayaka', [1, 2, 3]);
  });

  it('sends gameId with the info.json update', async () => {
    updateModInfo.mockResolvedValue({});
    const { result } = renderHook(() => useUpdateModInfo(), { wrapper });

    await result.current.mutateAsync({
      folderPath: 'C:/Mods/Ayaka',
      update: { is_favorite: true },
    });

    expect(updateModInfo).toHaveBeenCalledWith('game-1', 'C:/Mods/Ayaka', { is_favorite: true });
  });

  it('sends gameId when deleting a thumbnail', async () => {
    deleteModThumbnail.mockResolvedValue(null);
    const { result } = renderHook(() => useDeleteModThumbnail(), { wrapper });

    await result.current.mutateAsync('C:/Mods/Ayaka');

    expect(deleteModThumbnail).toHaveBeenCalledWith('game-1', 'C:/Mods/Ayaka');
  });

  it('refreshes workspace, runtime, and collections after safety changes', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const invalidate = vi.spyOn(queryClient, 'invalidateQueries');
    toggleModSafe.mockResolvedValue({
      sync_warning: { kind: 'ReconcileFailed', message: 'projection pending' },
    });
    const { result } = renderHook(() => useToggleModSafe(), {
      wrapper: ({ children }) =>
        React.createElement(QueryClientProvider, { client: queryClient }, children),
    });

    await result.current.mutateAsync({
      gameId: 'game-1',
      folderPath: 'C:/Mods/Ayaka',
      safe: false,
    });

    expect(invalidate).toHaveBeenCalledWith({
      queryKey: ['workspace', 'mods'],
      refetchType: 'active',
    });
    expect(notifyCommittedMutationSyncWarning).toHaveBeenCalledWith(
      expect.objectContaining({ sync_warning: expect.any(Object) }),
    );
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: ['v2-collections'],
      refetchType: 'active',
    });
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: ['v2-collection-runtime'],
      refetchType: 'active',
    });
  });
});

describe('without an active game', () => {
  it('fails loudly instead of sending a payload Rust will reject', async () => {
    activeGame = null;
    const { result } = renderHook(() => useUpdateModThumbnail(), { wrapper });

    await expect(
      result.current.mutateAsync({ folderPath: 'C:/Mods/Ayaka', sourcePath: 'C:/pic.png' }),
    ).rejects.toThrow('No active game selected');
    expect(updateModThumbnail).not.toHaveBeenCalled();
  });

  it('blocks thumbnail deletion without an active game', async () => {
    activeGame = null;
    const { result } = renderHook(() => useDeleteModThumbnail(), { wrapper });

    await expect(result.current.mutateAsync('C:/Mods/Ayaka')).rejects.toThrow(
      'No active game selected',
    );
    expect(deleteModThumbnail).not.toHaveBeenCalled();
  });
});
