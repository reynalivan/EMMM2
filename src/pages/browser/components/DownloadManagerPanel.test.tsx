import { fireEvent, render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { DownloadManagerPanel } from './DownloadManagerPanel';
import * as useBrowserStoreModule from '@/entities/browser';
import * as useDownloadsModule from '../hooks/useDownloads';
import type { BrowserDownloadItem } from '../types';

vi.mock('@/entities/browser');
vi.mock('../hooks/useDownloads');
vi.mock('@/app/store', () => ({
  useAppStore: {
    getState: () => ({ setWorkspaceView: vi.fn() }),
  },
}));

const mockDownloads: BrowserDownloadItem[] = [
  {
    id: 'dl-1',
    filename: 'mod_pack.zip',
    status: 'finished',
    bytes_total: 100,
    bytes_received: 100,
    session_id: null,
    file_path: 'path',
    source_url: 'url',
    error_msg: null,
    queue_order: 1,
    started_at: 'now',
    finished_at: 'now',
  },
  {
    id: 'dl-2',
    filename: 'downloading.rar',
    status: 'in_progress',
    bytes_total: 200,
    bytes_received: 100,
    session_id: null,
    file_path: 'path',
    source_url: 'url',
    error_msg: null,
    queue_order: 2,
    started_at: 'now',
    finished_at: null,
  },
];

describe('DownloadManagerPanel', () => {
  const closeDownloadPanel = vi.fn();
  const deleteDownload = vi.fn();
  const cancelDownload = vi.fn();
  const clearImported = vi.fn();
  const retryDownload = vi.fn();
  const refreshDownloads = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mocks
    vi.mocked(useBrowserStoreModule.useBrowserStore).mockReturnValue({
      isDownloadPanelOpen: true,
      closeDownloadPanel,
    });

    vi.mocked(useDownloadsModule.useDownloads).mockReturnValue({
      downloads: mockDownloads,
      deleteDownload,
      cancelDownload,
      clearImported,
      retryDownload,
      refreshDownloads,
      isRefreshing: false,
    } as never);
  });

  it('renders downloads correctly', () => {
    render(<DownloadManagerPanel />);
    expect(screen.getByText('mod_pack.zip')).toBeInTheDocument();
    expect(screen.getByText('Ready')).toBeInTheDocument();
    expect(screen.getByText('downloading.rar')).toBeInTheDocument();
    expect(screen.getByText('Downloading')).toBeInTheDocument();
  });

  it('shows Toolbar with Clear Imported when downloads exist', () => {
    render(<DownloadManagerPanel />);
    expect(screen.getByText('Clear Imported')).toBeInTheDocument();
  });

  it('does not show any checkboxes', () => {
    render(<DownloadManagerPanel />);
    const checkboxes = screen.queryAllByRole('checkbox');
    expect(checkboxes).toHaveLength(0);
  });

  it('shows queue position and an indeterminate progress bar for downloads without a size', () => {
    const queued: BrowserDownloadItem = {
      ...mockDownloads[0],
      id: 'dl-queued',
      filename: 'queued.zip',
      status: 'requested',
      bytes_received: 0,
      bytes_total: null,
      started_at: '2026-08-31T01:00:00Z',
      finished_at: null,
    };
    const unknownSize: BrowserDownloadItem = {
      ...mockDownloads[1],
      id: 'dl-unknown-size',
      filename: 'streamed.rar',
      bytes_received: 50,
      bytes_total: null,
    };

    vi.mocked(useDownloadsModule.useDownloads).mockReturnValue({
      downloads: [unknownSize, queued],
      deleteDownload,
      cancelDownload,
      clearImported,
      retryDownload,
      refreshDownloads,
      isRefreshing: false,
    } as never);

    render(<DownloadManagerPanel />);

    expect(screen.getByText('Queue position: #1')).toBeInTheDocument();
    const progress = screen.getByRole('progressbar');
    expect(progress).not.toHaveAttribute('value');
  });

  it('refreshes the list and retries terminal failed downloads', () => {
    const failed: BrowserDownloadItem = {
      ...mockDownloads[0],
      id: 'dl-failed',
      filename: 'failed.zip',
      status: 'failed',
      error_msg: 'network error',
      finished_at: 'now',
    };

    vi.mocked(useDownloadsModule.useDownloads).mockReturnValue({
      downloads: [failed],
      deleteDownload,
      cancelDownload,
      clearImported,
      retryDownload,
      refreshDownloads,
      isRefreshing: false,
    } as never);

    render(<DownloadManagerPanel />);

    fireEvent.click(screen.getByRole('button', { name: 'Refresh downloads' }));
    expect(refreshDownloads).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole('button', { name: 'Download this file again' }));
    expect(retryDownload).toHaveBeenCalledWith('dl-failed');
  });
});
