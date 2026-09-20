import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { WorkspaceParentEnableDialog } from './WorkspaceParentEnableDialogHost';

vi.mock('react-i18next', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-i18next')>();
  return {
    ...actual,
    useTranslation: () => ({
      t: (key: string, values?: Record<string, unknown>) => {
        const count = String(values?.count ?? '');
        const name = String(values?.name ?? '');
        const copy: Record<string, string> = {
          'parent_enable_dialog.title': 'Enable required folders',
          'parent_enable_dialog.target_blocked': `${name} needs its parent folders enabled first.`,
          'parent_enable_dialog.parents_required': `Folders to enable (${count})`,
          'parent_enable_dialog.will_activate': `Will become active (${count})`,
          'parent_enable_dialog.stay_disabled': `Will remain disabled (${count})`,
          'parent_enable_dialog.confirm': 'Enable folders and continue',
          'actions.cancel': 'Cancel',
          'actions.close': 'Close',
        };
        return copy[key] ?? key;
      },
    }),
  };
});

beforeEach(() => {
  HTMLDialogElement.prototype.showModal = vi.fn(function showModal(this: HTMLDialogElement) {
    this.setAttribute('open', '');
  });
  HTMLDialogElement.prototype.close = vi.fn(function close(this: HTMLDialogElement) {
    this.removeAttribute('open');
  });
});

describe('WorkspaceParentEnableDialog', () => {
  it('lists every required parent and its activation impact before confirming', () => {
    const onConfirm = vi.fn();
    render(
      <WorkspaceParentEnableDialog
        open
        isSubmitting={false}
        onConfirm={onConfirm}
        onClose={vi.fn()}
        requirement={{
          confirmation_token: 'confirm-1',
          requested_target: { path: 'Mods/DISABLED Group/DISABLED Alice/Blue', name: 'Blue' },
          parents: [
            { path: 'Mods/DISABLED Group', name: 'Group' },
            { path: 'Mods/DISABLED Group/DISABLED Alice', name: 'Alice' },
          ],
          will_activate: [
            { path: 'Mods/Group/Alice/Blue', name: 'Blue' },
            { path: 'Mods/Group/Other', name: 'Other' },
          ],
          stay_disabled: [{ path: 'Mods/Group/Alice/DISABLED Red', name: 'Red' }],
        }}
      />,
    );

    expect(screen.getByText('Folders to enable (2)')).toBeInTheDocument();
    expect(screen.getByText('Group')).toBeInTheDocument();
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.getByText('Will become active (2)')).toBeInTheDocument();
    expect(screen.getByText('Other')).toBeInTheDocument();
    expect(screen.getByText('Will remain disabled (1)')).toBeInTheDocument();
    expect(screen.getByText('Red')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Enable folders and continue' }));
    expect(onConfirm).toHaveBeenCalledOnce();
  });
});
