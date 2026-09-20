import { useEffect, useMemo, useRef, useState } from 'react';
import { useResponsive } from '../../../shared/lib/hooks/useResponsive';
import { useWorkspaceExplorerPages, useWorkspaceViewModel } from '@/features/workspace-runtime';
import { useFolderGridLayout } from './useFolderGridLayout';
import type { WorkspaceExplorerQuery } from '@/entities/workspace';
import { useAppStore } from '@/app/store';

interface UseFolderGridRuntimeOptions {
  activeGameId: string | undefined;
  viewMode: 'grid' | 'list';
  currentPath: string[];
  explorerSubPath: string | undefined;
  explorerScrollOffset: number;
  setExplorerScrollOffset: (offset: number) => void;
  sortField: 'name' | 'modified_at' | 'size_bytes';
  sortOrder: 'asc' | 'desc';
  explorerSearchQuery: string;
}

export const EXPLORER_SEARCH_DEBOUNCE_MS = 200;

export function useDebouncedExplorerSearchQuery(searchQuery: string): string {
  const [debouncedSearchQuery, setDebouncedSearchQuery] = useState(searchQuery);

  useEffect(() => {
    const timeoutId = window.setTimeout(
      () => setDebouncedSearchQuery(searchQuery),
      EXPLORER_SEARCH_DEBOUNCE_MS,
    );
    return () => window.clearTimeout(timeoutId);
  }, [searchQuery]);

  return debouncedSearchQuery;
}

export function getPreviousExplorerSubPath(
  explorerSubPath: string | undefined,
): string | undefined {
  if (!explorerSubPath) {
    return undefined;
  }

  const separatorIndex = explorerSubPath.lastIndexOf('/');
  return separatorIndex > 0 ? explorerSubPath.slice(0, separatorIndex) : undefined;
}

export function useFolderGridRuntime({
  activeGameId,
  viewMode,
  currentPath,
  explorerSubPath,
  explorerScrollOffset,
  setExplorerScrollOffset,
  sortField,
  sortOrder,
  explorerSearchQuery,
}: UseFolderGridRuntimeOptions) {
  const { isMobile } = useResponsive();
  const safetyFilter = useAppStore((state) => state.safetyFilter);
  const debouncedExplorerSearchQuery = useDebouncedExplorerSearchQuery(explorerSearchQuery);
  const isExplorerSearchPending = explorerSearchQuery !== debouncedExplorerSearchQuery;
  const parentRef = useRef<HTMLDivElement>(null);
  const {
    data: workspace,
    isLoading: isWorkspaceLoading,
    isFetching: isWorkspaceFetching,
    isError: isWorkspaceError,
    error: workspaceError,
    isPlaceholderData,
  } = useWorkspaceViewModel();
  const explorerQuery = useMemo<WorkspaceExplorerQuery | null>(
    () =>
      activeGameId
        ? {
            game_id: activeGameId,
            explorer_sub_path: explorerSubPath ?? null,
            search_query: debouncedExplorerSearchQuery.trim() || null,
            sort_field: sortField,
            sort_order: sortOrder,
            safety_filter: safetyFilter,
          }
        : null,
    [
      activeGameId,
      debouncedExplorerSearchQuery,
      explorerSubPath,
      safetyFilter,
      sortField,
      sortOrder,
    ],
  );
  const sourceAvailable = workspace?.runtime?.source_state.status !== 'unavailable';
  const explorerPages = useWorkspaceExplorerPages(explorerQuery, {
    enabled: Boolean(workspace && sourceAvailable),
  });
  const previousExplorerSubPath = getPreviousExplorerSubPath(explorerSubPath);
  const previousExplorerQuery = useMemo<WorkspaceExplorerQuery | null>(
    () =>
      activeGameId
        ? {
            game_id: activeGameId,
            explorer_sub_path: previousExplorerSubPath ?? null,
            search_query: null,
            sort_field: 'name',
            sort_order: 'asc',
            safety_filter: 'all',
          }
        : null,
    [activeGameId, previousExplorerSubPath],
  );
  const previousExplorerPages = useWorkspaceExplorerPages(previousExplorerQuery, {
    enabled: Boolean(workspace && sourceAvailable && currentPath.length > 1),
  });

  const rawResponse = workspace?.explorer;
  const rawFolders = explorerPages.items;
  const previousFolders = previousExplorerPages.items;
  const sortedFolders = rawFolders;
  const {
    fetchNextPage: fetchNextExplorerPage,
    hasNextPage: hasNextExplorerPage,
    isFetchingNextPage: isFetchingNextExplorerPage,
  } = explorerPages;
  const isGridView = viewMode === 'grid' && !isMobile;
  const layout = useFolderGridLayout({
    parentRef,
    explorerSubPath,
    explorerScrollOffset,
    setExplorerScrollOffset,
    isGridView,
    itemCount: sortedFolders.length,
  });
  const lastVirtualIndex = layout.virtualItems[layout.virtualItems.length - 1]?.index;

  useEffect(() => {
    if (
      lastVirtualIndex === undefined ||
      !hasNextExplorerPage ||
      isFetchingNextExplorerPage ||
      sortedFolders.length === 0
    ) {
      return;
    }

    const lastVisibleItemIndex = isGridView
      ? (lastVirtualIndex + 1) * layout.columnCount - 1
      : lastVirtualIndex;
    const preloadThreshold = Math.max(10, layout.columnCount * 2);
    if (lastVisibleItemIndex >= sortedFolders.length - preloadThreshold) {
      void fetchNextExplorerPage();
    }
  }, [
    fetchNextExplorerPage,
    hasNextExplorerPage,
    isFetchingNextExplorerPage,
    isGridView,
    lastVirtualIndex,
    layout.columnCount,
    sortedFolders.length,
  ]);

  const explorerLoading = sourceAvailable && explorerPages.isLoading;

  return {
    parentRef,
    isMobile,
    isGridView,
    workspace,
    rawResponse,
    rawFolders,
    previousFolders,
    hasMorePreviousFolders: previousExplorerPages.hasNextPage,
    isLoadingMorePreviousFolders: previousExplorerPages.isFetchingNextPage,
    loadMorePreviousFolders: previousExplorerPages.fetchNextPage,
    sortedFolders,
    explorerQuery,
    isExplorerSearchPending,
    totalMatching: explorerPages.totalMatching,
    listingRevision: explorerPages.listingRevision,
    isLoading: isWorkspaceLoading || explorerLoading,
    isRefreshing: (isWorkspaceFetching && !isWorkspaceLoading) || explorerPages.isFetchingNextPage,
    isError: isWorkspaceError || explorerPages.isError,
    error: workspaceError ?? explorerPages.error,
    isPlaceholderData,
    ...layout,
  };
}
