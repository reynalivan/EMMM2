// ──────────────────────────────────────────────────────────────────────────────
// Epic 44: Browser Feature — Shared TypeScript types
// ──────────────────────────────────────────────────────────────────────────────

export type DownloadStatus =
  | 'requested'
  | 'in_progress'
  | 'finished'
  | 'failed'
  | 'canceled'
  | 'imported';

export interface BrowserDownloadItem {
  id: string;
  session_id: string | null;
  filename: string;
  file_path: string | null;
  source_url: string | null;
  status: DownloadStatus;
  bytes_total: number | null;
  bytes_received: number;
  error_msg: string | null;
  started_at: string;
  finished_at: string | null;
}

export type ImportJobStatus =
  | 'queued'
  | 'extracting'
  | 'matching'
  | 'needs_review'
  | 'placing'
  | 'done'
  | 'failed'
  | 'canceled';

export interface ImportJobItem {
  id: string;
  download_id: string | null;
  game_id: string | null;
  archive_path: string;
  status: ImportJobStatus;
  match_category: string | null;
  match_object_id: string | null;
  match_confidence: number | null;
  match_reason: string | null;
  placed_path: string | null;
  error_msg: string | null;
  is_duplicate: boolean;
  created_at: string;
  updated_at: string;
}

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
  object_id?: string | null;
  confidence?: number | null;
  reason?: string | null;
  placed_path?: string | null;
  error?: string | null;
}
