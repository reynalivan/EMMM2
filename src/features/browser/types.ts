// ──────────────────────────────────────────────────────────────────────────────
// Epic 44: Browser Feature — Shared TypeScript types
// ──────────────────────────────────────────────────────────────────────────────

import type { BrowserDownloadDto, ImportJobDto } from '../../lib/bindings.gen';

export type DownloadStatus =
  | 'requested'
  | 'in_progress'
  | 'finished'
  | 'failed'
  | 'canceled'
  | 'imported';

// ponytail: derived from codegen so a Rust schema change breaks the build here
// instead of drifting silently. Only `status` is narrowed — the DTO types it as
// the raw `String` the backend serializes.
export type BrowserDownloadItem = Omit<BrowserDownloadDto, 'status'> & { status: DownloadStatus };

export type ImportJobStatus =
  | 'queued'
  | 'extracting'
  | 'matching'
  | 'needs_review'
  | 'placing'
  | 'done'
  | 'failed'
  | 'canceled';

export type ImportJobItem = Omit<ImportJobDto, 'status'> & { status: ImportJobStatus };

// Runtime download progress event
export interface DownloadProgressEvent {
  id: string;
  bytes_received: number;
  bytes_total: number | null;
}

// Runtime download status event
export interface DownloadStatusEvent {
  id: string;
  status: DownloadStatus;
  file_path?: string | null;
}

// Runtime import job update event
export interface ImportJobUpdateEvent {
  job_id: string;
  status: ImportJobStatus;
  category?: string | null;
  entry_key?: string | null;
  alias_name?: string | null;
  confidence?: number | null;
  reason?: string | null;
  placed_path?: string | null;
  error?: string | null;
}
