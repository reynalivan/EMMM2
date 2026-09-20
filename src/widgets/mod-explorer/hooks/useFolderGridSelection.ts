import { useCallback, useEffect, useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { ModFolder } from '@/entities/game-object';
import type { WorkspaceExplorerSelectionModel } from '@/entities/workspace';
import {
  isWorkspaceExplorerPathSelected,
  workspaceExplorerSelectionCount,
} from '@/entities/workspace';
import { workspaceKeys } from '@/features/workspace-runtime';
import { publishQueryInvalidations } from '@/shared/lib/queryRefresh';
import { useFolderNavigation } from './useFolderNavigation';
import { useRangeSelection } from '../../../shared/lib/hooks/useRangeSelection';
import { normalizeWorkspacePath } from '@/features/workspace-runtime';

const getFolderPath = (folder: ModFolder) => folder.path;

interface UseFolderGridSelectionOptions {
  sortedFolders: ModFolder[];
  selectedModPath?: string | null;
  selection: WorkspaceExplorerSelectionModel;
  addSelectionPaths: (paths: Iterable<string>) => void;
  toggleSelectionPath: (path: string, multi: boolean) => void;
  clearSelection: () => void;
  selectAllMatching: () => void;
  currentPath: string[];
  isGridView: boolean;
  columnCount: number;
  isMobile: boolean;
  scrollToIndex: (index: number, options: { align: 'auto' | 'start' | 'center' | 'end' }) => void;
  selectMod: (path: string | null, mobilePane?: 'sidebar' | 'grid' | 'details') => void;
  handleNavigate: (folderName: string) => void;
  handleBreadcrumbClick: (index: number) => void;
  handleDeleteRequest: (folder: ModFolder) => void;
  handleRenameRequest: (folder: ModFolder) => void;
}

export function useFolderGridSelection({
  sortedFolders,
  selectedModPath = null,
  selection,
  addSelectionPaths,
  toggleSelectionPath,
  clearSelection,
  selectAllMatching,
  currentPath,
  isGridView,
  columnCount,
  isMobile,
  scrollToIndex,
  selectMod,
  handleNavigate,
  handleBreadcrumbClick,
  handleDeleteRequest,
  handleRenameRequest,
}: UseFolderGridSelectionOptions) {
  const queryClient = useQueryClient();
  const { anchorId, setAnchorId, getRange } = useRangeSelection(sortedFolders, getFolderPath);
  const visiblePathKeys = useMemo(
    () => new Set(sortedFolders.map((folder) => normalizeWorkspacePath(folder.path))),
    [sortedFolders],
  );

  useEffect(() => {
    if (!anchorId || visiblePathKeys.has(normalizeWorkspacePath(anchorId))) {
      return;
    }

    const nextSelectedPath = sortedFolders.find((folder) =>
      isWorkspaceExplorerPathSelected(selection, folder.path),
    )?.path;
    if (nextSelectedPath) {
      setAnchorId(nextSelectedPath);
    }
  }, [anchorId, selection, setAnchorId, sortedFolders, visiblePathKeys]);

  const handleActivateItem = useCallback(
    (path: string) => {
      if (
        selectedModPath &&
        normalizeWorkspacePath(selectedModPath) === normalizeWorkspacePath(path)
      ) {
        void publishQueryInvalidations(queryClient, [workspaceKeys.previews], 'active');
      }
      clearSelection();
      selectMod(path, isMobile ? 'details' : undefined);
      setAnchorId(path);
    },
    [clearSelection, isMobile, queryClient, selectMod, selectedModPath, setAnchorId],
  );

  const handleToggleSelection = useCallback(
    (path: string, multi: boolean, isShift?: boolean) => {
      if (isShift) {
        const range = getRange(path);
        if (range) {
          addSelectionPaths(range);
          selectMod(path, isMobile ? 'details' : undefined);
          return;
        }
      }

      let nextSelectedModPath: string | null = path;
      let nextSelectionSize = 1;
      if (selection.mode === 'explicit') {
        const nextSelection = new Set(multi ? selection.paths : []);
        if (nextSelection.has(path)) {
          nextSelection.delete(path);
        } else {
          nextSelection.add(path);
        }
        nextSelectionSize = nextSelection.size;
        const nextPaths = Array.from(nextSelection);
        nextSelectedModPath = nextPaths[nextPaths.length - 1] ?? null;
      } else if (multi) {
        const wasSelected = isWorkspaceExplorerPathSelected(selection, path);
        nextSelectionSize = workspaceExplorerSelectionCount(selection) + (wasSelected ? -1 : 1);
        if (wasSelected) {
          nextSelectedModPath =
            sortedFolders.find(
              (folder) =>
                folder.path !== path && isWorkspaceExplorerPathSelected(selection, folder.path),
            )?.path ?? null;
        }
      }

      toggleSelectionPath(path, multi);
      selectMod(nextSelectedModPath, isMobile && nextSelectionSize === 1 ? 'details' : undefined);
      setAnchorId(path);
    },
    [
      addSelectionPaths,
      getRange,
      isMobile,
      selectMod,
      selection,
      setAnchorId,
      sortedFolders,
      toggleSelectionPath,
    ],
  );

  const { focusedId, handleKeyDown } = useFolderNavigation({
    items: sortedFolders,
    gridColumns: isGridView ? columnCount : 1,
    getId: (item: ModFolder) => item.path,
    onNavigate: (item: ModFolder) => handleNavigate(item.folder_name),
    onSelectionChange: (item: ModFolder, multi: boolean, isShift?: boolean) =>
      handleToggleSelection(item.path, multi, isShift),
    onSelectAll: selectAllMatching,
    onDelete: (items: ModFolder[]) => {
      if (items.length > 0) {
        handleDeleteRequest(items[0]);
      }
    },
    onRename: (item: ModFolder) => handleRenameRequest(item),
    onGoUp: () => {
      if (currentPath.length > 0) {
        handleBreadcrumbClick(currentPath.length - 2);
      }
    },
    onFocusChange: (nextId: string | null) => {
      const nextIndex = sortedFolders.findIndex((folder) => folder.path === nextId);
      if (nextIndex === -1) {
        return;
      }

      const rowIndex = isGridView ? Math.floor(nextIndex / columnCount) : nextIndex;
      scrollToIndex(rowIndex, { align: 'auto' });
    },
  });

  return {
    focusedId,
    handleKeyDown,
    handleToggleSelection,
    handleActivateItem,
  };
}
