import { useCallback, useEffect, useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { ModFolder } from '@/entities/game-object';
import type { WorkspaceExplorerSelectionModel } from '@/entities/workspace';
import {
  isWorkspaceExplorerPathSelected,
  workspaceExplorerSelectionCount,
} from '@/entities/workspace';
import { workspaceKeys } from '@/features/workspace-runtime';
import type { WorkspaceExplorerSelectionEffect } from '@/features/workspace-runtime';
import { publishQueryInvalidations } from '@/shared/lib/queryRefresh';
import { useFolderNavigation } from './useFolderNavigation';
import { useRangeSelection } from '../../../shared/lib/hooks/useRangeSelection';
import { normalizeWorkspacePath } from '@/features/workspace-runtime';

const getFolderPath = (folder: ModFolder) => folder.path;

interface UseFolderGridSelectionOptions {
  sortedFolders: ModFolder[];
  selectedModPath?: string | null;
  selection: WorkspaceExplorerSelectionModel;
  createAllMatchingCommit: (
    selection: Extract<WorkspaceExplorerSelectionModel, { mode: 'all_matching' }> | null,
    validateListingRevision?: boolean,
  ) => Pick<WorkspaceExplorerSelectionEffect, 'isCurrent' | 'onApplied'>;
  selectAllMatching: () => void;
  currentPath: string[];
  isGridView: boolean;
  columnCount: number;
  isMobile: boolean;
  scrollToIndex: (index: number, options: { align: 'auto' | 'start' | 'center' | 'end' }) => void;
  selectMod: (
    path: string | null,
    mobilePane?: 'sidebar' | 'grid' | 'details',
    selectionEffect?: WorkspaceExplorerSelectionEffect,
  ) => boolean;
  handleNavigate: (folderName: string) => void;
  handleBreadcrumbClick: (index: number) => void;
  handleDeleteRequest: (folder: ModFolder) => void;
  handleRenameRequest: (folder: ModFolder) => void;
}

export function useFolderGridSelection({
  sortedFolders,
  selectedModPath = null,
  selection,
  createAllMatchingCommit,
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
  const createSelectionEffect = useCallback(
    (nextSelection: WorkspaceExplorerSelectionModel): WorkspaceExplorerSelectionEffect => {
      if (nextSelection.mode === 'all_matching') {
        return {
          gridSelection: [],
          affectedPaths: [...nextSelection.excludedPaths],
          ...createAllMatchingCommit(nextSelection),
        };
      }

      const currentSnapshotCommit = createAllMatchingCommit(null, false);
      return {
        gridSelection: [...nextSelection.paths],
        isCurrent: currentSnapshotCommit.isCurrent,
        ...(selection.mode === 'all_matching'
          ? { onApplied: currentSnapshotCommit.onApplied }
          : {}),
      };
    },
    [createAllMatchingCommit, selection.mode],
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
      const selectionEffect = createSelectionEffect({ mode: 'explicit', paths: new Set() });
      if (!selectMod(path, isMobile ? 'details' : undefined, selectionEffect)) {
        return;
      }

      if (
        selectedModPath &&
        normalizeWorkspacePath(selectedModPath) === normalizeWorkspacePath(path)
      ) {
        void publishQueryInvalidations(queryClient, [workspaceKeys.previews], 'active');
      }
      setAnchorId(path);
    },
    [createSelectionEffect, isMobile, queryClient, selectMod, selectedModPath, setAnchorId],
  );

  const handleToggleSelection = useCallback(
    (path: string, multi: boolean, isShift?: boolean) => {
      if (isShift) {
        const range = getRange(path);
        if (range) {
          let nextSelection: WorkspaceExplorerSelectionModel;
          if (selection.mode === 'all_matching') {
            const excludedPaths = new Set(selection.excludedPaths);
            for (const rangePath of range) {
              excludedPaths.delete(rangePath);
            }
            nextSelection = { ...selection, excludedPaths };
          } else {
            nextSelection = { mode: 'explicit', paths: new Set([...selection.paths, ...range]) };
          }
          if (
            !selectMod(path, isMobile ? 'details' : undefined, createSelectionEffect(nextSelection))
          ) {
            return;
          }
          return;
        }
      }

      let nextSelectedModPath: string | null = path;
      let nextSelectionSize = 1;
      let nextSelection: WorkspaceExplorerSelectionModel;
      if (selection.mode === 'explicit') {
        const nextPaths = new Set(multi ? selection.paths : []);
        if (nextPaths.has(path)) {
          nextPaths.delete(path);
        } else {
          nextPaths.add(path);
        }
        nextSelectionSize = nextPaths.size;
        const nextPathEntries = Array.from(nextPaths);
        nextSelectedModPath = nextPathEntries[nextPathEntries.length - 1] ?? null;
        nextSelection = { mode: 'explicit', paths: nextPaths };
      } else if (multi) {
        const wasSelected = isWorkspaceExplorerPathSelected(selection, path);
        nextSelectionSize = workspaceExplorerSelectionCount(selection) + (wasSelected ? -1 : 1);
        const excludedPaths = new Set(selection.excludedPaths);
        if (wasSelected) {
          excludedPaths.add(path);
        } else {
          excludedPaths.delete(path);
        }
        nextSelection = { ...selection, excludedPaths };
        if (wasSelected) {
          nextSelectedModPath =
            sortedFolders.find(
              (folder) =>
                folder.path !== path && isWorkspaceExplorerPathSelected(selection, folder.path),
            )?.path ?? null;
        }
      } else {
        nextSelection = { mode: 'explicit', paths: new Set([path]) };
      }

      if (
        !selectMod(
          nextSelectedModPath,
          isMobile && nextSelectionSize === 1 ? 'details' : undefined,
          createSelectionEffect(nextSelection),
        )
      ) {
        return;
      }
      setAnchorId(path);
    },
    [createSelectionEffect, getRange, isMobile, selectMod, selection, setAnchorId, sortedFolders],
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
