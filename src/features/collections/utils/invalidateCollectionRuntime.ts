import type { QueryClient } from '@tanstack/react-query';

import { collectionKeys } from '../queryKeys';
import { invalidateCorridorRuntime } from './invalidateCorridorRuntime';

interface InvalidateCollectionRuntimeOptions {
  includeMods?: boolean;
}

export async function invalidateCollectionRuntime(
  queryClient: QueryClient,
  options?: InvalidateCollectionRuntimeOptions,
) {
  queryClient.invalidateQueries({ queryKey: collectionKeys.all });
  if (options?.includeMods) {
    queryClient.invalidateQueries({ queryKey: ['mods'] });
  }
  queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
  queryClient.invalidateQueries({ queryKey: ['objects'] });
  await invalidateCorridorRuntime(queryClient);
  queryClient.invalidateQueries({ queryKey: ['dashboard'] });
}
