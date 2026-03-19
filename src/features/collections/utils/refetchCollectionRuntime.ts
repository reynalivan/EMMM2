import type { QueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';

import { collectionKeys, corridorRuntimeKeys } from '../queryKeys';
import type { Collection, CollectionRuntimePreview, CorridorRuntimeSnapshot } from '../../../types/collection';

interface RefetchCollectionRuntimeOptions {
  gameId: string;
  isSafe: boolean;
  collectionId?: string | null;
}

export async function refetchCollectionRuntime(
  queryClient: QueryClient,
  options: RefetchCollectionRuntimeOptions,
) {
  await Promise.all([
    queryClient.fetchQuery({
      queryKey: collectionKeys.list(options.gameId),
      queryFn: () => invoke<Collection[]>('list_collections', { gameId: options.gameId }),
      staleTime: 0,
    }),
    queryClient.fetchQuery({
      queryKey: corridorRuntimeKeys.snapshot(options.gameId, options.isSafe),
      queryFn: () =>
        invoke<CorridorRuntimeSnapshot>('get_corridor_runtime_snapshot', {
          gameId: options.gameId,
          isSafe: options.isSafe,
        }),
      staleTime: 0,
    }),
    options.collectionId
      ? queryClient.fetchQuery({
          queryKey: collectionKeys.runtimePreview(options.collectionId),
          queryFn: () =>
            invoke<CollectionRuntimePreview>('get_collection_runtime_preview', {
              collectionId: options.collectionId,
              gameId: options.gameId,
            }),
          staleTime: 0,
        })
      : Promise.resolve(),
  ]);
}
