import type { QueryClient } from '@tanstack/react-query';
import { thumbnailKeys } from '../../../hooks/useThumbnail';
import {
  patchObjectEnabledCount,
  restoreObjectListQueries,
  snapshotObjectListQueries,
  type ObjectListSnapshot,
} from '../../../hooks/objectQueryCache';
import {
  dispatchWorkspaceRuntimeEvent,
  getWorkspaceRuntimeState,
  restoreWorkspaceRuntimeState,
} from '../state/workspaceStoreBridge';
import type { WorkspaceRuntimeState } from '../state/workspaceState';
import type { RuntimeEffectDescriptor } from './descriptor';

export interface OptimisticEffectSnapshot {
  objectSnapshot: ObjectListSnapshot;
  runtimeSnapshot: WorkspaceRuntimeState;
}

export function applyOptimisticEffects(
  queryClient: QueryClient,
  descriptor: RuntimeEffectDescriptor,
): OptimisticEffectSnapshot {
  const snapshot: OptimisticEffectSnapshot = {
    objectSnapshot: snapshotObjectListQueries(queryClient),
    runtimeSnapshot: getWorkspaceRuntimeState(),
  };

  for (const effect of descriptor.objectCountDeltas) {
    patchObjectEnabledCount(queryClient, effect.objectId, effect.delta);
  }

  if (descriptor.rewrites.length > 0) {
    dispatchWorkspaceRuntimeEvent({
      type: 'PATHS_REWRITTEN',
      rewrites: descriptor.rewrites,
    });
  }

  if (descriptor.invalidatedPaths.length > 0) {
    dispatchWorkspaceRuntimeEvent({
      type: 'TARGETS_INVALIDATED',
      paths: descriptor.invalidatedPaths,
      resetExplorer: true,
    });
  }

  for (const thumbnailPath of descriptor.thumbnailPaths) {
    queryClient.removeQueries({ queryKey: thumbnailKeys.folder(thumbnailPath) });
  }

  for (const queryKey of descriptor.removedQueryKeys) {
    queryClient.removeQueries({ queryKey });
  }

  for (const queryKey of descriptor.invalidatedQueryKeys) {
    void queryClient.invalidateQueries({
      queryKey,
      refetchType: 'active',
    });
  }

  return snapshot;
}

export function applyRuntimeEffects(
  queryClient: QueryClient,
  descriptor: RuntimeEffectDescriptor,
): void {
  for (const effect of descriptor.objectCountDeltas) {
    patchObjectEnabledCount(queryClient, effect.objectId, effect.delta);
  }

  if (descriptor.rewrites.length > 0) {
    dispatchWorkspaceRuntimeEvent({
      type: 'PATHS_REWRITTEN',
      rewrites: descriptor.rewrites,
    });
  }

  if (descriptor.invalidatedPaths.length > 0) {
    dispatchWorkspaceRuntimeEvent({
      type: 'TARGETS_INVALIDATED',
      paths: descriptor.invalidatedPaths,
      resetExplorer: true,
    });
  }

  for (const thumbnailPath of descriptor.thumbnailPaths) {
    queryClient.removeQueries({ queryKey: thumbnailKeys.folder(thumbnailPath) });
  }

  for (const queryKey of descriptor.removedQueryKeys) {
    queryClient.removeQueries({ queryKey });
  }

  for (const queryKey of descriptor.invalidatedQueryKeys) {
    void queryClient.invalidateQueries({
      queryKey,
      refetchType: 'active',
    });
  }
}

export function rollbackOptimisticEffects(
  queryClient: QueryClient,
  snapshot: OptimisticEffectSnapshot,
): void {
  restoreObjectListQueries(queryClient, snapshot.objectSnapshot);
  restoreWorkspaceRuntimeState(snapshot.runtimeSnapshot);
}
