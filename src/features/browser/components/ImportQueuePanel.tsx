import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useImportQueue } from '../hooks/useImportQueue';
import type { ImportJobItem } from '../types';
import { openImportBatchWizard } from '../../import-batches/launcher';

const PROCESSING = new Set(['queued', 'extracting', 'matching', 'placing', 'discovered', 'staged']);

const REVIEWABLE = new Set([
  'awaiting_category',
  'awaiting_destination',
  'ready',
  'skipped',
  'partial',
  'metadata_pending',
  'committing',
  'reconciling',
  'finalizing_metadata',
  'needs_review',
  'failed',
]);

type BatchRow = {
  batchId: string;
  representative: ImportJobItem;
  itemCount: number;
  status: string;
};

function groupJobs(jobs: ImportJobItem[]): BatchRow[] {
  const grouped = new Map<string, ImportJobItem[]>();
  for (const job of jobs) {
    const batchId = job.batch_id ?? job.id;
    const current = grouped.get(batchId) ?? [];
    current.push(job);
    grouped.set(batchId, current);
  }
  return [...grouped.entries()].map(([batchId, items]) => ({
    batchId,
    representative: items[0],
    itemCount: items.length,
    status:
      items.find((item) => REVIEWABLE.has(item.status))?.status ??
      items.find((item) => PROCESSING.has(item.status))?.status ??
      items[0].status,
  }));
}

export function ImportQueuePanel() {
  const { t } = useTranslation(['browser', 'match_wizard']);
  const { jobs, cancelBatch } = useImportQueue();
  const batches = useMemo(
    () =>
      groupJobs(jobs).filter((batch) => !['done', 'cancelled', 'canceled'].includes(batch.status)),
    [jobs],
  );

  if (batches.length === 0) return null;

  return (
    <div
      id="import-queue-panel"
      className="fixed bottom-6 left-6 z-55 bg-base-200 rounded-2xl shadow-xl border border-base-300 w-[340px]"
    >
      <div className="px-4 py-3 border-b border-base-300 flex items-center justify-between">
        <h3 className="text-sm font-semibold flex items-center gap-2">
          {t('import.title')}
          <span className="badge badge-primary badge-sm">{batches.length}</span>
        </h3>
      </div>
      <div className="max-h-64 overflow-y-auto">
        {batches.map((batch) => {
          const name =
            batch.representative.archive_path.split(/[/\\]/).pop() ??
            batch.representative.archive_path;
          const processing = PROCESSING.has(batch.status);
          const canReview = REVIEWABLE.has(batch.status) && batch.representative.batch_id;
          return (
            <div
              key={batch.batchId}
              className="flex items-center gap-3 px-4 py-3 border-b border-base-300/50 last:border-0"
            >
              {processing ? (
                <span className="loading loading-spinner loading-xs text-info shrink-0" />
              ) : (
                <span className="text-warning shrink-0">⚠</span>
              )}
              <div className="flex-1 min-w-0">
                <p className="text-xs font-medium truncate">{name}</p>
                <div className="flex gap-1 mt-1">
                  <span className="badge badge-xs badge-outline">{batch.status}</span>
                  <span className="badge badge-xs badge-ghost">
                    {t('match_wizard:batch_summary', {
                      count: batch.itemCount,
                      flow: 'browser',
                    })}
                  </span>
                </div>
                {batch.representative.error_msg && (
                  <p className="text-xs text-error truncate mt-1">
                    {batch.representative.error_msg}
                  </p>
                )}
              </div>
              <div className="flex gap-1 shrink-0">
                {canReview && (
                  <button
                    className="btn btn-warning btn-xs"
                    onClick={() =>
                      openImportBatchWizard({ kind: 'existing', batchId: batch.batchId })
                    }
                  >
                    {t('import.review')}
                  </button>
                )}
                {!processing && (
                  <button
                    className="btn btn-ghost btn-xs"
                    onClick={() => cancelBatch(batch.batchId)}
                  >
                    {t('import.skip')}
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
