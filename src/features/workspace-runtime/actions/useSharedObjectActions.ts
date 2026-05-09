import { useCallback, useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../lib/bindings';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { runObjectBatchMutation } from '../../../hooks/objectQueryCache';
import { useDeleteObject, useUpdateObject } from '../../../hooks/useObjectMutations';
import { toast } from '../../../stores/useToastStore';
import type { GameSchema } from '../../../types/object';
import type { WorkspaceObjectNode } from '../../../types/workspace';
import { applyObjectCategoryAndRefresh, revealObjectInExplorer } from './sharedObjectActionOps';
import {
  INITIAL_SHARED_OBJECT_ACTION_STATE,
  parseObjectHasModsError,
  type SharedObjectAction,
  sharedObjectActionsReducer,
} from './sharedObjectActionsState';
import { useSharedObjectSyncActions } from './useSharedObjectSyncActions';
import { publishRuntimeDescriptor } from '../../runtime-sync/queryRefresh';
import { buildRuntimeMutationDescriptor } from '../optimistic/descriptorBuilders';
import {
  dispatchWorkspaceRuntimeEvent,
  getWorkspaceRuntimeState,
  useWorkspaceRuntimeSelector,
} from '../state/workspaceStoreBridge';
import { useWorkspaceSwitchActions } from './useWorkspaceSwitchActions';

interface SharedObjectActionsOptions {
  objects: WorkspaceObjectNode[];
  schema: GameSchema | undefined;
}

function buildSharedObjectActionState(
  dialogState: ReturnType<typeof getWorkspaceRuntimeState>['dialogState'],
) {
  if (dialogState.kind === 'objectEdit') {
    return {
      ...INITIAL_SHARED_OBJECT_ACTION_STATE,
      editObject: dialogState.object,
    };
  }
  if (dialogState.kind === 'objectDelete') {
    return {
      ...INITIAL_SHARED_OBJECT_ACTION_STATE,
      deleteObjectDialog: {
        open: true,
        id: dialogState.id,
        name: dialogState.name,
      },
    };
  }
  if (dialogState.kind === 'objectForceDelete') {
    return {
      ...INITIAL_SHARED_OBJECT_ACTION_STATE,
      forceDeleteObjectDialog: {
        open: true,
        id: dialogState.id,
        name: dialogState.name,
        count: dialogState.count,
      },
    };
  }
  if (dialogState.kind === 'objectSync') {
    return {
      ...INITIAL_SHARED_OBJECT_ACTION_STATE,
      syncConfirm: {
        open: true,
        objectId: dialogState.objectId,
        objectName: dialogState.objectName,
        itemType: dialogState.itemType,
        match: dialogState.match,
        isLoading: dialogState.isLoading,
        currentData: dialogState.currentData,
      },
    };
  }

  return INITIAL_SHARED_OBJECT_ACTION_STATE;
}

export function useSharedObjectActions(options: SharedObjectActionsOptions) {
  const { t } = useTranslation(['objects', 'common']);
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();
  const deleteObjectMutation = useDeleteObject();
  const updateObject = useUpdateObject();
  const switchActions = useWorkspaceSwitchActions();
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);

  const state = useMemo(() => buildSharedObjectActionState(dialogState), [dialogState]);

  const dispatch = useCallback(
    (action: SharedObjectAction) => {
      const reduced = sharedObjectActionsReducer(
        buildSharedObjectActionState(getWorkspaceRuntimeState().dialogState),
        action,
      );

      if (reduced.editObject) {
        dispatchWorkspaceRuntimeEvent({
          type: 'DIALOG_OPENED',
          dialog: { kind: 'objectEdit', object: reduced.editObject },
        });
        return;
      }

      if (reduced.deleteObjectDialog.open) {
        dispatchWorkspaceRuntimeEvent({
          type: 'DIALOG_OPENED',
          dialog: {
            kind: 'objectDelete',
            id: reduced.deleteObjectDialog.id,
            name: reduced.deleteObjectDialog.name,
          },
        });
        return;
      }

      if (reduced.forceDeleteObjectDialog.open) {
        dispatchWorkspaceRuntimeEvent({
          type: 'DIALOG_OPENED',
          dialog: {
            kind: 'objectForceDelete',
            id: reduced.forceDeleteObjectDialog.id,
            name: reduced.forceDeleteObjectDialog.name,
            count: reduced.forceDeleteObjectDialog.count,
          },
        });
        return;
      }

      if (reduced.syncConfirm.open) {
        dispatchWorkspaceRuntimeEvent({
          type: dialogState.kind === 'objectSync' ? 'DIALOG_UPDATED' : 'DIALOG_OPENED',
          dialog: {
            kind: 'objectSync',
            objectId: reduced.syncConfirm.objectId,
            objectName: reduced.syncConfirm.objectName,
            itemType: reduced.syncConfirm.itemType,
            match: reduced.syncConfirm.match,
            isLoading: reduced.syncConfirm.isLoading,
            currentData: reduced.syncConfirm.currentData,
          },
        });
        return;
      }

      if (dialogState.kind.startsWith('object')) {
        dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: dialogState.kind });
      }
    },
    [dialogState],
  );

  const categoryNames = useMemo(
    () =>
      options.schema?.categories.map((category) => ({
        name: category.name,
        label: category.label,
      })) ?? [],
    [options.schema],
  );

  const setEditObject = useCallback(
    (object: WorkspaceObjectNode | null) => {
      if (!object) {
        dispatch({ type: 'closeEdit' });
        return;
      }

      dispatch({ type: 'openEdit', object });
    },
    [dispatch],
  );

  const setDeleteObjectDialog = useCallback(
    (next: { open: boolean; id: string; name: string }) => {
      if (!next.open) {
        dispatch({ type: 'closeDelete' });
        return;
      }

      dispatch({ type: 'openDelete', id: next.id, name: next.name });
    },
    [dispatch],
  );

  const setForceDeleteObjectDialog = useCallback(
    (next: { open: boolean; id: string; name: string; count: number }) => {
      if (!next.open) {
        dispatch({ type: 'closeForceDelete' });
        return;
      }

      dispatch({
        type: 'openForceDelete',
        id: next.id,
        name: next.name,
        count: next.count,
      });
    },
    [dispatch],
  );

  const syncActions = useSharedObjectSyncActions({
    activeGame,
    objects: options.objects,
    syncConfirm: state.syncConfirm,
    updateObject,
    dispatch,
  });

  const handleDeleteObject = useCallback(
    (id: string) => {
      const object = options.objects.find((candidate) => candidate.id === id);
      if (!object) {
        return;
      }

      dispatch({ type: 'openDelete', id: object.id, name: object.name });
    },
    [dispatch, options.objects],
  );

  const confirmDeleteObject = useCallback(async () => {
    const { id, name } = state.deleteObjectDialog;

    try {
      await deleteObjectMutation.mutateAsync({ id, force: false });
      dispatch({ type: 'closeDelete' });
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('objectRows'),
        'active',
      );
      toast.success(t('create_modal.success_message', { name }));
    } catch (error) {
      dispatch({ type: 'closeDelete' });
      const count = parseObjectHasModsError(error);
      if (count !== null) {
        dispatch({ type: 'openForceDelete', id, name, count });
        return;
      }

      console.error('Failed to delete object:', error);
      toast.error(
        t('create_modal.error_message', {
          error: String((error as Record<string, unknown>)?.message ?? error),
        }),
      );
    }
  }, [deleteObjectMutation, dispatch, queryClient, state.deleteObjectDialog, t]);

  const confirmForceDeleteObject = useCallback(async () => {
    const { id, name } = state.forceDeleteObjectDialog;
    dispatch({ type: 'closeForceDelete' });

    try {
      await deleteObjectMutation.mutateAsync({ id, force: true });
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('objectRows'),
        'active',
      );
      toast.success(t('create_modal.success_message', { name }));
    } catch (error) {
      console.error('Failed to force delete object:', error);
      toast.error(
        t('create_modal.error_message', {
          error: String((error as Record<string, unknown>)?.message ?? error),
        }),
      );
    }
  }, [deleteObjectMutation, dispatch, queryClient, state.forceDeleteObjectDialog, t]);

  const handleEdit = useCallback(
    (id: string) => {
      const object = options.objects.find((candidate) => candidate.id === id);
      if (!object) {
        return;
      }

      dispatch({ type: 'openEdit', object });
    },
    [dispatch, options.objects],
  );

  const handlePin = useCallback(
    async (id: string) => {
      const object = options.objects.find((candidate) => candidate.id === id);
      if (!object) {
        return;
      }

      try {
        await runObjectBatchMutation({
          queryClient,
          applyOptimisticUpdate: (candidate) =>
            candidate.id === id ? { ...candidate, is_pinned: !object.is_pinned } : candidate,
          mutation: async () => {
            await commands.pinObject({ id, pin: !object.is_pinned });
          },
        });

        toast.success(
          t(object.is_pinned ? 'toasts.pin_removed_one' : 'toasts.pin_added_one', { count: 1 }),
        );
      } catch (error) {
        console.error('Failed to pin object:', error);
      }
    },
    [options.objects, queryClient, t],
  );

  const handleMoveCategory = useCallback(
    async (id: string, category: string, itemType: 'object' | 'folder') => {
      if (!activeGame) {
        return;
      }

      try {
        await applyObjectCategoryAndRefresh({
          activeGame,
          objectId: id,
          category,
          itemType,
          queryClient,
          updateObject,
        });
      } catch (error) {
        console.error('Failed to move category:', error);
      }
    },
    [activeGame, queryClient, updateObject],
  );

  const toggleObjectMods = useCallback(
    async (objectId: string, enable: boolean) => {
      const object = options.objects.find((candidate) => candidate.id === objectId);
      if (!object) {
        return;
      }

      await switchActions.setNodeEnabled(object, enable, 'object_list', {
        syncExplorerPath: false,
      });
    },
    [options.objects, switchActions],
  );

  const handleEnableObject = useCallback(
    (objectId: string) => toggleObjectMods(objectId, true),
    [toggleObjectMods],
  );

  const handleDisableObject = useCallback(
    (objectId: string) => toggleObjectMods(objectId, false),
    [toggleObjectMods],
  );

  const handleRevealInExplorer = useCallback(
    async (objectId: string) => {
      if (!activeGame) {
        return;
      }

      const object = options.objects.find((candidate) => candidate.id === objectId);
      try {
        await revealObjectInExplorer({
          activeGame,
          objectId,
          objectFolderPath: object?.folder_path,
        });
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        toast.error(message);
        void publishRuntimeDescriptor(
          queryClient,
          buildRuntimeMutationDescriptor('objectStructure'),
          'active',
        );
      }
    },
    [activeGame, options.objects, queryClient],
  );

  return {
    editObject: state.editObject,
    setEditObject,
    deleteObjectDialog: state.deleteObjectDialog,
    setDeleteObjectDialog,
    forceDeleteObjectDialog: state.forceDeleteObjectDialog,
    setForceDeleteObjectDialog,
    syncConfirm: state.syncConfirm,
    setSyncConfirm: syncActions.setSyncConfirm,
    categoryNames,
    isSwitchPending: switchActions.isPending,
    isObjectSwitchPending: switchActions.isNodePending,
    handleDeleteObject,
    confirmDeleteObject,
    confirmForceDeleteObject,
    handleEdit,
    handlePin,
    handleMoveCategory,
    toggleObjectMods,
    handleEnableObject,
    handleDisableObject,
    handleRevealInExplorer,
    handleSyncWithDb: syncActions.handleSyncWithDb,
    handleApplySyncMatch: syncActions.handleApplySyncMatch,
  };
}
