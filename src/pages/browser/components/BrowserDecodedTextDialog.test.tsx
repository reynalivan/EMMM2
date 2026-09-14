import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { BrowserDecodedTextDialog } from './BrowserDecodedTextDialog';

vi.mock('@/shared/lib/hooks/useDialogSync', () => ({
  useDialogSync: vi.fn(),
}));

describe('BrowserDecodedTextDialog', () => {
  it('shows decoded text and lets the user close it', () => {
    const onClose = vi.fn();

    render(
      <BrowserDecodedTextDialog
        decodedText={'line one\nline two'}
        onOpenLink={vi.fn()}
        onClose={onClose}
      />,
    );

    expect(screen.getByText('Decoded Base64 text')).toBeInTheDocument();
    expect(document.querySelector('pre')?.textContent).toBe('line one\nline two');

    fireEvent.click(screen.getAllByRole('button', { name: 'Close', hidden: true })[0]);

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('offers an open action only for decoded HTTP links', () => {
    const onOpenLink = vi.fn();

    render(
      <BrowserDecodedTextDialog
        decodedText="https://example.com"
        onOpenLink={onOpenLink}
        onClose={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Open link', hidden: true }));

    expect(onOpenLink).toHaveBeenCalledWith('https://example.com/');
  });
});
