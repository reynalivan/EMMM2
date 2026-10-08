import { fireEvent, render, screen } from '@testing-library/react';
import { useState } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TrustInformationDialog, type TrustDocument } from './TrustInformationDialog';

vi.mock('react-i18next', async (importOriginal) => ({
  ...(await importOriginal<typeof import('react-i18next')>()),
  useTranslation: () => ({ t: (key: string) => key }),
}));

function StatefulDialog() {
  const [document, setDocument] = useState<TrustDocument | null>(null);
  return (
    <>
      <button onClick={() => setDocument('privacy')}>Privacy</button>
      <TrustInformationDialog document={document} onClose={() => setDocument(null)} />
    </>
  );
}

describe('TrustInformationDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    let opener: Element | null = null;
    vi.spyOn(HTMLDialogElement.prototype, 'showModal').mockImplementation(function (
      this: HTMLDialogElement,
    ) {
      opener = document.activeElement;
      this.open = true;
      this.querySelector('button')?.focus();
    });
    vi.spyOn(HTMLDialogElement.prototype, 'close').mockImplementation(function (
      this: HTMLDialogElement,
    ) {
      this.open = false;
      if (opener instanceof HTMLElement) opener.focus();
    });
  });

  it('opens a native modal when selected from an initially empty dialog', () => {
    render(<StatefulDialog />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Privacy' }));

    expect(HTMLDialogElement.prototype.showModal).toHaveBeenCalledOnce();
    expect(screen.getByRole('dialog')).toHaveAttribute('open');
    expect(
      screen.getByRole('heading', { name: 'general.trust.privacy.title' }),
    ).toBeInTheDocument();
  });

  it.each(['escape', 'close button', 'backdrop'])(
    'closes via %s and returns focus to the opener',
    (action) => {
      render(<StatefulDialog />);
      const opener = screen.getByRole('button', { name: 'Privacy' });
      opener.focus();
      fireEvent.click(opener);
      const dialog = screen.getByRole('dialog');

      if (action === 'escape') fireEvent(dialog, new Event('cancel', { cancelable: true }));
      else
        fireEvent.click(
          screen.getByRole('button', {
            name: action === 'close button' ? 'general.trust.close_dialog' : 'general.trust.close',
          }),
        );

      expect(HTMLDialogElement.prototype.close).toHaveBeenCalledOnce();
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
      expect(opener).toHaveFocus();
    },
  );
});
