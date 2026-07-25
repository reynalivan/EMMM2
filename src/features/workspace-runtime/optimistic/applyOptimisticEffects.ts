import type { QueryClient } from '@tanstack/react-query';
import { thumbnailKeys } from '../../../hooks/useThumbnail';
import { publishQueryInvalidations } from '../../runtime-sync/queryRefresh';
import { patchObjectEnabledCount } from '../../../hooks/objectQueryCache';
import { dispatchWorkspaceRuntimeEvent } from '../state/workspaceStoreBridge';
import type { RuntimeEffectDescriptor } from '../../../lib/runtimeEffects';
import { applyWorkspacePathRewrites } from './workspaceViewModelRewrite';

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
