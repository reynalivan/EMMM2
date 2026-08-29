import { useMemo, useRef } from 'react';
import { useResponsive } from '../../../hooks/useResponsive';
import { sortFolders } from '../../../hooks/folderCache';
import { useWorkspaceViewModel } from '../../workspace-runtime/useWorkspaceViewModel';
import { useFolderGridLayout } from './useFolderGridLayout';
import type { WorkspaceExplorerNode } from '../../../types/workspace';
import { useAppStore } from '../../../stores/useAppStore';
import { filterFoldersBySafety } from './safetyFilter';

interface UseFolderGridRuntimeOptions {
  viewMode: 'grid' | 'list';
  explorerSubPath: string | undefined;
  explorerScrollOffset: number;
  setExplorerScrollOffset: (offset: number) => void;
  sortField: 'name' | 'modified_at' | 'size_bytes';
  sortOrder: 'asc' | 'desc';
  explorerSearchQuery: string;
}

export function useFolderGridRuntime({
  viewMode,
  explorerSubPath,
  explorerScrollOffset,
  setExplorerScrollOffset,
  sortField,
  sortOrder,
  explorerSearchQuery,
}: UseFolderGridRuntimeOptions) {
  const { isMobile } = useResponsive();
  const safetyFilter = useAppStore((state) => state.safetyFilter);
  const parentRef = useRef<HTMLDivElement>(null);
  const { data: workspace, isLoading, isError, error, isPlaceholderData } = useWorkspaceViewModel();

  const rawResponse = workspace?.explorer;
  const rawFolders = useMemo(
    () => rawResponse?.children || ([] as WorkspaceExplorerNode[]),
    [rawResponse?.children],
  );
  const filteredFolders = useMemo(() => {
    const safetyFiltered = filterFoldersBySafety(rawFolders, safetyFilter);
    if (!explorerSearchQuery) {
      return safetyFiltered;
    }

    const query = explorerSearchQuery.toLowerCase();
    return safetyFiltered.filter((folder) => folder.name.toLowerCase().includes(query));
  }, [explorerSearchQuery, rawFolders, safetyFilter]);
  const sortedFolders = useMemo(
    () => sortFolders(filteredFolders, sortField, sortOrder),
    [filteredFolders, sortField, sortOrder],
  );
  const isGridView = viewMode === 'grid' && !isMobile;
  const layout = useFolderGridLayout({
    parentRef,
    explorerSubPath,
    explorerScrollOffset,
    setExplorerScrollOffset,
    isGridView,
    itemCount: sortedFolders.length,
  });

  return {
    parentRef,
    isMobile,
    isGridView,
    workspace,
    rawResponse,
    rawFolders,
    sortedFolders,
    isLoading,
    isError,
    error,
    isPlaceholderData,
    ...layout,
  };
}
