import { useInfiniteQuery, useQueryClient, type InfiniteData } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';
import type { WorkspaceExplorerPage, WorkspaceExplorerQuery } from '@/entities/workspace';
import type { AppError } from '@/shared/api/tauri/bindings.gen';
import { commands } from '@/shared/api/tauri/bindings';
import { isExplorerSnapshotExpired } from '@/shared/lib/appError';
import { workspaceKeys } from './useWorkspaceViewModel';

const EXPLORER_PAGE_SIZE = 100;

interface UseWorkspaceExplorerPagesOptions {
  enabled?: boolean;
  pageSize?: number;
}

export function useWorkspaceExplorerPages(
  query: WorkspaceExplorerQuery | null,
  options?: UseWorkspaceExplorerPagesOptions,
) {
  const pageSize = options?.pageSize ?? EXPLORER_PAGE_SIZE;
  const queryClient = useQueryClient();
  const queryKey = useMemo(() => workspaceKeys.explorerPages(query), [query]);
  const result = useInfiniteQuery<
    WorkspaceExplorerPage,
    AppError | Error,
    InfiniteData<WorkspaceExplorerPage, string | null>,
    ReturnType<typeof workspaceKeys.explorerPages>,
    string | null
  >({
    queryKey,
    queryFn: ({ pageParam }) => {
      if (!query) {
        throw new Error('Workspace explorer query is unavailable');
      }
      return commands.getWorkspaceExplorerPage({
        query,
        cursor: pageParam,
        page_size: pageSize,
      });
    },
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
    enabled: query !== null && (options?.enabled ?? true),
    staleTime: 0,
    refetchOnWindowFocus: false,
  });
  const { fetchNextPage: fetchNextQueryPage } = result;
  const fetchNextPage = useCallback(async () => {
    try {
      const nextPageResult = await fetchNextQueryPage();
      if (!isExplorerSnapshotExpired(nextPageResult.error)) {
        return;
      }
    } catch (error: unknown) {
      if (!isExplorerSnapshotExpired(error)) {
        throw error;
      }
    }

    await queryClient.resetQueries({ queryKey, exact: true });
  }, [fetchNextQueryPage, queryClient, queryKey]);
  const items = useMemo(
    () => result.data?.pages.flatMap((page) => page.items) ?? [],
    [result.data?.pages],
  );
  const firstPage = result.data?.pages[0];

  return {
    ...result,
    fetchNextPage,
    items,
    totalMatching: firstPage?.total_matching ?? 0,
    queryFingerprint: firstPage?.query_fingerprint ?? null,
    listingRevision: firstPage?.listing_revision ?? null,
  };
}
