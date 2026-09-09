import { useMemo, useRef } from 'react';
import { useResponsive } from '../../../shared/lib/hooks/useResponsive';
import { sortFolders } from './folderCache';
import { useWorkspaceViewModel } from '@/features/workspace-runtime';
import { useFolderGridLayout } from './useFolderGridLayout';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import { useAppStore } from '@/app/store';
import { filterFoldersBySafety } from './safetyFilter';

interface UseFolderGridRuntimeOptions {
  viewMode: 'grid' | 'list';
  currentPath: string[];
  explorerSubPath: string | undefined;
  explorerScrollOffset: number;
  setExplorerScrollOffset: (offset: number) => void;
  sortField: 'name' | 'modified_at' | 'size_bytes';
  sortOrder: 'asc' | 'desc';
  explorerSearchQuery: string;
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
  const parentRef = useRef<HTMLDivElement>(null);
  const { data: workspace, isLoading, isError, error, isPlaceholderData } = useWorkspaceViewModel();
  const previousExplorerSubPath = getPreviousExplorerSubPath(explorerSubPath);
  const previousFolderSelection = useMemo(
    () => ({ explorerSubPath: previousExplorerSubPath, selectedModPath: null }),
    [previousExplorerSubPath],
  );
  const { data: previousWorkspace } = useWorkspaceViewModel({
    selectionOverrides: previousFolderSelection,
    enabled: currentPath.length > 1,
  });

  const rawResponse = workspace?.explorer;
  const rawFolders = useMemo(
    () => rawResponse?.children || ([] as WorkspaceExplorerNode[]),
    [rawResponse?.children],
  );
  const previousFolders = useMemo(
    () => previousWorkspace?.explorer.children || ([] as WorkspaceExplorerNode[]),
    [previousWorkspace?.explorer.children],
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
    previousFolders,
    sortedFolders,
    isLoading,
    isError,
    error,
    isPlaceholderData,
    ...layout,
  };
}
