import type { QueryClient, RefetchType } from '@tanstack/react-query';

import { corridorPreviewKeys, corridorRuntimeKeys } from '../queryKeys';

interface InvalidateCorridorRuntimeOptions {
  refetchType?: RefetchType;
}

export async function invalidateCorridorRuntime(
  queryClient: QueryClient,
  options?: InvalidateCorridorRuntimeOptions,
) {
  await queryClient.invalidateQueries({
    queryKey: corridorRuntimeKeys.all,
    refetchType: options?.refetchType,
  });
  await queryClient.invalidateQueries({
    queryKey: corridorPreviewKeys.all,
    refetchType: options?.refetchType,
  });
}
