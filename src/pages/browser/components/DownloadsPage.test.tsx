import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import DownloadsPage from './DownloadsPage';
import type { BrowserDownloadItem } from '../types';

const setWorkspaceView = vi.fn();
const pauseDownload = vi.fn();
const resumeDownload = vi.fn();
const cancelDownload = vi.fn();
let downloads: BrowserDownloadItem[] = [];

vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: { setWorkspaceView: typeof setWorkspaceView }) => unknown) =>
    selector({ setWorkspaceView }),
}));

vi.mock('../hooks/useDownloads', () => ({
  useDownloads: () => ({
    downloads,
    deleteDownload: vi.fn(),
    cancelDownload,
    pauseDownload,
    resumeDownload,
    retryDownload: vi.fn(),
    refreshDownloads: vi.fn(),
    isRefreshing: false,
  }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        'downloads.open_mod_inbox': 'Open Mod Inbox',
        'downloads.refresh': 'Refresh downloads',
        'downloads.empty': 'No downloads yet',
        'downloads.description': 'Track downloads and send files to Mod Inbox.',
      })[key] ?? key,
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

describe('DownloadsPage', () => {
  beforeEach(() => {
    setWorkspaceView.mockReset();
    vi.clearAllMocks();
    downloads = [];
    document.body.innerHTML = '<div id="topbar-actions-portal"></div>';
  });

  it('opens Mod Inbox from the download toolbar', () => {
    render(<DownloadsPage />);

    fireEvent.click(screen.getByRole('button', { name: 'Open Mod Inbox' }));

    expect(setWorkspaceView).toHaveBeenCalledWith('mod-inbox');
  });

  it('shows the page context in the shared frame', () => {
    render(<DownloadsPage />);

    expect(screen.getByText('Track downloads and send files to Mod Inbox.')).toBeInTheDocument();
  });

  it.each(['in_progress', 'paused', 'failed'] as const)(
    'provides native transfer controls for one %s item',
    (status) => {
      downloads = [
        {
          id: 'native-1',
          game_id: 'game-1',
          session_id: null,
          filename: 'pack.zip',
          file_path: 'C:/Downloads/pack.zip',
          source_url: 'https://example.com/pack.zip',
          status,
          bytes_total: 100,
          bytes_received: 10,
          error_msg: null,
          can_resume: true,
          tab_label: 'browser-tab-1',
          queue_order: 1,
          started_at: '2026-10-09',
          finished_at: null,
        },
      ];
      render(<DownloadsPage />);
      if (status === 'in_progress') {
        fireEvent.click(screen.getByRole('button', { name: 'downloads.pause_title' }));
        expect(pauseDownload).toHaveBeenCalledWith('native-1');
      } else {
        fireEvent.click(screen.getByRole('button', { name: 'downloads.resume_title' }));
        expect(resumeDownload).toHaveBeenCalledWith('native-1');
      }
      if (status === 'paused' || status === 'failed') {
        fireEvent.click(screen.getByRole('button', { name: 'downloads.cancel_title' }));
        expect(cancelDownload).toHaveBeenCalledWith('native-1');
      }
      expect(screen.queryByRole('button', { name: 'downloads.actions_menu' })).toBeNull();
    },
  );
});
