import { act, render, screen, waitFor } from '@testing-library/react';
import { listen } from '@tauri-apps/api/event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DownloadConfirmationRequest } from '../types';
import { DownloadConfirmationHost } from './DownloadConfirmationHost';

type ConfirmationListener = (event: { payload: DownloadConfirmationRequest }) => void;
type AppStoreSlice = { setWorkspaceView: (view: string) => void };

const mocks = vi.hoisted(() => ({
  setWorkspaceView: vi.fn(),
  setDownloadConfirmationOpen: vi.fn(),
}));

vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: AppStoreSlice) => unknown) =>
    selector({ setWorkspaceView: mocks.setWorkspaceView }),
}));

vi.mock('@/app/store/useBrowserStore', () => ({
  useBrowserStore: {
    getState: () => ({ setDownloadConfirmationOpen: mocks.setDownloadConfirmationOpen }),
  },
}));

const confirmation = (id: string, filename: string): DownloadConfirmationRequest => ({
  id,
  filename,
  source_url: `https://example.test/${filename}`,
  destination_path: `C:/Downloads/${filename}`,
});

describe('DownloadConfirmationHost', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('keeps confirmations FIFO instead of replacing an earlier request', async () => {
    render(<DownloadConfirmationHost />);

    await waitFor(() => expect(listen).toHaveBeenCalled());
    const listener = vi
      .mocked(listen)
      .mock.calls.find(
        ([eventName]) => eventName === 'browser:download-confirmation-requested',
      )?.[1] as unknown as ConfirmationListener;

    act(() => {
      listener({ payload: confirmation('first', 'first.zip') });
      listener({ payload: confirmation('second', 'second.zip') });
    });

    expect(screen.getByText('first.zip')).toBeInTheDocument();
    expect(screen.queryByText('second.zip')).not.toBeInTheDocument();
  });
});
