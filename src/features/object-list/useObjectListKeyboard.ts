import { useCallback, type FocusEvent, type KeyboardEvent } from 'react';
import type { WorkspaceObjectNode } from '../../types/workspace';
import type { useObjectBulkSelect } from './useObjectBulkSelect';

interface UseObjectListKeyboardInput {
  activePane: string;
  mutationsDisabled: boolean;
  selectedObjectFolderPath: string | null;
  objects: WorkspaceObjectNode[];
  bulkSelect: ReturnType<typeof useObjectBulkSelect>;
  setActivePane: (pane: 'objectList' | 'folderGrid') => void;
  handleBulkDelete: (ids: Set<string>) => Promise<void>;
  handleDeleteObject: (id: string) => void;
}

export function useObjectListKeyboard({
  activePane,
  mutationsDisabled,
  selectedObjectFolderPath,
  objects,
  bulkSelect,
  setActivePane,
  handleBulkDelete,
  handleDeleteObject,
}: UseObjectListKeyboardInput) {
  const onFocus = useCallback(
    (event: FocusEvent<HTMLDivElement>) => {
      if (!event.defaultPrevented) {
        setActivePane('objectList');
      }
    },
    [setActivePane],
  );

  const onKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>) => {
      if (activePane !== 'objectList') {
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key === 'a') {
        event.preventDefault();
        bulkSelect.selectAll();
        return;
      }

      if (event.key === 'Escape' && bulkSelect.isAnySelected) {
        event.preventDefault();
        bulkSelect.clearSelection();
        return;
      }

      if (event.key !== 'Delete' || mutationsDisabled) {
        return;
      }

      if (bulkSelect.isAnySelected) {
        event.preventDefault();
        void handleBulkDelete(bulkSelect.selectedIds).then(bulkSelect.clearSelection);
        return;
      }

      if (!selectedObjectFolderPath) {
        return;
      }

      const targetObject = objects.find(
        (object) => object.folder_path === selectedObjectFolderPath,
      );
      if (!targetObject) {
        return;
      }

      event.preventDefault();
      handleDeleteObject(targetObject.id);
    },
    [
      activePane,
      bulkSelect,
      handleBulkDelete,
      handleDeleteObject,
      mutationsDisabled,
      objects,
      selectedObjectFolderPath,
    ],
  );

  return { onFocus, onKeyDown };
}
