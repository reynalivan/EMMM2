import React from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createWrapper } from '../../../tests/testing/test-utils';
import { useAppStore } from '@/app/store';
import { useToastStore } from '@/shared/ui/toast';
import { detailsKeys, thumbnailKeys } from '@/entities/mod';
import {
  useModIniDocuments,
  usePreviewImages,
  useSavePreviewImage,
  useRemovePreviewImage,
  useRestorePreviewImage,
  useClearPreviewImages,
  useWriteModIni,
  useUpdateModInfoDetails,
} from './usePreviewData';

vi.unmock('@tanstack/react-query');

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

function countInvokes(command: string): number {
  return vi.mocked(invoke).mock.calls.filter(([name]) => name === command).length;
}

describe('usePreviewData hooks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({ activeGameId: 'game-1' });
    useToastStore.setState({ toasts: [] });
  });

  it('fetches all ini documents in one request', async () => {
    vi.mocked(invoke).mockResolvedValue([
      { filename: 'config.ini', document: { mode: 'Structured', raw_lines: [] } },
    ]);

    const { result } = renderHook(() => useModIniDocuments('E:/Mods/ModA'), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(invoke).toHaveBeenCalledWith('read_mod_ini_documents', {
      gameId: 'game-1',
      folderPath: 'E:/Mods/ModA',
    });
  });

  it('fetches ordered preview images including reversible visibility', async () => {
    const entries = [
      { path: 'E:/Mods/ModA/preview_custom.png', is_hidden: false },
      { path: 'E:/Mods/ModA/texture.png', is_hidden: true },
    ];
    vi.mocked(invoke).mockResolvedValue(entries);

    const { result } = renderHook(() => usePreviewImages('E:/Mods/ModA'), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual(entries);
    expect(invoke).toHaveBeenCalledWith('list_mod_preview_images_with_visibility', {
      gameId: 'game-1',
      folderPath: 'E:/Mods/ModA',
    });
  });

  it('writes ini line updates with mutation', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    const { result } = renderHook(() => useWriteModIni(), { wrapper: createWrapper });

    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
      fileName: 'config.ini',
      expectedSourceHash: 'source-1',
      lineUpdates: [{ line_idx: 1, content: '$swapvar = 1' }],
    });

    expect(invoke).toHaveBeenCalledWith('write_mod_ini', {
      gameId: 'game-1',
      folderPath: 'E:/Mods/ModA',
      fileName: 'config.ini',
      expectedSourceHash: 'source-1',
      lineUpdates: [{ line_idx: 1, content: '$swapvar = 1' }],
    });
  });

  it('invalidates active conflicts after writing an ini file', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const wrapper = ({ children }: { children: React.ReactNode }) =>
      React.createElement(QueryClientProvider, { client: queryClient }, children);
    queryClient.setQueryData(['conflicts', 'game-1'], [{ hash: 'abcdef12' }]);
    vi.mocked(invoke).mockResolvedValue(undefined);

    const { result } = renderHook(() => useWriteModIni(), { wrapper });
    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
      fileName: 'config.ini',
      expectedSourceHash: 'source-1',
      lineUpdates: [{ line_idx: 1, content: 'hash = deadbeef' }],
    });

    await waitFor(() => {
      expect(queryClient.getQueryState(['conflicts', 'game-1'])?.isInvalidated).toBe(true);
    });
  });

  it('surfaces a nonfatal reconcile warning after a committed ini write', async () => {
    vi.mocked(invoke).mockResolvedValue({
      sync_warning: { kind: 'ReconcileFailed', message: 'projection pending' },
    });

    const { result } = renderHook(() => useWriteModIni(), { wrapper: createWrapper });
    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
      fileName: 'config.ini',
      expectedSourceHash: 'source-1',
      lineUpdates: [{ line_idx: 1, content: '$swapvar = 1' }],
    });

    expect(useToastStore.getState().toasts).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          type: 'warning',
          message: 'Disk changes were applied, but runtime refresh is still pending.',
        }),
      ]),
    );
  });

  it('saves preview image with object naming mutation', async () => {
    vi.mocked(invoke).mockResolvedValue('E:/Mods/ModA/preview_keqing.png');

    const { result } = renderHook(() => useSavePreviewImage(), { wrapper: createWrapper });

    await result.current.mutateAsync({
      gameId: 'game-1',
      folderPath: 'E:/Mods/ModA',
      objectName: 'Keqing',
      imageData: [1, 2, 3],
    });

    expect(invoke).toHaveBeenCalledWith('save_mod_preview_image', {
      gameId: 'game-1',
      folderPath: 'E:/Mods/ModA',
      objectName: 'Keqing',
      imageData: [1, 2, 3],
    });
  });

  it('hides one preview image with mutation', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    const { result } = renderHook(() => useRemovePreviewImage(), { wrapper: createWrapper });

    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
      imagePath: 'E:/Mods/ModA/preview_keqing.png',
    });

    expect(invoke).toHaveBeenCalledWith('remove_mod_preview_image', {
      gameId: 'game-1',
      folderPath: 'E:/Mods/ModA',
      imagePath: 'E:/Mods/ModA/preview_keqing.png',
    });
  });

  it('restores a hidden image through the typed visibility command', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    const { result } = renderHook(() => useRestorePreviewImage(), { wrapper: createWrapper });

    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
      imagePath: 'E:/Mods/ModA/texture.png',
    });

    expect(invoke).toHaveBeenCalledWith('set_mod_preview_image_hidden', {
      gameId: 'game-1',
      folderPath: 'E:/Mods/ModA',
      imagePath: 'E:/Mods/ModA/texture.png',
      hidden: false,
    });
  });

  it('invalidates the grid thumbnail after changing preview visibility', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const wrapper = ({ children }: { children: React.ReactNode }) =>
      React.createElement(QueryClientProvider, { client: queryClient }, children);
    const thumbnailKey = thumbnailKeys.folder('E:/Mods/ModA', 'game-1');
    const previewImagesKey = detailsKeys.previewImages('E:/Mods/ModA');
    queryClient.setQueryData(thumbnailKey, 'cached.webp');
    queryClient.setQueryData(previewImagesKey, []);
    vi.mocked(invoke).mockResolvedValue(undefined);
    const { result } = renderHook(() => useRemovePreviewImage(), { wrapper });

    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
      imagePath: 'E:/Mods/ModA/preview_custom.png',
    });

    await waitFor(() => {
      expect(queryClient.getQueryState(thumbnailKey)?.isInvalidated).toBe(true);
      expect(queryClient.getQueryState(previewImagesKey)?.isInvalidated).toBe(true);
    });
  });

  it('clears all preview images with mutation', async () => {
    vi.mocked(invoke).mockResolvedValue(['E:/Mods/ModA/preview_keqing.png']);

    const { result } = renderHook(() => useClearPreviewImages(), { wrapper: createWrapper });

    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
    });

    expect(invoke).toHaveBeenCalledWith('clear_mod_preview_images', {
      gameId: 'game-1',
      folderPath: 'E:/Mods/ModA',
    });
  });

  it('preview image mutations refetch the preview image query instead of patching it', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const wrapper = ({ children }: { children: React.ReactNode }) =>
      React.createElement(QueryClientProvider, { client: queryClient }, children);

    vi.mocked(invoke).mockResolvedValue([]);
    const thumbnailKey = thumbnailKeys.folder('E:/Mods/ModA', 'game-1');
    queryClient.setQueryData(thumbnailKey, 'cached.webp');

    const { result } = renderHook(
      () => ({
        images: usePreviewImages('E:/Mods/ModA'),
        save: useSavePreviewImage(),
      }),
      { wrapper },
    );

    await waitFor(() => expect(result.current.images.isSuccess).toBe(true));
    const before = countInvokes('list_mod_preview_images_with_visibility');

    await act(async () => {
      await result.current.save.mutateAsync({
        gameId: 'game-1',
        folderPath: 'E:/Mods/ModA',
        objectName: 'Keqing',
        imageData: [1, 2, 3],
      });
    });

    await waitFor(() =>
      expect(countInvokes('list_mod_preview_images_with_visibility')).toBeGreaterThan(before),
    );
    expect(queryClient.getQueryState(thumbnailKey)?.isInvalidated).toBe(true);
  });

  it('mutation error is exposed to caller', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('Operation in progress. Please wait.'));

    const { result } = renderHook(() => useWriteModIni(), { wrapper: createWrapper });

    await expect(
      result.current.mutateAsync({
        folderPath: 'E:/Mods/ModA',
        fileName: 'config.ini',
        expectedSourceHash: 'source-1',
        lineUpdates: [{ line_idx: 1, content: '$swapvar = 1' }],
      }),
    ).rejects.toThrow('Operation in progress. Please wait.');
  });

  it('updates mod info with mutation', async () => {
    vi.mocked(invoke).mockResolvedValue({ actual_name: 'Mod A', description: 'desc' });

    const { result } = renderHook(() => useUpdateModInfoDetails(), { wrapper: createWrapper });

    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
      update: { actual_name: 'Mod A+', description: 'updated' },
    });

    expect(invoke).toHaveBeenCalledWith('update_mod_info', {
      gameId: 'game-1',
      folderPath: 'E:/Mods/ModA',
      update: { actual_name: 'Mod A+', description: 'updated' },
    });
  });
});
