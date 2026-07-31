import { useCallback, useEffect, useMemo } from 'react';
import type { ModFolder } from '../../../types/object';
import { useFolderNavigation } from './useFolderNavigation';
import { useRangeSelection } from '../../../hooks/useRangeSelection';
import { normalizeWorkspacePath } from '../../workspace-runtime/pathRewrite';

const getFolderPath = (folder: ModFolder) => folder.path;

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
  const { anchorId, setAnchorId, getRange } = useRangeSelection(sortedFolders, getFolderPath);
  const visiblePathKeys = useMemo(
    () => new Set(sortedFolders.map((folder) => normalizeWorkspacePath(folder.path))),
    [sortedFolders],
  );

  useEffect(() => {
    if (!anchorId || visiblePathKeys.has(normalizeWorkspacePath(anchorId))) {
      return;
    }

    const nextSelectedPath = Array.from(gridSelection).find((path) =>
      visiblePathKeys.has(normalizeWorkspacePath(path)),
    );
    if (nextSelectedPath) {
      setAnchorId(nextSelectedPath);
    }
  }, [anchorId, gridSelection, setAnchorId, visiblePathKeys]);

  const handleActivateItem = useCallback(
    (path: string) => {
      setGridSelection(new Set());
      selectMod(path, isMobile ? 'details' : undefined);
      setAnchorId(path);
    },
    [isMobile, selectMod, setAnchorId, setGridSelection],
  );

  const handleToggleSelection = useCallback(
    (path: string, multi: boolean, isShift?: boolean) => {
      if (isShift) {
        const range = getRange(path);
        if (range) {
          const nextSelection = new Set(gridSelection);
          for (const rangePath of range) {
            nextSelection.add(rangePath);
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
      setAnchorId(path);
    },
    [getRange, gridSelection, isMobile, selectMod, setAnchorId, setGridSelection],
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
