import { useImportQueue } from '../hooks/useImportQueue';
import { NeedsReviewModal } from './NeedsReviewModal';
import { useState } from 'react';
import type { ImportJobItem, ImportJobStatus } from '../types';

const STATUS_BADGE: Record<ImportJobStatus, { label: string; cls: string }> = {
  queued: { label: 'Queued', cls: 'badge-neutral' },
  extracting: { label: 'Extracting', cls: 'badge-info' },
  matching: { label: 'Matching', cls: 'badge-info' },
  needs_review: { label: 'Review', cls: 'badge-warning' },
  placing: { label: 'Placing', cls: 'badge-info' },
  done: { label: 'Done', cls: 'badge-success' },
  failed: { label: 'Failed', cls: 'badge-error' },
  canceled: { label: 'Canceled', cls: 'badge-ghost' },
};

export function ImportQueuePanel() {
  const { jobs, confirmJob, skipJob } = useImportQueue();
  const [reviewingJob, setReviewingJob] = useState<ImportJobItem | null>(null);

  const activeJobs = jobs.filter((j) => j.status !== 'done' && j.status !== 'canceled');

  if (activeJobs.length === 0) return null;

  return (
    <>
      {/* Floating import queue badge (bottom-left) */}
      <div
        id="import-queue-panel"
        className="fixed bottom-6 left-6 z-55 bg-base-200 rounded-2xl shadow-xl border border-base-300 w-[320px]"
      >
        <div className="px-4 py-3 border-b border-base-300 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-base-content flex items-center gap-2">
            <span className="loading loading-spinner loading-xs text-primary" />
            Import Queue
            <span className="badge badge-primary badge-sm">{activeJobs.length}</span>
          </h3>
        </div>

        <div className="max-h-60 overflow-y-auto">
          {activeJobs.map((job) => (
            <ImportJobRow
              key={job.id}
              job={job}
              onReview={() => setReviewingJob(job)}
              onSkip={() => skipJob(job.id)}
            />
          ))}
        </div>
      </div>

      {/* NeedsReview modal (spawned per job) */}
      {reviewingJob && (
        <NeedsReviewModal
          job={reviewingJob}
          open={!!reviewingJob}
          onClose={() => setReviewingJob(null)}
          onConfirm={(gameId, category, objectId) => {
            confirmJob({ jobId: reviewingJob.id, gameId, category, objectId });
            setReviewingJob(null);
          }}
          onSkip={() => {
            skipJob(reviewingJob.id);
            setReviewingJob(null);
          }}
        />
      )}
    </>
  );
}

function ImportJobRow({
  job,
  onReview,
  onSkip,
}: {
  job: ImportJobItem;
  onReview: () => void;
  onSkip: () => void;
}) {
  const badge = STATUS_BADGE[job.status];
  const name = job.archive_path.split(/[/\\]/).pop() ?? job.archive_path;

  const isProcessing =
    job.status === 'queued' ||
    job.status === 'extracting' ||
    job.status === 'matching' ||
    job.status === 'placing';

  return (
    <div className="flex items-center gap-3 px-4 py-3 border-b border-base-300/50 last:border-0">
      {isProcessing && <span className="loading loading-spinner loading-xs text-info shrink-0" />}
      {!isProcessing && job.status !== 'needs_review' && <div className="w-4 shrink-0" />}
      {job.status === 'needs_review' && <span className="text-warning shrink-0">⚠</span>}

      <div className="flex-1 min-w-0">
        <p className="text-xs font-medium text-base-content truncate">{name}</p>
        <span className={`badge badge-xs ${badge.cls}`}>{badge.label}</span>
        {job.status === 'failed' && job.error_msg && (
          <p className="text-xs text-error truncate mt-0.5">{job.error_msg}</p>
        )}
      </div>

      <div className="flex gap-1 shrink-0">
        {job.status === 'needs_review' && (
          <button className="btn btn-warning btn-xs" onClick={onReview}>
            Review
          </button>
        )}
        {(job.status === 'needs_review' || job.status === 'failed') && (
          <button className="btn btn-ghost btn-xs" onClick={onSkip}>
            Skip
          </button>
        )}
      </div>
    </div>
  );
}
