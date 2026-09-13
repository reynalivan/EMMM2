import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import DownloadsPage from './DownloadsPage';

const setWorkspaceView = vi.fn();

vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: { setWorkspaceView: typeof setWorkspaceView }) => unknown) =>
    selector({ setWorkspaceView }),
}));

vi.mock('../hooks/useDownloads', () => ({
  useDownloads: () => ({
    downloads: [],
    deleteDownload: vi.fn(),
    cancelDownload: vi.fn(),
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
});
