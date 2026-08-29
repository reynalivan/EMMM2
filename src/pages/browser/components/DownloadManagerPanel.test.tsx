import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { DownloadManagerPanel } from './DownloadManagerPanel';
import * as useBrowserStoreModule from '@/app/store/useBrowserStore';
import * as useDownloadsModule from '../hooks/useDownloads';
import type { BrowserDownloadItem } from '../types';

vi.mock('../../../stores/useBrowserStore');
vi.mock('../hooks/useDownloads');

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
    started_at: 'now',
    finished_at: null,
  },
];

describe('DownloadManagerPanel', () => {
  const closeDownloadPanel = vi.fn();
  const deleteDownload = vi.fn();
  const cancelDownload = vi.fn();
  const clearImported = vi.fn();

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
});
