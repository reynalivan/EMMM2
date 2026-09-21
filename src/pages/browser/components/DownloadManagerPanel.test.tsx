import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { DownloadManagerPanel } from './DownloadManagerPanel';
import * as useBrowserStoreModule from '@/entities/browser';
import * as useDownloadsModule from '../hooks/useDownloads';
import type { BrowserDownloadItem } from '../types';

vi.mock('@/entities/browser');
vi.mock('../hooks/useDownloads');
vi.mock('@/app/store', () => ({
  useAppStore: Object.assign(
    vi.fn((selector: (state: { activeGameId: string }) => unknown) =>
      selector({ activeGameId: 'game-1' }),
    ),
    { getState: () => ({ setWorkspaceView: vi.fn() }) },
  ),
}));

const mockDownloads: BrowserDownloadItem[] = [
  {
    id: 'dl-1',
    game_id: 'game-1',
    filename: 'mod_pack.zip',
    status: 'finished',
    bytes_total: 100,
    bytes_received: 100,
    session_id: null,
    file_path: 'path',
    source_url: 'url',
    error_msg: null,
    can_resume: false,
    tab_label: null,
    queue_order: 1,
    started_at: 'now',
    finished_at: 'now',
  },
  {
    id: 'dl-2',
    game_id: 'game-1',
    filename: 'downloading.rar',
    status: 'in_progress',
    bytes_total: 200,
    bytes_received: 100,
    session_id: null,
    file_path: 'path',
    source_url: 'url',
    error_msg: null,
    can_resume: false,
    tab_label: null,
    queue_order: 2,
    started_at: 'now',
    finished_at: null,
  },
];

describe('DownloadManagerPanel', () => {
  const closeDownloadPanel = vi.fn();
  const deleteDownload = vi.fn();
  const cancelDownload = vi.fn();
  const retryDownload = vi.fn();
  const refreshDownloads = vi.fn();
  const renameDownload = vi.fn().mockResolvedValue(undefined);
  const openDownloadFile = vi.fn();
  const openDownloadLocation = vi.fn();

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
      retryDownload,
      renameDownload,
      openDownloadFile,
      openDownloadLocation,
      refreshDownloads,
      isRefreshing: false,
    } as never);
  });

  it('renders downloads correctly', () => {
    render(<DownloadManagerPanel layout="docked" />);
    expect(screen.getByText('mod_pack.zip')).toBeInTheDocument();
    expect(screen.getByText('Ready')).toBeInTheDocument();
    expect(screen.getByText('downloading.rar')).toBeInTheDocument();
    expect(screen.getByText('Downloading')).toBeInTheDocument();
  });

  it('does not show any checkboxes', () => {
    render(<DownloadManagerPanel layout="docked" />);
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
      retryDownload,
      refreshDownloads,
      isRefreshing: false,
    } as never);

    render(<DownloadManagerPanel layout="docked" />);

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
      retryDownload,
      refreshDownloads,
      isRefreshing: false,
    } as never);

    render(<DownloadManagerPanel layout="docked" />);

    fireEvent.click(screen.getByRole('button', { name: 'Refresh downloads' }));
    expect(refreshDownloads).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole('button', { name: 'Download this file again' }));
    expect(retryDownload).toHaveBeenCalledWith('dl-failed');
  });

  it('renders as a non-modal docked region without viewport positioning', () => {
    render(<DownloadManagerPanel layout="docked" />);

    const panel = screen.getByRole('complementary', { name: 'Downloads' });
    expect(panel).not.toHaveClass('fixed');
    expect(panel).toHaveClass('w-[400px]');
  });

  it('keeps file actions behind the row menu and sends delete to the recycle-bin path', async () => {
    render(<DownloadManagerPanel layout="docked" />);

    fireEvent.click(screen.getAllByRole('button', { name: 'Download actions' })[0]);

    expect(screen.getByRole('menuitem', { name: 'Open file' })).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: 'Open in File Explorer' })).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: 'Rename file' })).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: 'Remove from list' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('menuitem', { name: 'Delete file' }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Delete file' }));
    expect(deleteDownload).toHaveBeenCalledWith({ id: 'dl-1', deleteFile: true });

    fireEvent.click(screen.getAllByRole('button', { name: 'Download actions' })[0]);
    fireEvent.click(screen.getByRole('menuitem', { name: 'Remove from list' }));
    expect(deleteDownload).toHaveBeenCalledWith({ id: 'dl-1', deleteFile: false });

    fireEvent.click(screen.getAllByRole('button', { name: 'Download actions' })[0]);
    fireEvent.click(screen.getByRole('menuitem', { name: 'Rename file' }));
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'renamed.zip' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save name' }));
    await waitFor(() => {
      expect(renameDownload).toHaveBeenCalledWith({ id: 'dl-1', filename: 'renamed.zip' });
    });
  });

  it('renders the row action menu in a viewport overlay so scroll containers cannot clip it', () => {
    render(<DownloadManagerPanel layout="docked" />);

    fireEvent.click(screen.getAllByRole('button', { name: 'Download actions' })[0]);

    const menu = screen.getByRole('menu', { name: 'Download actions' });
    expect(menu.parentElement).toBe(document.body);
    expect(menu).toHaveClass('fixed');
  });
});
