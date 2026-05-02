import type { RuntimeRefreshEvent } from '../../runtime-sync/queryRefresh';

export interface ObjectCountDeltaEffect {
  objectId: string;
  delta: number;
}

export interface RuntimeEffectDescriptor {
  rewrites: Array<{ oldPath: string; newPath: string }>;
  invalidatedPaths: string[];
  objectCountDeltas: ObjectCountDeltaEffect[];
  thumbnailPaths: string[];
  removedQueryKeys: Array<readonly unknown[]>;
  invalidatedQueryKeys: Array<readonly unknown[]>;
  refreshEvents: RuntimeRefreshEvent[];
}

export const EMPTY_RUNTIME_EFFECT_DESCRIPTOR: RuntimeEffectDescriptor = {
  rewrites: [],
  invalidatedPaths: [],
  objectCountDeltas: [],
  thumbnailPaths: [],
  removedQueryKeys: [],
  invalidatedQueryKeys: [],
  refreshEvents: [],
};

export function mergeRuntimeEffectDescriptors(
  ...descriptors: RuntimeEffectDescriptor[]
): RuntimeEffectDescriptor {
  return {
    rewrites: descriptors.flatMap((descriptor) => descriptor.rewrites),
    invalidatedPaths: descriptors.flatMap((descriptor) => descriptor.invalidatedPaths),
    objectCountDeltas: descriptors.flatMap((descriptor) => descriptor.objectCountDeltas),
    thumbnailPaths: descriptors.flatMap((descriptor) => descriptor.thumbnailPaths),
    removedQueryKeys: descriptors.flatMap((descriptor) => descriptor.removedQueryKeys),
    invalidatedQueryKeys: descriptors.flatMap((descriptor) => descriptor.invalidatedQueryKeys),
    refreshEvents: descriptors.flatMap((descriptor) => descriptor.refreshEvents),
  };
}
