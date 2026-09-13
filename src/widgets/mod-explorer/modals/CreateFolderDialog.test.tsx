import { fireEvent, render, screen, waitFor } from '../../../tests/testing/test-utils';
import { describe, expect, it, vi } from 'vitest';
import CreateFolderDialog from './CreateFolderDialog';

describe('CreateFolderDialog', () => {
  it('uses the native dialog top layer and closes from Escape', () => {
    const onClose = vi.fn();

    render(
      <CreateFolderDialog
        open
        existingFolderNames={[]}
        isCreating={false}
        onClose={onClose}
        onSubmit={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    const dialog = screen.getByRole('dialog');
    expect(dialog.tagName).toBe('DIALOG');
    expect(dialog).toHaveProperty('open', true);

    fireEvent(dialog, new Event('cancel', { cancelable: true }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('requires a unique folder name before it submits', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);

    render(
      <CreateFolderDialog
        open
        existingFolderNames={['Variants']}
        isCreating={false}
        onClose={vi.fn()}
        onSubmit={onSubmit}
      />,
    );

    const input = screen.getByLabelText('Folder name');
    const submit = screen.getByRole('button', { name: 'Add folder' });

    expect(submit).toBeDisabled();

    fireEvent.change(input, { target: { value: 'variants' } });
    expect(screen.getByText('A folder with this name already exists here.')).toBeInTheDocument();
    expect(submit).toBeDisabled();

    fireEvent.change(input, { target: { value: 'Presets' } });
    fireEvent.click(submit);

    await waitFor(() => expect(onSubmit).toHaveBeenCalledWith('Presets'));
  });
});
