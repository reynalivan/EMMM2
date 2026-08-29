import type { ImportBatch } from '../../lib/bindings.gen';

const TERMINAL_BATCH_STATUSES = new Set(['done', 'cancelled']);

export function selectLatestResumableBatch(
  batches: ImportBatch[],
  activeGameId: string | null,
): ImportBatch | null {
  if (!activeGameId) return null;
  return (
    batches.find(
      (batch) => batch.gameId === activeGameId && !TERMINAL_BATCH_STATUSES.has(batch.status),
    ) ?? null
  );
}

export function needsSourceAnalysis(batch: ImportBatch): boolean {
  return batch.items.some(
    (item) =>
      item.status === 'discovered' ||
      item.status === 'staged' ||
      (item.status === 'failed' && item.matchCategory === null),
  );
}
