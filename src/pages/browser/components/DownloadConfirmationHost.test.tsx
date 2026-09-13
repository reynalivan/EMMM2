import { act, render, screen, waitFor } from '@testing-library/react';
import { listen } from '@tauri-apps/api/event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  DownloadConfirmationRequest,
  DownloadInformationFailure,
  DownloadInformationLoading,
} from '../types';
import { DownloadConfirmationHost } from './DownloadConfirmationHost';

type ConfirmationListener = (event: { payload: DownloadConfirmationRequest }) => void;
type LoadingListener = (event: { payload: DownloadInformationLoading }) => void;
type FailureListener = (event: { payload: DownloadInformationFailure }) => void;
type AppStoreSlice = { setWorkspaceView: (view: string) => void };

const mocks = vi.hoisted(() => ({
  setWorkspaceView: vi.fn(),
  setDownloadConfirmationOpen: vi.fn(),
}));

vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: AppStoreSlice) => unknown) =>
    selector({ setWorkspaceView: mocks.setWorkspaceView }),
}));

vi.mock('@/entities/browser', () => ({
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

  it('shows preparation feedback until the matching confirmation arrives', async () => {
    render(<DownloadConfirmationHost />);

    await waitFor(() => expect(listen).toHaveBeenCalledTimes(3));
    const loadingListener = vi
      .mocked(listen)
      .mock.calls.find(
        ([eventName]) => eventName === 'browser:download-information-loading',
      )?.[1] as unknown as LoadingListener;
    const confirmationListener = vi
      .mocked(listen)
      .mock.calls.find(
        ([eventName]) => eventName === 'browser:download-confirmation-requested',
      )?.[1] as unknown as ConfirmationListener;

    act(() => {
      loadingListener({
        payload: { id: 'preparing', source_url: 'https://example.test/file.zip' },
      });
    });

    expect(screen.getByText('Getting file details')).toBeInTheDocument();
    expect(mocks.setDownloadConfirmationOpen).toHaveBeenLastCalledWith(true);

    act(() => {
      confirmationListener({ payload: confirmation('preparing', 'file.zip') });
    });

    expect(screen.getByText('file.zip')).toBeInTheDocument();
    expect(screen.queryByText('Getting file details')).not.toBeInTheDocument();
  });

  it('replaces a failed preparation spinner with an actionable error', async () => {
    render(<DownloadConfirmationHost />);

    await waitFor(() => expect(listen).toHaveBeenCalledTimes(3));
    const loadingListener = vi
      .mocked(listen)
      .mock.calls.find(
        ([eventName]) => eventName === 'browser:download-information-loading',
      )?.[1] as unknown as LoadingListener;
    const failureListener = vi
      .mocked(listen)
      .mock.calls.find(
        ([eventName]) => eventName === 'browser:download-information-failed',
      )?.[1] as unknown as FailureListener;

    act(() => {
      loadingListener({
        payload: { id: 'unavailable', source_url: 'https://example.test/file.zip' },
      });
      failureListener({
        payload: {
          id: 'unavailable',
          source_url: 'https://example.test/file.zip',
          reason: 'unavailable',
        },
      });
    });

    expect(screen.getByText("Couldn't prepare the download")).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Close' })).toHaveFocus();
  });
});
