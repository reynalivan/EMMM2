import { describe, expect, it } from 'vitest';
import { getDownloadFailureMessageKey, getQueuePosition } from './downloadPresentation';
import type { BrowserDownloadItem } from './types';

const queuedDownload = (id: string, queueOrder: number): BrowserDownloadItem => ({
  id,
  game_id: 'game-1',
  session_id: null,
  filename: `${id}.zip`,
  file_path: null,
  source_url: `https://example.test/${id}.zip`,
  status: 'requested',
  bytes_total: null,
  bytes_received: 0,
  error_msg: null,
  can_resume: false,
  tab_label: null,
  queue_order: queueOrder,
  started_at: '2026-08-31T00:00:00Z',
  finished_at: null,
});

describe('getQueuePosition', () => {
  it('uses persisted admission order when timestamps are identical', () => {
    const first = queuedDownload('first', 10);
    const second = queuedDownload('second', 11);

    expect(getQueuePosition([second, first], first.id)).toBe(1);
    expect(getQueuePosition([second, first], second.id)).toBe(2);
  });
});

describe('getDownloadFailureMessageKey', () => {
  it('maps actionable backend failure codes to localized copy', () => {
    expect(getDownloadFailureMessageKey('download.timeout')).toBe('downloads.failure.timeout');
    expect(getDownloadFailureMessageKey('download.offline')).toBe('downloads.failure.offline');
    expect(getDownloadFailureMessageKey('download.not_found')).toBe('downloads.failure.not_found');
  });

  it('keeps unknown and legacy errors safe to display', () => {
    expect(getDownloadFailureMessageKey('request timed out')).toBe('downloads.failure.timeout');
    expect(getDownloadFailureMessageKey('unexpected failure')).toBe('downloads.failure.unknown');
  });
});
