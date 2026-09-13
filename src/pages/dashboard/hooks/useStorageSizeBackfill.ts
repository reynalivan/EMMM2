import { useCallback, useEffect, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '@/shared/api/tauri/bindings.gen';
import type { StorageSizeBackfillStatus } from '@/shared/api/tauri/bindings.gen';
import { formatAppError } from '@/shared/lib/appError';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';
import { isDemoMode } from '@/shared/lib/appMode';

const POLL_INTERVAL_MS = 1_000;

export type { StorageSizeBackfillStatus } from '@/shared/api/tauri/bindings.gen';

function failedStatus(error: unknown): StorageSizeBackfillStatus {
  return {
    state: 'Failed',
    total_games: 0,
    completed_games: 0,
    current_game_id: null,
    errors: [error instanceof Error ? error.message : String(error)],
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
  const [status, setStatus] = useState<StorageSizeBackfillStatus | null>(null);
  const [retryAttempt, setRetryAttempt] = useState(0);

  useEffect(() => {
    if (isDemoMode) {
      return;
    }

    let isMounted = true;
    let pollTimer: number | undefined;
    let jobWasRunning = false;

    const clearPolling = () => {
      if (pollTimer !== undefined) {
        window.clearTimeout(pollTimer);
        pollTimer = undefined;
      }
    };

    const handleFailure = (error: unknown) => {
      if (!isMounted) return;
      clearPolling();
      setStatus(failedStatus(error));
    };

    const poll = () => {
      void commands.getStorageSizeBackfillStatus().then(handleStatus).catch(handleFailure);
    };

    const schedulePoll = () => {
      clearPolling();
      pollTimer = window.setTimeout(poll, POLL_INTERVAL_MS);
    };

    const handleStatus = (nextStatus: StorageSizeBackfillStatus) => {
      if (!isMounted) return;

      setStatus(nextStatus);
      if (nextStatus.state === 'Running') {
        jobWasRunning = true;
        schedulePoll();
        return;
      }

      clearPolling();
      if (nextStatus.state === 'Completed' && jobWasRunning) {
        jobWasRunning = false;
        void publishQueryScopes(queryClient, ['dashboard']);
      }
    };

    void startStorageSizeBackfill().then(handleStatus).catch(handleFailure);

    return () => {
      isMounted = false;
      clearPolling();
    };
  }, [queryClient, retryAttempt]);

  const retry = useCallback(() => {
    setStatus(null);
    setRetryAttempt((attempt) => attempt + 1);
  }, []);

  return { status, retry };
}
