import { useAppStore } from '@/app/store';

const ACTIVATION_TIMEOUT_MS = 5 * 60_000;

export function waitForGameActivationReady(gameId: string): Promise<void> {
  return new Promise((resolve, reject) => {
    let settled = false;
    let unsubscribe: () => void = () => undefined;

    const finish = (error?: Error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeoutId);
      unsubscribe();
      if (error) reject(error);
      else resolve();
    };

    const check = () => {
      const status = useAppStore.getState().gameActivationByGame[gameId];
      if (status?.phase === 'ready' && status.reconcile_revision !== null) {
        finish();
      } else if (status?.phase === 'failed' || status?.phase === 'source_unavailable') {
        finish(new Error(status.error ?? `Could not activate game '${gameId}'`));
      }
    };

    unsubscribe = useAppStore.subscribe(check);
    const timeoutId = setTimeout(
      () => finish(new Error(`Timed out waiting for game '${gameId}' to finish activation`)),
      ACTIVATION_TIMEOUT_MS,
    );
    check();
  });
}
