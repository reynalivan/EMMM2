import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { DownloadConfirmationDialog } from './DownloadConfirmationDialog';

describe('DownloadConfirmationDialog', () => {
  it('focuses the download action and rejects the request when Escape is pressed', async () => {
    const onConfirm = vi.fn();
    const onReject = vi.fn();

    render(
      <DownloadConfirmationDialog
        request={{
          id: 'request-1',
          filename: 'mod.zip',
          source_url: 'https://example.com/mod.zip',
          destination_path: 'C:/Downloads/mod.zip',
        }}
        isSubmitting={false}
        onConfirm={onConfirm}
        onReject={onReject}
      />,
    );

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('mod.zip')).toBeInTheDocument();

    const confirmButton = screen.getByRole('button', { name: 'Download' });
    await waitFor(() => expect(confirmButton).toHaveFocus());

    fireEvent.keyDown(confirmButton, { key: 'Escape', code: 'Escape' });

    expect(onReject).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('does not offer a download action for blocked executable files', async () => {
    const onConfirm = vi.fn();
    const onReject = vi.fn();

    render(
      <DownloadConfirmationDialog
        request={{
          id: 'request-2',
          filename: 'mod-installer.exe',
          source_url: 'https://example.com/mod-installer.exe',
          destination_path: 'C:/Downloads/mod-installer.exe',
          risk_level: 'blocked',
        }}
        isSubmitting={false}
        onConfirm={onConfirm}
        onReject={onReject}
      />,
    );

    expect(
      screen.getByText('Executable and script downloads are blocked to protect your mod library.'),
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Download' })).not.toBeInTheDocument();

    const cancelButton = screen.getByRole('button', { name: 'Cancel' });
    await waitFor(() => expect(cancelButton).toHaveFocus());
    fireEvent.click(cancelButton);

    expect(onReject).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });
});
