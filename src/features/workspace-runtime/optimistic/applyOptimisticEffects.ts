import type { QueryClient } from '@tanstack/react-query';
import { thumbnailKeys } from '../../../hooks/useThumbnail';
import { publishQueryInvalidations } from '../../runtime-sync/queryRefresh';
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
import { useAppStore } from '../../../stores/useAppStore';
import { applyWorkspacePathRewrites } from './workspaceViewModelRewrite';

export interface OptimisticEffectSnapshot {
  objectSnapshot: ObjectListSnapshot;
  runtimeSnapshot: WorkspaceRuntimeState;
  gridSelectionSnapshot: Set<string>;
}

export function applyOptimisticEffects(
  queryClient: QueryClient,
  descriptor: RuntimeEffectDescriptor,
): OptimisticEffectSnapshot {
  const snapshot: OptimisticEffectSnapshot = {
    objectSnapshot: snapshotObjectListQueries(queryClient),
    runtimeSnapshot: getWorkspaceRuntimeState(),
    gridSelectionSnapshot: new Set(useAppStore.getState().gridSelection),
  };

  for (const effect of descriptor.objectCountDeltas) {
    patchObjectEnabledCount(queryClient, effect.objectId, effect.delta);
  }

  if (descriptor.rewrites.length > 0) {
    applyWorkspacePathRewrites(queryClient, descriptor.rewrites, 'internal');
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

  void publishQueryInvalidations(queryClient, descriptor.invalidatedQueryKeys, 'active');

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
    applyWorkspacePathRewrites(queryClient, descriptor.rewrites, 'internal');
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

  void publishQueryInvalidations(queryClient, descriptor.invalidatedQueryKeys, 'active');
}

export function rollbackOptimisticEffects(
  queryClient: QueryClient,
  snapshot: OptimisticEffectSnapshot,
): void {
  restoreObjectListQueries(queryClient, snapshot.objectSnapshot);
  restoreWorkspaceRuntimeState(snapshot.runtimeSnapshot);
  useAppStore.setState({ gridSelection: snapshot.gridSelectionSnapshot });
}
