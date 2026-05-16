import { useMemo } from 'react';
import type { useObjectBulkSelect } from './useObjectBulkSelect';

interface BulkTagModalState {
  open: boolean;
  mode: 'add' | 'remove';
}

interface UseObjectListBulkToolbarPropsInput {
  activePane: string;
  mutationsDisabled: boolean;
  bulkSelect: ReturnType<typeof useObjectBulkSelect>;
  setBulkTagModal: (state: BulkTagModalState) => void;
  handleBulkDelete: (ids: Set<string>) => Promise<void>;
  handleBulkPin: (ids: Set<string>, pin: boolean) => Promise<void>;
  handleBulkEnable: (ids: Set<string>) => Promise<void>;
  handleBulkDisable: (ids: Set<string>) => Promise<void>;
  handleBulkAutoRecognize: (ids: Set<string>) => Promise<void>;
  handleBulkFavorite: (ids: Set<string>, favorite: boolean) => Promise<void>;
  handleBulkSafe: (ids: Set<string>, safe: boolean) => Promise<void>;
}

function runWhenAvailable(mutationsDisabled: boolean, action: () => Promise<void>): void {
  if (mutationsDisabled) {
    return;
  }

  void action();
}

export function useObjectListBulkToolbarProps({
  activePane,
  mutationsDisabled,
  bulkSelect,
  setBulkTagModal,
  handleBulkDelete,
  handleBulkPin,
  handleBulkEnable,
  handleBulkDisable,
  handleBulkAutoRecognize,
  handleBulkFavorite,
  handleBulkSafe,
}: UseObjectListBulkToolbarPropsInput) {
  return useMemo(
    () => ({
      isAnySelected: activePane === 'objectList' && bulkSelect.isAnySelected,
      selectionCount: bulkSelect.selectionCount,
      mutationsDisabled,
      onDelete: () =>
        runWhenAvailable(mutationsDisabled, () =>
          handleBulkDelete(bulkSelect.selectedIds).then(bulkSelect.clearSelection),
        ),
      onPin: (pin: boolean) =>
        runWhenAvailable(mutationsDisabled, () =>
          handleBulkPin(bulkSelect.selectedIds, pin).then(bulkSelect.clearSelection),
        ),
      onEnable: () =>
        runWhenAvailable(mutationsDisabled, () =>
          handleBulkEnable(bulkSelect.selectedIds).then(bulkSelect.clearSelection),
        ),
      onDisable: () =>
        runWhenAvailable(mutationsDisabled, () =>
          handleBulkDisable(bulkSelect.selectedIds).then(bulkSelect.clearSelection),
        ),
      onAddTags: () => {
        if (!mutationsDisabled) {
          setBulkTagModal({ open: true, mode: 'add' });
        }
      },
      onRemoveTags: () => {
        if (!mutationsDisabled) {
          setBulkTagModal({ open: true, mode: 'remove' });
        }
      },
      onAutoRecognize: () =>
        runWhenAvailable(mutationsDisabled, () =>
          handleBulkAutoRecognize(bulkSelect.selectedIds).then(bulkSelect.clearSelection),
        ),
      onFavorite: (favorite: boolean) =>
        runWhenAvailable(mutationsDisabled, () =>
          handleBulkFavorite(bulkSelect.selectedIds, favorite).then(bulkSelect.clearSelection),
        ),
      onMarkSafe: (safe: boolean) =>
        runWhenAvailable(mutationsDisabled, () =>
          handleBulkSafe(bulkSelect.selectedIds, safe).then(bulkSelect.clearSelection),
        ),
      onClear: bulkSelect.clearSelection,
    }),
    [
      activePane,
      bulkSelect.isAnySelected,
      bulkSelect.selectionCount,
      bulkSelect.selectedIds,
      bulkSelect.clearSelection,
      handleBulkDelete,
      handleBulkPin,
      handleBulkEnable,
      handleBulkDisable,
      handleBulkAutoRecognize,
      handleBulkFavorite,
      handleBulkSafe,
      mutationsDisabled,
      setBulkTagModal,
    ],
  );
}
