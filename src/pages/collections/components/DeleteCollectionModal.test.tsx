import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '../../../tests/testing/test-utils';
import { DeleteCollectionModal } from './DeleteCollectionModal';

describe('DeleteCollectionModal', () => {
  it('dismisses an idle native cancel request without deleting', () => {
    const onCancel = vi.fn();
    const onConfirm = vi.fn();
    render(
      <DeleteCollectionModal
        isOpen
        collectionName="Preset"
        isDeleting={false}
        onCancel={onCancel}
        onConfirm={onConfirm}
      />,
    );
    expect(HTMLDialogElement.prototype.showModal).toHaveBeenCalled();
    fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true }));
    expect(onCancel).toHaveBeenCalledOnce();
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('prevents native cancellation during deletion', () => {
    const onCancel = vi.fn();
    render(
      <DeleteCollectionModal
        isOpen
        collectionName="Preset"
        isDeleting
        onCancel={onCancel}
        onConfirm={vi.fn()}
      />,
    );
    const dialog = screen.getByRole('dialog');
    const cancel = new Event('cancel', { cancelable: true });
    fireEvent(dialog, cancel);
    expect(cancel.defaultPrevented).toBe(true);
    fireEvent(dialog, new Event('close'));
    expect(onCancel).not.toHaveBeenCalled();
  });

  it('reopens after its controlled visibility changes', () => {
    const props = {
      collectionName: 'Preset',
      isDeleting: false,
      onCancel: vi.fn(),
      onConfirm: vi.fn(),
    };
    const { rerender } = render(<DeleteCollectionModal {...props} isOpen={false} />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    rerender(<DeleteCollectionModal {...props} isOpen />);
    expect(screen.getByRole('dialog')).toHaveAttribute('open');
    rerender(<DeleteCollectionModal {...props} isOpen={false} />);
    rerender(<DeleteCollectionModal {...props} isOpen />);
    expect(screen.getByRole('dialog')).toHaveAttribute('open');
  });
});
