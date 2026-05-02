import { useMemo } from 'react';
import type { GameSchema } from '../../types/object';
import type { WorkspaceObjectNode } from '../../types/workspace';
import { useSharedObjectActions } from '../workspace-runtime/actions/useSharedObjectActions';

interface CrudDeps {
  objects: WorkspaceObjectNode[];
  schema: GameSchema | undefined;
}

export function useObjHandlersCrud({ objects, schema }: CrudDeps) {
  const actions = useSharedObjectActions({ objects, schema });

  return useMemo(
    () => ({
      editObject: actions.editObject,
      setEditObject: actions.setEditObject,
      deleteObjectDialog: actions.deleteObjectDialog,
      setDeleteObjectDialog: actions.setDeleteObjectDialog,
      forceDeleteObjectDialog: actions.forceDeleteObjectDialog,
      setForceDeleteObjectDialog: actions.setForceDeleteObjectDialog,
      syncConfirm: actions.syncConfirm,
      setSyncConfirm: actions.setSyncConfirm,
      handleDeleteObject: actions.handleDeleteObject,
      confirmDeleteObject: actions.confirmDeleteObject,
      confirmForceDeleteObject: actions.confirmForceDeleteObject,
      handleEdit: actions.handleEdit,
      handlePin: actions.handlePin,
      handleMoveCategory: actions.handleMoveCategory,
      categoryNames: actions.categoryNames,
      isSwitchPending: actions.isSwitchPending,
      isObjectSwitchPending: actions.isObjectSwitchPending,
      toggleObjectMods: actions.toggleObjectMods,
      handleEnableObject: actions.handleEnableObject,
      handleDisableObject: actions.handleDisableObject,
      handleRevealInExplorer: actions.handleRevealInExplorer,
      handleSyncWithDb: actions.handleSyncWithDb,
      handleApplySyncMatch: actions.handleApplySyncMatch,
    }),
    [actions],
  );
}
