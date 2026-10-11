import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ModFolder } from '@/entities/game-object';
import { invoke } from '@tauri-apps/api/core';
import { useEditObjectForm } from './useEditObjectForm';

const mocks = vi.hoisted(() => ({
  activeGameId: 'game-original',
  rename: vi.fn(),
  category: vi.fn(),
  thumbnailUpdate: vi.fn(),
  thumbnailDelete: vi.fn(),
  info: vi.fn(),
  safety: vi.fn(),
  updateObject: vi.fn(),
}));

vi.unmock('@tanstack/react-query');
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: { id: mocks.activeGameId } }),
}));
vi.mock('@/features/workspace-runtime', () => ({
  useUpdateObject: () => ({ mutateAsync: mocks.updateObject, isPending: false }),
}));
vi.mock('@/features/mod-runtime', () => ({
  useRenameMod: () => ({ mutateAsync: mocks.rename, isPending: false }),
  useUpdateModCategory: () => ({ mutateAsync: mocks.category, isPending: false }),
  useUpdateModThumbnail: () => ({ mutateAsync: mocks.thumbnailUpdate, isPending: false }),
  useDeleteModThumbnail: () => ({ mutateAsync: mocks.thumbnailDelete, isPending: false }),
  useUpdateModInfo: () => ({ mutateAsync: mocks.info, isPending: false }),
  useToggleModSafe: () => ({ mutateAsync: mocks.safety, isPending: false }),
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function createWrapper(client: QueryClient) {
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client }, children);
}

describe('useEditObjectForm game ownership', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.activeGameId = 'game-original';
    vi.mocked(invoke).mockResolvedValue({ is_auto_sync: false, metadata: {} });
    mocks.rename.mockResolvedValue({ new_path: 'E:/Mods/Renamed' });
    mocks.category.mockResolvedValue(undefined);
    mocks.thumbnailDelete.mockResolvedValue(undefined);
    mocks.info.mockResolvedValue(undefined);
  });

  it('keeps delete-thumbnail and info mutations on the game captured before async steps', async () => {
    const category = deferred<void>();
    mocks.category.mockReturnValue(category.promise);
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const folder = {
      path: 'E:/Mods/Old',
      name: 'Old',
      is_safe: true,
    } as unknown as ModFolder;
    const onClose = vi.fn();
    const { result, rerender } = renderHook(
      () => useEditObjectForm(true, folder, onClose, null, 'delete'),
      { wrapper: createWrapper(client) },
    );

    await waitFor(() => expect(result.current.isLoadingDetails).toBe(false));
    act(() => {
      result.current.form.setValue('name', 'Renamed');
      result.current.form.setValue('object_type', 'character');
    });

    let submission!: Promise<void>;
    act(() => {
      submission = result.current.handleSubmit();
    });
    await waitFor(() => expect(mocks.category).toHaveBeenCalled());
    mocks.activeGameId = 'game-next';
    rerender();
    await act(async () => {
      category.resolve();
      await submission;
    });

    expect(mocks.info).toHaveBeenCalledWith(
      expect.objectContaining({
        gameId: 'game-original',
        folderPath: 'E:/Mods/Renamed',
      }),
    );
    expect(mocks.thumbnailDelete).toHaveBeenCalledWith({
      gameId: 'game-original',
      folderPath: 'E:/Mods/Renamed',
    });
  });
});
