// ──────────────────────────────────────────────────────────────────────────────
// Epic 44: Browser Feature — Shared TypeScript types
// ──────────────────────────────────────────────────────────────────────────────

import type { BrowserDownloadDto } from '@/shared/api/tauri/bindings.gen';

export type DownloadStatus =
  'requested' | 'in_progress' | 'finished' | 'failed' | 'canceled' | 'imported';

// ponytail: derived from codegen so a Rust schema change breaks the build here
// instead of drifting silently. Only `status` is narrowed — the DTO types it as
// the raw `String` the backend serializes.
export type BrowserDownloadItem = Omit<BrowserDownloadDto, 'status'> & { status: DownloadStatus };

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
  /** Present on queue creation when the backend can provide the persisted row. */
  download?: BrowserDownloadItem;
  filename?: string;
  file_path?: string | null;
}

/** A native WebView download awaits an explicit user decision before it is queued. */
export interface DownloadConfirmationRequest {
  id: string;
  filename: string;
  source_url: string;
  destination_path: string;
}
