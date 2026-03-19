import type { QueryClient } from '@tanstack/react-query';

import { useAppStore } from '../../../stores/useAppStore';
import { refetchCollectionRuntime } from './refetchCollectionRuntime';

export async function refetchCurrentCorridorRuntime(queryClient: QueryClient, gameId: string) {
  const { safeMode } = useAppStore.getState();
  await refetchCollectionRuntime(queryClient, {
    gameId,
    isSafe: safeMode,
  });
}
