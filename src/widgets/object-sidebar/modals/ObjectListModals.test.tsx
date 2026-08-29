import React from 'react';
import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import ObjectListModals from './ObjectListModals';

vi.mock('../../../shared/ui/components/ui/ConfirmDialog', () => ({
  default: ({ open }: { open: boolean }) =>
    open ? <div data-testid="confirm-dialog">Confirm Dialog</div> : null,
}));
vi.mock('./EditObjectModal', () => ({
  default: ({ open }: { open: boolean }) =>
    open ? <div data-testid="edit-modal">Edit Modal</div> : null,
}));
vi.mock('./CreateObjectModal', () => ({
  default: ({ open }: { open: boolean }) =>
    open ? <div data-testid="create-modal">Create Modal</div> : null,
}));
vi.mock('./AutoSetupModal', () => ({
  default: ({ open }: { open: boolean }) =>
    open ? <div data-testid="autosetup-modal">Auto Setup Modal</div> : null,
}));

describe('ObjectListModals', () => {
  it('renders nothing when not explicitly opened', () => {
    render(
      <ObjectListModals
        editObject={null}
        onCloseEdit={vi.fn()}
        createModalOpen={false}
        onCloseCreate={vi.fn()}
        autoSetupOpen={false}
        onCloseAutoSetup={vi.fn()}
        deleteObjectDialog={{ open: false, id: '', name: '' }}
        onConfirmDeleteObject={vi.fn()}
        onCancelDeleteObject={vi.fn()}
        forceDeleteObjectDialog={{ open: false, id: '', name: '', count: 0 }}
        onConfirmForceDeleteObject={vi.fn()}
        onCancelForceDeleteObject={vi.fn()}
      />,
    );
    expect(screen.queryByTestId('confirm-dialog')).toBeNull();
  });

  it('renders modals when opened', () => {
    render(
      <ObjectListModals
        editObject={
          { id: '1', name: 'Z' } as unknown as React.ComponentProps<
            typeof ObjectListModals
          >['editObject']
        }
        onCloseEdit={vi.fn()}
        createModalOpen={true}
        onCloseCreate={vi.fn()}
        autoSetupOpen={true}
        onCloseAutoSetup={vi.fn()}
        deleteObjectDialog={{ open: true, id: '1', name: 'Amber' }}
        onConfirmDeleteObject={vi.fn()}
        onCancelDeleteObject={vi.fn()}
        forceDeleteObjectDialog={{ open: false, id: '', name: '', count: 0 }}
        onConfirmForceDeleteObject={vi.fn()}
        onCancelForceDeleteObject={vi.fn()}
      />,
    );

    expect(screen.getByTestId('confirm-dialog')).toBeInTheDocument();
    expect(screen.getByTestId('edit-modal')).toBeInTheDocument();
    expect(screen.getByTestId('create-modal')).toBeInTheDocument();
    expect(screen.getByTestId('autosetup-modal')).toBeInTheDocument();
  });
});
