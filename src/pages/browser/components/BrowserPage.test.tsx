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
});
