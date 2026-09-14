import { useCallback, useEffect, useRef, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { commands } from '@/shared/api/tauri/bindings.gen';
import type { StorageSizeBackfillStatus } from '@/shared/api/tauri/bindings.gen';
import { formatAppError } from '@/shared/lib/appError';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';
import { isDemoMode } from '@/shared/lib/appMode';

const POLL_INTERVAL_MS = 1_000;
const storageSizeBackfillKey = ['storage-size-backfill'] as const;

export type { StorageSizeBackfillStatus } from '@/shared/api/tauri/bindings.gen';

function failedStatus(error: unknown): StorageSizeBackfillStatus {
  return {
    state: 'Failed',
    total_games: 0,
    completed_games: 0,
    current_game_id: null,
    errors: [formatAppError(error)],
  };
}

async function startStorageSizeBackfill(): Promise<StorageSizeBackfillStatus> {
  const result = await commands.startStorageSizeBackfill();
  if (result.status === 'error') {
    throw new Error(formatAppError(result.error));
  }

  return result.data;
}

export function useStorageSizeBackfill() {
  const queryClient = useQueryClient();
  const [attempt, setAttempt] = useState(0);
  const [startStatus, setStartStatus] = useState<StorageSizeBackfillStatus | null>(null);
  const [startError, setStartError] = useState<unknown>(null);
  const runningSinceMount = useRef(false);
  const startMutation = useMutation({
    mutationFn: startStorageSizeBackfill,
    networkMode: 'always',
    onMutate: () => {
      setStartStatus(null);
      setStartError(null);
    },
    onSuccess: (status) => {
      setStartStatus(status);
      queryClient.setQueryData(storageSizeBackfillKey, status);
    },
    onError: (error) => {
      setStartError(error);
      queryClient.setQueryData(storageSizeBackfillKey, failedStatus(error));
    },
  });
  const { mutate: startBackfill, reset: resetStartBackfill } = startMutation;
  const statusQuery = useQuery<StorageSizeBackfillStatus>({
    queryKey: storageSizeBackfillKey,
    queryFn: () => commands.getStorageSizeBackfillStatus(),
    networkMode: 'always',
    enabled: !isDemoMode && startMutation.isSuccess,
    staleTime: 0,
    retry: false,
    refetchInterval: (query) =>
      query.state.error || query.state.data?.state !== 'Running' ? false : POLL_INTERVAL_MS,
  });
  const status = statusQuery.error
    ? failedStatus(statusQuery.error)
    : (statusQuery.data ?? startStatus ?? (startError ? failedStatus(startError) : null));

  useEffect(() => {
    if (isDemoMode) {
      return;
    }
    startBackfill();
  }, [attempt, startBackfill]);

  useEffect(() => {
    if (status?.state === 'Running') {
      runningSinceMount.current = true;
      return;
    }
    if (status?.state === 'Completed' && runningSinceMount.current) {
      runningSinceMount.current = false;
      void publishQueryScopes(queryClient, ['dashboard']);
    }
  }, [queryClient, status?.state]);

  const retry = useCallback(() => {
    queryClient.removeQueries({ queryKey: storageSizeBackfillKey });
    resetStartBackfill();
    setStartStatus(null);
    setStartError(null);
    setAttempt((current) => current + 1);
  }, [queryClient, resetStartBackfill]);

  return { status, retry };
}
