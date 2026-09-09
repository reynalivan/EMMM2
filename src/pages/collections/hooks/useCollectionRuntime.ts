import { useQuery } from '@tanstack/react-query';
import { commands } from '../../../shared/api/tauri/bindings';
import type { CollectionRuntimeDescriptor, CollectionRuntimeSnapshot } from '@/entities/collection';
import { collectionRuntimeKeys } from '../queryKeys';

export function useCollectionRuntime(gameId: string | null) {
  return useQuery<CollectionRuntimeSnapshot>({
    queryKey: collectionRuntimeKeys.state(gameId ?? ''),
    queryFn: () => commands.getCollectionRuntimeState(gameId ?? ''),
    enabled: !!gameId,
    staleTime: 5_000,
  });
}

/** Compact runtime status for global surfaces; full snapshots remain Collections page-only. */
export function useCollectionRuntimeDescriptor(gameId: string | null) {
  return useQuery<CollectionRuntimeDescriptor>({
    queryKey: collectionRuntimeKeys.descriptor(gameId ?? ''),
    queryFn: () => commands.getCollectionRuntimeDescriptor(gameId ?? ''),
    enabled: !!gameId,
    staleTime: 5_000,
  });
}
