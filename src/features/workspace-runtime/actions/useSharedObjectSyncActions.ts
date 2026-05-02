import { useCallback } from 'react';
import type { Dispatch } from 'react';
import type { MatchedDbEntry } from '../../../lib/bindings';
import type { GameConfig } from '../../../types/game';
import type { UpdateObjectInput } from '../../../types/object';
import type { WorkspaceObjectNode } from '../../../types/workspace';
import {
  applyObjectSyncMatch,
  buildObjectSyncCurrentData,
  loadObjectSyncMatch,
} from './sharedObjectActionOps';
import type { SharedObjectAction, SyncConfirmState } from './sharedObjectActionsState';

interface UpdateObjectMutationLike {
  mutateAsync: (variables: { id: string; updates: UpdateObjectInput }) => Promise<unknown>;
}

interface SharedObjectSyncActionsOptions {
  activeGame: GameConfig | null;
  objects: WorkspaceObjectNode[];
  syncConfirm: SyncConfirmState;
  updateObject: UpdateObjectMutationLike;
  dispatch: Dispatch<SharedObjectAction>;
}

export function useSharedObjectSyncActions(options: SharedObjectSyncActionsOptions) {
  const setSyncConfirm = useCallback(
    (next: SyncConfirmState) => {
      if (!next.open || !next.currentData) {
        options.dispatch({ type: 'closeSync' });
        return;
      }

      options.dispatch({
        type: 'openSync',
        objectId: next.objectId,
        objectName: next.objectName,
        currentData: next.currentData,
      });
      options.dispatch({
        type: 'setSyncMatch',
        match: next.match,
        isLoading: next.isLoading,
      });
    },
    [options],
  );

  const handleSyncWithDb = useCallback(
    async (objectId: string, objectName: string) => {
      if (!options.activeGame) {
        return;
      }

      const object = options.objects.find((candidate) => candidate.id === objectId);
      options.dispatch({
        type: 'openSync',
        objectId,
        objectName,
        currentData: buildObjectSyncCurrentData(object, objectName),
      });

      try {
        const match = await loadObjectSyncMatch({
          activeGame: options.activeGame,
          objectName,
        });
        options.dispatch({ type: 'setSyncMatch', match, isLoading: false });
      } catch (error) {
        console.error('Match failed:', error);
        options.dispatch({ type: 'setSyncMatch', match: null, isLoading: false });
      }
    },
    [options],
  );

  const handleApplySyncMatch = useCallback(
    async (match: MatchedDbEntry) => {
      const objectId = options.syncConfirm.objectId;
      if (!objectId || !options.activeGame) {
        return;
      }

      try {
        await applyObjectSyncMatch({
          activeGame: options.activeGame,
          objectId,
          match,
          updateObject: options.updateObject,
        });
        options.dispatch({ type: 'closeSync' });
      } catch (error) {
        console.error('Apply sync match failed:', error);
      }
    },
    [options],
  );

  return {
    setSyncConfirm,
    handleSyncWithDb,
    handleApplySyncMatch,
  };
}
