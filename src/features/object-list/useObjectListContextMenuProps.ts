import { useMemo } from 'react';
import type { ContextMenuHandlerProps } from './ObjectListContent';

type UseObjectListContextMenuPropsInput = ContextMenuHandlerProps;

export function useObjectListContextMenuProps(
  input: UseObjectListContextMenuPropsInput,
): ContextMenuHandlerProps {
  const {
    isSyncing,
    categoryNames,
    handleEdit,
    handleSyncWithDb,
    handleDeleteObject,
    handlePin,
    handleMoveCategory,
    handleRevealInExplorer,
    handleEnableObject,
    handleDisableObject,
  } = input;

  return useMemo(
    () => ({
      isSyncing,
      categoryNames,
      handleEdit,
      handleSyncWithDb,
      handleDeleteObject,
      handlePin,
      handleMoveCategory,
      handleRevealInExplorer,
      handleEnableObject,
      handleDisableObject,
    }),
    [
      categoryNames,
      handleDeleteObject,
      handleDisableObject,
      handleEdit,
      handleEnableObject,
      handleMoveCategory,
      handlePin,
      handleRevealInExplorer,
      handleSyncWithDb,
      isSyncing,
    ],
  );
}
