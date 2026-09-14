// ──────────────────────────────────────────────────────────────────────────────
// Epic 44: Browser Feature — Shared TypeScript types
// ──────────────────────────────────────────────────────────────────────────────

import type { BrowserDownloadDto } from '@/shared/api/tauri/bindings.gen';

export type DownloadStatus =
  'requested' | 'in_progress' | 'paused' | 'finished' | 'failed' | 'canceled' | 'imported';

// ponytail: derived from codegen so a Rust schema change breaks the build here
// instead of drifting silently. Only `status` is narrowed — the DTO types it as
// the raw `String` the backend serializes.
export type BrowserDownloadItem = Omit<BrowserDownloadDto, 'status'> & { status: DownloadStatus };

// Runtime download progress event
export interface DownloadProgressEvent {
  id: string;
  bytes_received: number;
  bytes_total: number | null;
  eta?: string | null;
}

// Runtime download status event
export interface DownloadStatusEvent {
  id: string;
  status: DownloadStatus;
  /** Present on queue creation when the backend can provide the persisted row. */
  download?: BrowserDownloadItem;
  filename?: string;
  file_path?: string | null;
  error_msg?: string | null;
  can_resume?: boolean;
}

/** A native WebView download awaits an explicit user decision before it is queued. */
export interface DownloadConfirmationRequest {
  id: string;
  filename: string;
  source_url: string;
  destination_path: string;
  mime_type?: string | null;
  content_disposition?: string | null;
  bytes_total?: number | null;
  risk_level?: 'blocked' | 'warning' | null;
}

/** A native browser download is resolving its safe display information. */
export interface DownloadInformationLoading {
  id: string;
  source_url: string;
}

/** A preparation failure occurs before a download is queued or persisted. */
export interface DownloadInformationFailure extends DownloadInformationLoading {
  reason: 'queue_full' | 'timeout' | 'unavailable';
}
