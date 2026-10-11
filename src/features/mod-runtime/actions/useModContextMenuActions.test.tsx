import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { WorkspaceExplorerNode } from '@/entities/workspace';

vi.unmock('@tanstack/react-query');

const mocks = vi.hoisted(() => ({
  activeGameId: 'game-original',
  openDialog: vi.fn(),
}));

vi.mock('react-i18next', async (importOriginal) => ({
  ...(await importOriginal<typeof import('react-i18next')>()),
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: { id: mocks.activeGameId } }),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: (...args: unknown[]) => mocks.openDialog(...args),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import { useModContextMenuActions } from './useModContextMenuActions';

function createWrapper(queryClient: QueryClient) {
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

const folder = (path: string) => ({ path, name: 'Example' }) as unknown as WorkspaceExplorerNode;

describe('useModContextMenuActions async ownership', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.activeGameId = 'game-original';
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it('keeps the folder and game from before clipboard reading began', async () => {
    let finishRead!: (items: ClipboardItems) => void;
    const read = vi.fn(
      () =>
        new Promise<ClipboardItems>((resolve) => {
          finishRead = resolve;
        }),
    );
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { read } });
    const queryClient = new QueryClient();
    const { result, rerender } = renderHook(({ path }) => useModContextMenuActions(folder(path)), {
      initialProps: { path: 'E:/Mods/Original' },
      wrapper: createWrapper(queryClient),
    });

    let paste!: Promise<void>;
    act(() => {
      paste = result.current.pasteThumbnailFromClipboard();
    });
    mocks.activeGameId = 'game-next';
    rerender({ path: 'E:/Mods/Next' });
    const bytes = new Uint8Array([1, 2, 3]);
    finishRead([
      {
        presentationStyle: 'inline',
        types: ['image/png'],
        getType: async () => ({ arrayBuffer: async () => bytes.buffer }) as unknown as Blob,
      },
    ]);
    await act(async () => paste);

    expect(invoke).toHaveBeenCalledWith('paste_thumbnail', {
      gameId: 'game-original',
      folderPath: 'E:/Mods/Original',
      imageData: [1, 2, 3],
    });
  });

  it('keeps the folder and game from before the picker opened', async () => {
    let finishPicker!: (path: string | null) => void;
    mocks.openDialog.mockReturnValueOnce(
      new Promise((resolve) => {
        finishPicker = resolve;
      }),
    );
    const queryClient = new QueryClient();
    const { result, rerender } = renderHook(({ path }) => useModContextMenuActions(folder(path)), {
      initialProps: { path: 'E:/Mods/Original' },
      wrapper: createWrapper(queryClient),
    });

    let importing!: Promise<void>;
    act(() => {
      importing = result.current.importThumbnail();
    });
    mocks.activeGameId = 'game-next';
    rerender({ path: 'E:/Mods/Next' });
    finishPicker('E:/Images/selected.png');
    await act(async () => importing);

    expect(invoke).toHaveBeenCalledWith('update_mod_thumbnail', {
      gameId: 'game-original',
      folderPath: 'E:/Mods/Original',
      sourcePath: 'E:/Images/selected.png',
    });
  });
});
