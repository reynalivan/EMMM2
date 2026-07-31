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
  return useMemo(() => {
    // Every bulk action is "skip while disabled, run on the selection, then clear".
    const run =
      <TArgs extends unknown[]>(action: (ids: Set<string>, ...args: TArgs) => Promise<void>) =>
      (...args: TArgs): void => {
        if (mutationsDisabled) {
          return;
        }

        void action(bulkSelect.selectedIds, ...args).then(bulkSelect.clearSelection);
      };

    const openTagModal = (mode: BulkTagModalState['mode']) => () => {
      if (!mutationsDisabled) {
        setBulkTagModal({ open: true, mode });
      }
    };

    return {
      isAnySelected: activePane === 'objectList' && bulkSelect.isAnySelected,
      selectionCount: bulkSelect.selectionCount,
      mutationsDisabled,
      onDelete: run(handleBulkDelete),
      onPin: run(handleBulkPin),
      onEnable: run(handleBulkEnable),
      onDisable: run(handleBulkDisable),
      onAddTags: openTagModal('add'),
      onRemoveTags: openTagModal('remove'),
      onAutoRecognize: run(handleBulkAutoRecognize),
      onFavorite: run(handleBulkFavorite),
      onMarkSafe: run(handleBulkSafe),
      onClear: bulkSelect.clearSelection,
    };
  }, [
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
  ]);
}
