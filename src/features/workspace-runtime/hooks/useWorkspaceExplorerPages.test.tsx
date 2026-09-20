import { createElement, type ReactNode } from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { WorkspaceExplorerQuery } from '@/entities/workspace';
import { useWorkspaceExplorerPages } from './useWorkspaceExplorerPages';

vi.unmock('@tanstack/react-query');

const { getWorkspaceExplorerPage } = vi.hoisted(() => ({
  getWorkspaceExplorerPage: vi.fn(),
}));

vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: { getWorkspaceExplorerPage },
}));

const explorerQuery: WorkspaceExplorerQuery = {
  game_id: 'game-1',
  explorer_sub_path: 'Alice',
  search_query: null,
  sort_field: 'name',
  sort_order: 'asc',
  safety_filter: 'all',
};

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return function Wrapper({ children }: { children: ReactNode }) {
    return createElement(QueryClientProvider, { client: queryClient }, children);
  };
}

describe('useWorkspaceExplorerPages', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('recovers an expired next-page snapshot by refetching from the first page', async () => {
    getWorkspaceExplorerPage
      .mockResolvedValueOnce({
        items: [],
        next_cursor: 'cursor-1',
        total_matching: 2,
        query_fingerprint: 'fingerprint-1',
        listing_revision: 'revision-1',
      })
      .mockRejectedValueOnce({ type: 'ExplorerSnapshotExpired' })
      .mockResolvedValueOnce({
        items: [],
        next_cursor: null,
        total_matching: 1,
        query_fingerprint: 'fingerprint-2',
        listing_revision: 'revision-2',
      });
    const { result } = renderHook(() => useWorkspaceExplorerPages(explorerQuery), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.listingRevision).toBe('revision-1'));

    await act(async () => {
      await result.current.fetchNextPage();
    });

    await waitFor(() => {
      expect(result.current.listingRevision).toBe('revision-2');
      expect(result.current.isError).toBe(false);
      expect(result.current.error).toBeNull();
    });
    expect(getWorkspaceExplorerPage).toHaveBeenNthCalledWith(1, {
      query: explorerQuery,
      cursor: null,
      page_size: 100,
    });
    expect(getWorkspaceExplorerPage).toHaveBeenNthCalledWith(2, {
      query: explorerQuery,
      cursor: 'cursor-1',
      page_size: 100,
    });
    expect(getWorkspaceExplorerPage).toHaveBeenNthCalledWith(3, {
      query: explorerQuery,
      cursor: null,
      page_size: 100,
    });
  });
});
