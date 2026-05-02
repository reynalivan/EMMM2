import { useEffect, useRef } from 'react';
import { join } from '@tauri-apps/api/path';
import { commands } from '../../../lib/bindings';

interface UseObjectSelectionRepairParams {
  modRootPath: string | null;
  selectedObjectFolderPath: string | null;
  clearSelection: () => void;
  requestRepairSync: () => Promise<void>;
}

export function useObjectSelectionRepair({
  modRootPath,
  selectedObjectFolderPath,
  clearSelection,
  requestRepairSync,
}: UseObjectSelectionRepairParams) {
  const repairedSelectionKeyRef = useRef<string | null>(null);

  useEffect(() => {
    if (!modRootPath || !selectedObjectFolderPath) {
      repairedSelectionKeyRef.current = null;
      return;
    }

    const selectionKey = `${modRootPath}\0${selectedObjectFolderPath}`;
    if (repairedSelectionKeyRef.current === selectionKey) {
      return;
    }

    let isCancelled = false;

    const validateSelection = async () => {
      try {
        const fullPath = await join(modRootPath, selectedObjectFolderPath);
        const folderExists = await commands.checkPathExistsCmd({ path: fullPath });

        if (isCancelled || folderExists) {
          return;
        }

        repairedSelectionKeyRef.current = selectionKey;
        clearSelection();
        await requestRepairSync();
      } catch (error) {
        console.error('Failed to validate selected object path:', error);
      }
    };

    void validateSelection();

    return () => {
      isCancelled = true;
    };
  }, [clearSelection, modRootPath, requestRepairSync, selectedObjectFolderPath]);
}

