import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { detailsKeys, thumbnailKeys } from '@/entities/mod';

vi.unmock('@tanstack/react-query');

vi.mock('@/entities/game', () => ({ useActiveGame: () => ({ activeGame: { id: 'game-1' } }) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import { usePasteThumbnail, useUpdateModThumbnail } from './useFolderMutations';

function createWrapper(queryClient: QueryClient) {
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

describe('thumbnail folder mutations', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it('imports with its game ID and invalidates thumbnail and gallery queries', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const folderPath = 'E:/Mods/ModA';
    const thumbnailKey = thumbnailKeys.folder(folderPath, 'game-1');
    const previewImagesKey = detailsKeys.previewImages(folderPath);
    queryClient.setQueryData(thumbnailKey, 'old-thumbnail.webp');
    queryClient.setQueryData(previewImagesKey, []);
    const { result } = renderHook(() => useUpdateModThumbnail(), {
      wrapper: createWrapper(queryClient),
    });

    await act(async () => {
      await result.current.mutateAsync({
        gameId: 'game-1',
        folderPath,
        sourcePath: 'selected.png',
      });
    });

    await waitFor(() => {
      expect(queryClient.getQueryState(thumbnailKey)?.isInvalidated).toBe(true);
      expect(queryClient.getQueryState(previewImagesKey)?.isInvalidated).toBe(true);
    });
    expect(invoke).toHaveBeenCalledWith('update_mod_thumbnail', {
      gameId: 'game-1',
      folderPath,
      sourcePath: 'selected.png',
    });
  });

  it('pastes with its game ID and invalidates thumbnail and gallery queries', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const folderPath = 'E:/Mods/ModA';
    const thumbnailKey = thumbnailKeys.folder(folderPath, 'game-1');
    const previewImagesKey = detailsKeys.previewImages(folderPath);
    queryClient.setQueryData(thumbnailKey, 'old-thumbnail.webp');
    queryClient.setQueryData(previewImagesKey, []);
    const { result } = renderHook(() => usePasteThumbnail(), {
      wrapper: createWrapper(queryClient),
    });

    await act(async () => {
      await result.current.mutateAsync({ gameId: 'game-1', folderPath, imageData: [1, 2, 3] });
    });

    await waitFor(() => {
      expect(queryClient.getQueryState(thumbnailKey)?.isInvalidated).toBe(true);
      expect(queryClient.getQueryState(previewImagesKey)?.isInvalidated).toBe(true);
    });
    expect(invoke).toHaveBeenCalledWith('paste_thumbnail', {
      gameId: 'game-1',
      folderPath,
      imageData: [1, 2, 3],
    });
  });
});
