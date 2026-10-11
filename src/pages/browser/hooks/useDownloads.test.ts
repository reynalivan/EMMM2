import { describe, expect, it } from 'vitest';
import { mergeDownloadProgress } from './useDownloads';
import type { BrowserDownloadItem } from '../types';

const terminalDownload: BrowserDownloadItem = {
  id: 'download-1',
  game_id: 'game-1',
  session_id: null,
  filename: 'mod.zip',
  file_path: 'C:/Downloads/mod.zip',
  source_url: 'https://example.com/mod.zip',
  status: 'finished',
  bytes_total: 100,
  bytes_received: 100,
  error_msg: null,
  can_resume: false,
  tab_label: null,
  queue_order: 1,
  started_at: '2026-09-11T00:00:00Z',
  finished_at: '2026-09-11T00:01:00Z',
};

describe('mergeDownloadProgress', () => {
  it.each(['paused', 'requested'] as const)(
    'does not change %s into a running transfer',
    (status) => {
      const download = { ...terminalDownload, status, bytes_received: 10, can_resume: true };
      expect(
        mergeDownloadProgress(download, { id: download.id, bytes_received: 20, bytes_total: 100 }),
      ).toEqual(download);
    },
  );
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
