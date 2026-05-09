import { commands } from '../../lib/bindings';

export interface WatcherSuppressionOptions {
  releaseDelayMs: number | null;
}

function waitForDelay(delayMs: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, delayMs);
  });
}

export async function withWatcherSuppression<T>(
  options: WatcherSuppressionOptions,
  operation: () => Promise<T>,
): Promise<T> {
  await commands.setWatcherSuppression({ suppressed: true });
  try {
    return await operation();
  } finally {
    if (options.releaseDelayMs !== null && options.releaseDelayMs > 0) {
      await waitForDelay(options.releaseDelayMs);
    }
    await commands.setWatcherSuppression({ suppressed: false });
  }
}
