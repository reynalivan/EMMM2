import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/app/store';
import { useBulkDelete, useBulkToggle } from './useBulkModMutations';

const mocks = vi.hoisted(() => ({
  bulkToggleMods: vi.fn(),
  bulkDeleteMods: vi.fn(),
  applyRuntimeEffects: vi.fn(),
  cancelRuntimeDescriptorQueries: vi.fn(),
  publishRuntimeDescriptor: vi.fn(),
  toastSuccess: vi.fn(),
}));

vi.mock('@tanstack/react-query', async () =>
  vi.importActual<typeof import('@tanstack/react-query')>('@tanstack/react-query'),
);

vi.mock('../../../shared/api/tauri/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    bulkToggleMods: (...args: unknown[]) => mocks.bulkToggleMods(...args),
    bulkDeleteMods: (...args: unknown[]) => mocks.bulkDeleteMods(...args),
  },
}));

vi.mock('@/features/workspace-runtime/@x/mod-runtime', () => ({
  applyRuntimeEffects: (...args: unknown[]) => mocks.applyRuntimeEffects(...args),
  buildQueryRemovalDescriptor: vi.fn(() => ({})),
  buildRuntimeMutationDescriptor: vi.fn(() => ({})),
  buildWorkspacePathRewritesDescriptor: vi.fn(() => ({})),
  collectionReferenceImpactRefreshEvents: vi.fn(() => []),
  notifyCollectionReferenceImpact: vi.fn(),
  openFileInUseRetryDialog: vi.fn(() => false),
}));

vi.mock('@/shared/lib/queryRefresh', () => ({
  cancelRuntimeDescriptorQueries: (...args: unknown[]) =>
    mocks.cancelRuntimeDescriptorQueries(...args),
  publishRuntimeDescriptor: (...args: unknown[]) => mocks.publishRuntimeDescriptor(...args),
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    success: (...args: unknown[]) => mocks.toastSuccess(...args),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

vi.mock('../../../shared/lib/committedMutationWarning', () => ({
  notifyCommittedMutationSyncWarning: vi.fn(),
}));

const bulkResult = {
  success: ['E:/Mods/Blue'],
  failures: [],
  cancelled: false,
  processed_count: 1,
  unprocessed_count: 0,
  collection_impact: { changed_collection_ids: [], stale_collection_ids: [] },
  path_rewrites: [],
  sync_warning: null,
  runtime_sync_generation: 4,
};

function wrapper({ children }: { children: React.ReactNode }) {
  return React.createElement(
    QueryClientProvider,
    { client: new QueryClient({ defaultOptions: { mutations: { retry: false } } }) },
    children,
  );
}

describe('bulk mod mutations', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({ activeGameId: 'game-1' });
    mocks.cancelRuntimeDescriptorQueries.mockResolvedValue(undefined);
    mocks.publishRuntimeDescriptor.mockResolvedValue(undefined);
  });

  it('assigns a unique operation ID to the backend batch', async () => {
    mocks.bulkToggleMods.mockResolvedValue(bulkResult);
    const { result } = renderHook(() => useBulkToggle(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({
        gameId: 'game-1',
        paths: ['E:/Mods/DISABLED Blue'],
        enable: true,
      });
    });

    expect(mocks.bulkToggleMods).toHaveBeenCalledWith(
      'game-1',
      ['E:/Mods/DISABLED Blue'],
      true,
      expect.stringMatching(/^toggle-[0-9a-f-]+$/),
    );
  });

  it('does not publish a stale batch into the newly active game', async () => {
    let complete: ((value: typeof bulkResult) => void) | undefined;
    mocks.bulkToggleMods.mockReturnValue(
      new Promise<typeof bulkResult>((resolve) => {
        complete = resolve;
      }),
    );
    const { result } = renderHook(() => useBulkToggle(), { wrapper });

    let pending!: Promise<unknown>;
    act(() => {
      pending = result.current.mutateAsync({
        gameId: 'game-1',
        paths: ['E:/Mods/DISABLED Blue'],
        enable: true,
      });
    });
    useAppStore.setState({ activeGameId: 'game-2' });
    await act(async () => {
      complete?.(bulkResult);
      await pending;
    });

    expect(mocks.applyRuntimeEffects).not.toHaveBeenCalled();
    expect(mocks.publishRuntimeDescriptor).not.toHaveBeenCalled();
  });

  it('settles the mutation without waiting for background cache revalidation', async () => {
    let finishRefresh!: () => void;
    mocks.bulkToggleMods.mockResolvedValue(bulkResult);
    mocks.publishRuntimeDescriptor.mockReturnValue(
      new Promise<void>((resolve) => {
        finishRefresh = resolve;
      }),
    );
    const { result } = renderHook(() => useBulkToggle(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({
        gameId: 'game-1',
        paths: ['E:/Mods/DISABLED Blue'],
        enable: true,
      });
    });

    expect(result.current.isPending).toBe(false);
    expect(mocks.cancelRuntimeDescriptorQueries.mock.invocationCallOrder[0]).toBeLessThan(
      mocks.applyRuntimeEffects.mock.invocationCallOrder[0],
    );
    await vi.waitFor(() => expect(mocks.publishRuntimeDescriptor).toHaveBeenCalled());
    finishRefresh();
  });

  it('suppresses delete feedback when the active game changes during refresh', async () => {
    let finishRefresh!: () => void;
    mocks.bulkDeleteMods.mockResolvedValue(bulkResult);
    mocks.publishRuntimeDescriptor.mockReturnValue(
      new Promise<void>((resolve) => {
        finishRefresh = resolve;
      }),
    );
    const { result } = renderHook(() => useBulkDelete(), { wrapper });

    let pending!: Promise<unknown>;
    act(() => {
      pending = result.current.mutateAsync({
        gameId: 'game-1',
        paths: ['E:/Mods/Blue'],
      });
    });
    await vi.waitFor(() => expect(mocks.publishRuntimeDescriptor).toHaveBeenCalled());
    useAppStore.setState({ activeGameId: 'game-2' });

    await act(async () => {
      finishRefresh();
      await pending;
    });

    expect(mocks.toastSuccess).not.toHaveBeenCalled();
  });
});
