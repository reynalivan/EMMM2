import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { DownloadManagerPanel } from './DownloadManagerPanel';
import * as useBrowserStoreModule from '../../../stores/useBrowserStore';
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
  const onImportSelected = vi.fn();
  const toggleSelectDownload = vi.fn();
  const selectAll = vi.fn();
  const clearSelection = vi.fn();
  const closeDownloadPanel = vi.fn();
  const deleteDownload = vi.fn();
  const cancelDownload = vi.fn();
  const clearImported = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mocks
    vi.mocked(useBrowserStoreModule.useBrowserStore).mockReturnValue({
      isDownloadPanelOpen: true,
      selectedDownloadIds: new Set(),
      toggleSelectDownload,
      selectAll,
      clearSelection,
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
    render(<DownloadManagerPanel onImportSelected={onImportSelected} />);
    expect(screen.getByText('mod_pack.zip')).toBeInTheDocument();
    expect(screen.getByText('Ready')).toBeInTheDocument();
    expect(screen.getByText('downloading.rar')).toBeInTheDocument();
    expect(screen.getByText('Downloading')).toBeInTheDocument();
  });

  it('shows Bulk Toolbar only when downloads exist', () => {
    render(<DownloadManagerPanel onImportSelected={onImportSelected} />);
    expect(screen.getByText('Clear Imported')).toBeInTheDocument();
  });

  it('does not show checkboxes for non-finished items', () => {
    render(<DownloadManagerPanel onImportSelected={onImportSelected} />);
    const checkboxes = screen.getAllByRole('checkbox');
    // 1 for global bulk, 1 for the 'finished' item
    expect(checkboxes).toHaveLength(2);
  });

  it('calls onImportSelected when clicking Import on a single item', () => {
    render(<DownloadManagerPanel onImportSelected={onImportSelected} />);
    const importBtn = screen.getByText('Import');
    fireEvent.click(importBtn);
    expect(onImportSelected).toHaveBeenCalledWith(['dl-1'], '');
  });

  it('shows Import Selected when multiple items are selected', () => {
    vi.mocked(useBrowserStoreModule.useBrowserStore).mockReturnValue({
      isDownloadPanelOpen: true,
      selectedDownloadIds: new Set(['dl-1']),
      toggleSelectDownload,
      selectAll,
      clearSelection,
      closeDownloadPanel,
    });

    render(<DownloadManagerPanel onImportSelected={onImportSelected} />);
    const importSelectedBtn = screen.getByText('Import Selected');
    expect(importSelectedBtn).toBeInTheDocument();

    fireEvent.click(importSelectedBtn);
    expect(onImportSelected).toHaveBeenCalledWith(['dl-1'], '');
  });
});
