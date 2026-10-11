import { useCallback, useEffect, useRef, useState } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { DiskReconcilePhase, DiskReconcileProgress } from '@/shared/api/tauri/bindings';

const PHASE_ORDER: Record<DiskReconcilePhase, number> = {
  DiscoveringRoots: 0,
  ScanningRoots: 1,
  Projecting: 2,
  Finalizing: 3,
  Completed: 4,
  Failed: 4,
};

interface ProgressSubscription {
  active: boolean;
  unlisten?: UnlistenFn;
}

export function useOnboardingDiskProgress() {
  const [progress, setProgress] = useState<DiskReconcileProgress | null>(null);
  const subscription = useRef<ProgressSubscription | null>(null);

  const stop = useCallback(() => {
    const owned = subscription.current;
    subscription.current = null;
    if (owned) {
      owned.active = false;
      owned.unlisten?.();
    }
  }, []);

  useEffect(() => stop, [stop]);

  const start = useCallback(
    async (sessionId: string, gameId: string) => {
      stop();
      setProgress(null);
      const owned: ProgressSubscription = { active: true };
      subscription.current = owned;
      const runId = `${sessionId}:${gameId}`;
      const attemptFor = (id: string): number | null => {
        if (id === runId) return 0;
        if (!id.startsWith(`${runId}:`)) return null;
        const suffix = id.slice(runId.length + 1);
        const attempt = Number(suffix);
        return Number.isSafeInteger(attempt) && attempt > 0 && String(attempt) === suffix
          ? attempt
          : null;
      };
      try {
        const unlisten = await listen<DiskReconcileProgress>(
          'disk_reconcile:progress',
          ({ payload }) => {
            if (
              !owned.active ||
              subscription.current !== owned ||
              payload.game_id !== gameId ||
              payload.reason !== 'OnboardingCompleted' ||
              attemptFor(payload.run_id) === null
            ) {
              return;
            }
            setProgress((previous) => {
              if (previous) {
                const attempt = attemptFor(payload.run_id);
                const previousAttempt = attemptFor(previous.run_id);
                if (attempt === null || previousAttempt === null || attempt < previousAttempt)
                  return previous;
                if (attempt > previousAttempt) return payload;
                if (previous.phase === 'Completed' || previous.phase === 'Failed') return previous;
                if (PHASE_ORDER[payload.phase] < PHASE_ORDER[previous.phase]) return previous;
                if (
                  payload.phase === previous.phase &&
                  (payload.elapsed_ms < previous.elapsed_ms ||
                    payload.completed_units < previous.completed_units)
                ) {
                  return previous;
                }
              }
              return payload;
            });
          },
        );
        if (!owned.active || subscription.current !== owned) {
          unlisten();
          throw new Error('Onboarding progress subscription stopped before indexing');
        }
        owned.unlisten = unlisten;
      } catch (error) {
        if (subscription.current === owned) stop();
        throw error;
      }
    },
    [stop],
  );

  return { progress, start, stop };
}
