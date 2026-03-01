/**
 * useFolderGridNav — Navigation logic extracted from useFolderGrid.
 *
 * Handles: breadcrumb navigation, deep folder entry, goHome, sort toggling.
 */

import { useCallback } from 'react';
import type { SortField, SortOrder } from '../../../types/mod';
import type { ObjectSummary } from '../../../types/object';

interface FolderGridNavOptions {
  currentPath: string[];
  explorerSubPath?: string;
  selectedObject: string | null;
  objects: ObjectSummary[];
  setCurrentPath: (path: string[]) => void;
  setExplorerSubPath: (subPath?: string) => void;
  setExplorerScrollOffset: (offset: number) => void;
  clearGridSelection: () => void;
  sortField: SortField;
  sortOrder: SortOrder;
  setSortField: (field: SortField) => void;
  setSortOrder: (order: SortOrder) => void;
}

export function useFolderGridNav({
  currentPath,
  explorerSubPath,
  selectedObject,
  objects,
  setCurrentPath,
  setExplorerSubPath,
  setExplorerScrollOffset,
  clearGridSelection,
  sortField,
  sortOrder,
  setSortField,
  setSortOrder,
}: FolderGridNavOptions) {
  const handleNavigate = useCallback(
    (folderName: string) => {
      const newPath = [...currentPath, folderName];
      setCurrentPath(newPath);
      // Build the new filesystem sub-path by appending to the current one.
      // This ensures we keep the full physical path (e.g., "hook/HookAsDionaMod-new")
      // instead of losing the parent when stripping the object display name.
      const newSubPath = explorerSubPath ? `${explorerSubPath}/${folderName}` : folderName;
      setExplorerSubPath(newSubPath);
      setExplorerScrollOffset(0);
      clearGridSelection();
    },
    [
      currentPath,
      explorerSubPath,
      setCurrentPath,
      setExplorerSubPath,
      setExplorerScrollOffset,
      clearGridSelection,
    ],
  );

  const handleBreadcrumbClick = useCallback(
    (index: number) => {
      // If object is selected, we cannot navigate ABOVE the object level (index 0).
      // The Object Name is always at index 0 in this mode.
      if (selectedObject && index < 0) return;

      const newPath = currentPath.slice(0, index + 1);
      setCurrentPath(newPath);

      // Build the filesystem sub-path from explorerSubPath segments.
      // In object mode, explorerSubPath first segment is the object's physical folder name.
      // Number of FS segments to keep = index + 1 when object is selected (because
      // currentPath[0]=displayName maps to subPath segment[0]=physicalFolder),
      // or simply index + 1 when no object is selected.
      if (!explorerSubPath) {
        // Already at root — nothing to truncate
        setExplorerSubPath(undefined);
      } else {
        const subSegments = explorerSubPath.split('/');
        // In object mode: currentPath[0] = display name, subSegments[0] = physical folder
        // Clicking index 0 → keep 1 segment; clicking index 1 → keep 2 segments; etc.
        const keepCount = selectedObject ? index + 1 : index + 1;
        const truncated = subSegments.slice(0, keepCount);
        setExplorerSubPath(truncated.length > 0 ? truncated.join('/') : undefined);
      }

      setExplorerScrollOffset(0);
      clearGridSelection();
    },
    [
      currentPath,
      explorerSubPath,
      setCurrentPath,
      setExplorerSubPath,
      setExplorerScrollOffset,
      clearGridSelection,
      selectedObject,
    ],
  );

  const handleGoHome = useCallback(() => {
    // If an object is selected, "Home" means the object's root view (DB-filtered, no FS sub-path).
    if (selectedObject) {
      const obj = objects.find((o) => o.id === selectedObject);
      if (obj) {
        setCurrentPath([obj.name]);
        setExplorerSubPath(undefined);
        setExplorerScrollOffset(0);
        clearGridSelection();
        return;
      }
    }

    // Default behavior (No object selected) - Go to Game Root
    setCurrentPath([]);
    setExplorerSubPath(undefined);
    setExplorerScrollOffset(0);
    clearGridSelection();
  }, [
    setCurrentPath,
    setExplorerSubPath,
    setExplorerScrollOffset,
    clearGridSelection,
    selectedObject,
    objects,
  ]);

  const handleSortToggle = useCallback(() => {
    const fields = ['name', 'modified_at', 'size_bytes'] as const;
    const currentIdx = fields.indexOf(sortField);
    if (sortOrder === 'desc') {
      const nextIdx = (currentIdx + 1) % fields.length;
      setSortField(fields[nextIdx]);
      setSortOrder('asc');
    } else {
      setSortOrder('desc');
    }
  }, [sortField, sortOrder, setSortField, setSortOrder]);

  const sortLabel = sortField === 'name' ? 'Name' : sortField === 'modified_at' ? 'Date' : 'Size';

  return {
    handleNavigate,
    handleBreadcrumbClick,
    handleGoHome,
    handleSortToggle,
    sortLabel,
  };
}
