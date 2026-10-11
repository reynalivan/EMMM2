import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import { useSavePreviewImage } from './usePreviewData';
import { usePreviewActions } from './usePreviewActions';

vi.unmock('@tanstack/react-query');

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('react-i18next', async (importOriginal) => ({
  ...(await importOriginal<typeof import('react-i18next')>()),
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock('@/shared/ui/toast', () => ({
  toast: { warning: vi.fn(), error: vi.fn(), success: vi.fn() },
}));
vi.mock('@/features/workspace-runtime', () => ({ publishWorkspaceIntent: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

function createWrapper(queryClient: QueryClient) {
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

describe('usePreviewActions async save ownership', () => {
  it('saves imported files to the folder and game captured when the picker opened', async () => {
    vi.mocked(invoke).mockResolvedValue('E:/Mods/Original/preview_skin.png');
    let finishRead!: (buffer: ArrayBuffer) => void;
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { result, rerender } = renderHook(
      ({ activeGameId, activePath, objectName }) => {
        const savePreviewImage = useSavePreviewImage();
        return usePreviewActions({
          activeGameId,
          activePath,
          selectedFolder: { name: objectName } as unknown as WorkspaceExplorerNode,
          images: [],
          currentImagePath: null,
          setCurrentImageIndex: vi.fn(),
          savePreviewImage,
          removePreviewImage: { isPending: false, mutateAsync: vi.fn() },
          clearPreviewImages: { isPending: false, mutateAsync: vi.fn() },
          restorePreviewImage: { isPending: false, mutateAsync: vi.fn() },
        });
      },
      {
        initialProps: {
          activeGameId: 'game-original',
          activePath: 'E:/Mods/Original',
          objectName: 'Original skin',
        },
        wrapper: createWrapper(queryClient),
      },
    );
    const input = document.createElement('input');
    input.type = 'file';
    result.current.importInputRef(input);
    const click = vi.spyOn(input, 'click');

    act(() => result.current.triggerThumbnailImport());
    expect(click).toHaveBeenCalledOnce();
    rerender({ activeGameId: 'game-next', activePath: 'E:/Mods/Next', objectName: 'Next skin' });
    const file = {
      arrayBuffer: () =>
        new Promise<ArrayBuffer>((resolve) => {
          finishRead = resolve;
        }),
    } as unknown as File;
    const importPromise = result.current.handleImportInputChange({
      target: { files: [file] },
      currentTarget: { files: [file], value: 'selected.png' },
    } as unknown as React.ChangeEvent<HTMLInputElement>);

    const bytes = new Uint8Array([7, 8, 9]);
    await act(async () => {
      finishRead(bytes.buffer);
      await importPromise;
    });
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('save_mod_preview_image', {
        gameId: 'game-original',
        folderPath: 'E:/Mods/Original',
        objectName: 'Original skin',
        imageData: [7, 8, 9],
      }),
    );
  });

  it('clears the pending target when the native picker is canceled', () => {
    vi.mocked(invoke).mockClear();
    const queryClient = new QueryClient();
    const { result } = renderHook(
      () => {
        const savePreviewImage = useSavePreviewImage();
        return usePreviewActions({
          activeGameId: 'game-original',
          activePath: 'E:/Mods/Original',
          selectedFolder: null,
          images: [],
          currentImagePath: null,
          setCurrentImageIndex: vi.fn(),
          savePreviewImage,
          removePreviewImage: { isPending: false, mutateAsync: vi.fn() },
          clearPreviewImages: { isPending: false, mutateAsync: vi.fn() },
          restorePreviewImage: { isPending: false, mutateAsync: vi.fn() },
        });
      },
      { wrapper: createWrapper(queryClient) },
    );
    const input = document.createElement('input');
    result.current.importInputRef(input);

    act(() => result.current.triggerThumbnailImport());
    input.dispatchEvent(new Event('cancel'));
    const file = { arrayBuffer: vi.fn() } as unknown as File;
    void result.current.handleImportInputChange({
      target: { files: [file] },
      currentTarget: { files: [file], value: 'selected.png' },
    } as unknown as React.ChangeEvent<HTMLInputElement>);

    expect(file.arrayBuffer).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalled();
  });

  it('saves clipboard images to the game and folder captured before reading', async () => {
    vi.mocked(invoke).mockResolvedValue('E:/Mods/Original/preview_mod.png');
    let finishRead!: (items: ClipboardItems) => void;
    const read = vi.fn(
      () =>
        new Promise<ClipboardItems>((resolve) => {
          finishRead = resolve;
        }),
    );
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { read } });
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { result, rerender } = renderHook(
      ({ activeGameId, activePath }) => {
        const savePreviewImage = useSavePreviewImage();
        return usePreviewActions({
          activeGameId,
          activePath,
          selectedFolder: null,
          images: [],
          currentImagePath: null,
          setCurrentImageIndex: vi.fn(),
          savePreviewImage,
          removePreviewImage: { isPending: false, mutateAsync: vi.fn() },
          clearPreviewImages: { isPending: false, mutateAsync: vi.fn() },
          restorePreviewImage: { isPending: false, mutateAsync: vi.fn() },
        });
      },
      {
        initialProps: { activeGameId: 'game-original', activePath: 'E:/Mods/Original' },
        wrapper: createWrapper(queryClient),
      },
    );

    let paste!: Promise<void>;
    act(() => {
      paste = result.current.pasteThumbnailFromClipboard();
    });
    rerender({ activeGameId: 'game-next', activePath: 'E:/Mods/Next' });
    const bytes = new Uint8Array([4, 5, 6]);
    finishRead([
      {
        presentationStyle: 'inline',
        types: ['image/png'],
        getType: async () => ({ arrayBuffer: async () => bytes.buffer }) as unknown as Blob,
      },
    ]);
    await act(async () => paste);

    expect(invoke).toHaveBeenCalledWith('save_mod_preview_image', {
      gameId: 'game-original',
      folderPath: 'E:/Mods/Original',
      objectName: 'mod',
      imageData: [4, 5, 6],
    });
  });
});
