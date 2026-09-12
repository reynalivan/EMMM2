import { describe, expect, it } from 'vitest';
import { mergeDownloadProgress } from './useDownloads';
import type { BrowserDownloadItem } from '../types';

const terminalDownload: BrowserDownloadItem = {
  id: 'download-1',
  session_id: null,
  filename: 'mod.zip',
  file_path: 'C:/Downloads/mod.zip',
  source_url: 'https://example.com/mod.zip',
  status: 'finished',
  bytes_total: 100,
  bytes_received: 100,
  error_msg: null,
  queue_order: 1,
  started_at: '2026-09-11T00:00:00Z',
  finished_at: '2026-09-11T00:01:00Z',
};

describe('mergeDownloadProgress', () => {
  it('does not overwrite a terminal download with a late progress event', () => {
    expect(
      mergeDownloadProgress(terminalDownload, {
        id: terminalDownload.id,
        bytes_received: 25,
        bytes_total: 100,
      }),
    ).toEqual(terminalDownload);
  });
});
