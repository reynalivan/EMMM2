import { useCallback, useEffect, useMemo, useState } from 'react';
import type { ModFolder } from '../../../types/mod';
import { useFolderNavigation } from './useFolderNavigation';
import { normalizeWorkspacePath } from '../../workspace-runtime/pathRewrite';

interface UseFolderGridSelectionOptions {
  sortedFolders: ModFolder[];
  gridSelection: Set<string>;
  setGridSelection: (selection: Set<string>) => void;
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
  gridSelection,
  setGridSelection,
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
  const [lastSelectedPath, setLastSelectedPath] = useState<string | null>(null);
  const visiblePathKeys = useMemo(
    () => new Set(sortedFolders.map((folder) => normalizeWorkspacePath(folder.path))),
    [sortedFolders],
  );

  useEffect(() => {
    if (!lastSelectedPath || visiblePathKeys.has(normalizeWorkspacePath(lastSelectedPath))) {
      return;
    }

    const nextSelectedPath = Array.from(gridSelection).find((path) =>
      visiblePathKeys.has(normalizeWorkspacePath(path)),
    );
    if (nextSelectedPath) {
      setLastSelectedPath(nextSelectedPath);
    }
  }, [gridSelection, lastSelectedPath, visiblePathKeys]);

  const handleActivateItem = useCallback(
    (path: string) => {
      setGridSelection(new Set());
      selectMod(path, isMobile ? 'details' : undefined);
      setLastSelectedPath(path);
    },
    [isMobile, selectMod, setGridSelection],
  );

  const handleToggleSelection = useCallback(
    (path: string, multi: boolean, isShift?: boolean) => {
      if (isShift && lastSelectedPath) {
        const startIdx = sortedFolders.findIndex((folder) => folder.path === lastSelectedPath);
        const endIdx = sortedFolders.findIndex((folder) => folder.path === path);

        if (startIdx !== -1 && endIdx !== -1) {
          const min = Math.min(startIdx, endIdx);
          const max = Math.max(startIdx, endIdx);
          const nextSelection = new Set(gridSelection);

          for (let index = min; index <= max; index += 1) {
            nextSelection.add(sortedFolders[index].path);
          }

          setGridSelection(nextSelection);
          selectMod(path, isMobile ? 'details' : undefined);
          return;
        }
      }

      const nextSelection = new Set(multi ? gridSelection : []);
      if (nextSelection.has(path)) {
        nextSelection.delete(path);
      } else {
        nextSelection.add(path);
      }

      setGridSelection(nextSelection);
      const nextSelectedModPath =
        nextSelection.size > 0 ? Array.from(nextSelection)[nextSelection.size - 1] : null;
      selectMod(nextSelectedModPath, isMobile && nextSelection.size === 1 ? 'details' : undefined);
      setLastSelectedPath(path);
    },
    [gridSelection, isMobile, lastSelectedPath, selectMod, setGridSelection, sortedFolders],
  );

  const { focusedId, handleKeyDown } = useFolderNavigation({
    items: sortedFolders,
    gridColumns: isGridView ? columnCount : 1,
    getId: (item: ModFolder) => item.path,
    onNavigate: (item: ModFolder) => handleNavigate(item.folder_name),
    onSelectionChange: (item: ModFolder, multi: boolean, isShift?: boolean) =>
      handleToggleSelection(item.path, multi, isShift),
    onSelectAll: () => setGridSelection(new Set(sortedFolders.map((folder) => folder.path))),
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
