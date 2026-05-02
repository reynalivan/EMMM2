import type { QueryClient } from '@tanstack/react-query';
import type { QueryRefetchType } from '../../runtime-sync/queryRefresh';
import { publishRuntimeDescriptor } from '../../runtime-sync/queryRefresh';
import { applyRuntimeEffects } from '../optimistic/applyOptimisticEffects';
import {
  buildQueryInvalidationDescriptor,
  buildPathInvalidationDescriptor,
  buildRuntimeMutationDescriptor,
} from '../optimistic/descriptorBuilders';
import { mergeRuntimeEffectDescriptors } from '../optimistic/descriptor';

type RuntimeMutationKind = Parameters<typeof buildRuntimeMutationDescriptor>[0];

// Use this mapper only for FE-owned optimistic/non-filesystem mutations.
// Disk Reconcile results must flow through applyDiskReconcileResult instead.
export async function applyRuntimeMutationResult(
  queryClient: QueryClient,
  mutationKind: RuntimeMutationKind,
  refetchType: QueryRefetchType = 'active',
): Promise<void> {
  const descriptor = buildRuntimeMutationDescriptor(mutationKind);
  applyRuntimeEffects(queryClient, descriptor);
  await publishRuntimeDescriptor(queryClient, descriptor, refetchType);
}

export async function applyRuntimeQueryInvalidationResult(
  queryClient: QueryClient,
  queryKeys: readonly (readonly unknown[])[],
  mutationKind: RuntimeMutationKind,
  refetchType: QueryRefetchType = 'active',
): Promise<void> {
  const descriptor = buildQueryInvalidationDescriptor(
    [...queryKeys],
    buildRuntimeMutationDescriptor(mutationKind).refreshEvents,
  );
  applyRuntimeEffects(queryClient, descriptor);
  await publishRuntimeDescriptor(queryClient, descriptor, refetchType);
}

export async function applyRuntimePathInvalidationMutationResult(
  queryClient: QueryClient,
  paths: string[],
  mutationKind: RuntimeMutationKind,
  refetchType: QueryRefetchType = 'active',
): Promise<void> {
  const invalidationDescriptor = paths.reduce(
    (descriptor, path) =>
      mergeRuntimeEffectDescriptors(descriptor, buildPathInvalidationDescriptor(path, [])),
    buildRuntimeMutationDescriptor(mutationKind),
  );
  applyRuntimeEffects(queryClient, invalidationDescriptor);
  await publishRuntimeDescriptor(queryClient, invalidationDescriptor, refetchType);
}
