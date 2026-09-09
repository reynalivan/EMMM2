/**
 * useFolderGridNav — Navigation logic extracted from useFolderGrid.
 *
 * Handles: breadcrumb navigation, deep folder entry, goHome, sort toggling.
 */

import { useCallback } from 'react';
import { useWorkspaceRuntime } from '@/features/workspace-runtime';

interface FolderGridNavOptions {
  currentPath: string[];
  explorerSubPath?: string;
  selectedObjectFolderPath: string | null;
}

export function useFolderGridNav({
  currentPath,
  explorerSubPath,
  selectedObjectFolderPath,
}: FolderGridNavOptions) {
  const runtime = useWorkspaceRuntime();

  const handleNavigate = useCallback(
    (folderName: string) => {
      const newPath = [...currentPath, folderName];
      const newSubPath = explorerSubPath ? `${explorerSubPath}/${folderName}` : folderName;
      runtime.navigateExplorer(newPath, newSubPath);
    },
    [currentPath, explorerSubPath, runtime],
  );

  const handleBreadcrumbClick = useCallback(
    (index: number) => {
      // If object is selected, we cannot navigate ABOVE the object level (index 0).
      // The Object Name is always at index 0 in this mode.
      if (selectedObjectFolderPath && index < 0) return;

      const newPath = currentPath.slice(0, index + 1);

      if (selectedObjectFolderPath) {
        if (index <= 0) {
          runtime.focusObject(selectedObjectFolderPath);
          return;
        }
        const nestedSegments = newPath.slice(1);
        const nextSubPath =
          nestedSegments.length > 0
            ? `${selectedObjectFolderPath}/${nestedSegments.join('/')}`
            : selectedObjectFolderPath;
        runtime.navigateExplorer(newPath, nextSubPath);
        return;
      }

      if (!explorerSubPath) {
        runtime.navigateExplorer(newPath, undefined);
        return;
      }

      const subSegments = explorerSubPath.split('/');
      const keepCount = index + 1;
      const truncated = subSegments.slice(0, keepCount);
      runtime.navigateExplorer(newPath, truncated.length > 0 ? truncated.join('/') : undefined);
    },
    [currentPath, explorerSubPath, runtime, selectedObjectFolderPath],
  );

  const handleGoHome = useCallback(() => {
    if (selectedObjectFolderPath) {
      runtime.focusObject(selectedObjectFolderPath);
      return;
    }

    runtime.navigateExplorer([], undefined);
  }, [runtime, selectedObjectFolderPath]);

  return {
    handleNavigate,
    handleBreadcrumbClick,
    handleGoHome,
  };
}
