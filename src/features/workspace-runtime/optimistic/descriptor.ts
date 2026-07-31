import type { RuntimeEffectDescriptor } from '../../../lib/runtimeEffects';

// ponytail: six spelled-out lines on purpose. The return-type annotation already
// fails the build if a descriptor field is added and not merged here, and every
// keyed-loop form needs an `as RuntimeEffectDescriptor` that throws that away.
export function mergeRuntimeEffectDescriptors(
  ...descriptors: RuntimeEffectDescriptor[]
): RuntimeEffectDescriptor {
  return {
    rewrites: descriptors.flatMap((descriptor) => descriptor.rewrites),
    invalidatedPaths: descriptors.flatMap((descriptor) => descriptor.invalidatedPaths),
    thumbnailPaths: descriptors.flatMap((descriptor) => descriptor.thumbnailPaths),
    removedQueryKeys: descriptors.flatMap((descriptor) => descriptor.removedQueryKeys),
    invalidatedQueryKeys: descriptors.flatMap((descriptor) => descriptor.invalidatedQueryKeys),
    refreshEvents: descriptors.flatMap((descriptor) => descriptor.refreshEvents),
  };
}
